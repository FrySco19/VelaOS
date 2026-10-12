/*
 * VelaOS x86_64 kernel (v0.1 "first light").
 * Boots via GRUB Multiboot2, takes a linear framebuffer and draws the Vela desktop:
 * top bar with clock, dock, movable windows, a PS/2 mouse pointer and an interactive shell.
 * No interrupts yet: keyboard, mouse and RTC are polled.
 */
#include <stdint.h>
#include <stddef.h>
#include "font.h"

#define VELA_VERSION "0.3.0-glass-dev"
#include "wallpaper_size.h"
extern const uint16_t vela_wallpaper[];

/* ---------------------------------------------------------------- libc bits */
void *memset(void *d, int c, size_t n) { uint8_t *p = d; while (n--) *p++ = (uint8_t)c; return d; }
void *memcpy(void *d, const void *s, size_t n) {
    void *r = d;
    __asm__ volatile("rep movsb" : "+D"(d), "+S"(s), "+c"(n) :: "memory");
    return r;
}
void *memmove(void *d, const void *s, size_t n) {
    uint8_t *dp = d; const uint8_t *sp = s;
    if (dp < sp) while (n--) *dp++ = *sp++;
    else { dp += n; sp += n; while (n--) *--dp = *--sp; }
    return d;
}
static size_t kstrlen(const char *s) { size_t n = 0; while (s[n]) n++; return n; }
static int kstreq(const char *a, const char *b) { while (*a && *a == *b) a++, b++; return *a == *b; }
static int kstartswith(const char *s, const char *p) { while (*p) if (*s++ != *p++) return 0; return 1; }
static void kstrcpy(char *d, const char *s, size_t cap) { size_t i = 0; while (s[i] && i + 1 < cap) { d[i] = s[i]; i++; } d[i] = 0; }
static void kstrcat(char *d, const char *s, size_t cap) { size_t n = kstrlen(d); kstrcpy(d + n, s, cap - n); }
static void utoa(uint64_t v, char *out) {
    char t[24]; int n = 0;
    do { t[n++] = '0' + v % 10; v /= 10; } while (v);
    for (int i = 0; i < n; i++) out[i] = t[n - 1 - i];
    out[n] = 0;
}
static void utoa2(unsigned v, char *out) { out[0] = '0' + (v / 10) % 10; out[1] = '0' + v % 10; out[2] = 0; }

/* ---------------------------------------------------------------- port io */
static inline void outb(uint16_t p, uint8_t v) { __asm__ volatile("outb %0,%1" :: "a"(v), "Nd"(p)); }
static inline void outw(uint16_t p, uint16_t v) { __asm__ volatile("outw %0,%1" :: "a"(v), "Nd"(p)); }
static inline uint16_t inw(uint16_t p) { uint16_t v; __asm__ volatile("inw %1,%0" : "=a"(v) : "Nd"(p)); return v; }
static inline uint8_t inb(uint16_t p) { uint8_t v; __asm__ volatile("inb %1,%0" : "=a"(v) : "Nd"(p)); return v; }
static inline uint64_t rdtsc(void) { uint32_t lo, hi; __asm__ volatile("rdtsc" : "=a"(lo), "=d"(hi)); return ((uint64_t)hi << 32) | lo; }
static void cpuid(uint32_t leaf, uint32_t *a, uint32_t *b, uint32_t *c, uint32_t *d) {
    __asm__ volatile("cpuid" : "=a"(*a), "=b"(*b), "=c"(*c), "=d"(*d) : "a"(leaf), "c"(0));
}

/* serial COM1 log, handy for headless QEMU runs */
static void serial_init(void) {
    outb(0x3F9, 0); outb(0x3FB, 0x80); outb(0x3F8, 1); outb(0x3F9, 0);
    outb(0x3FB, 0x03); outb(0x3FA, 0xC7); outb(0x3FC, 0x0B);
}
static void serial_puts(const char *s) {
    for (; *s; s++) { for (int i = 0; i < 10000 && !(inb(0x3FD) & 0x20); i++) {} outb(0x3F8, (uint8_t)*s); }
}

/* ---------------------------------------------------------------- framebuffer */
#define MAXW 1920
#define MAXH 1200
static uint8_t *fb; static uint32_t fb_pitch, fb_bpp;
static int W, H;
static uint32_t backbuf[MAXW * MAXH];
static uint32_t wallpaper[MAXW * MAXH];

static inline uint32_t blend(uint32_t dst, uint32_t src, unsigned a) {
    unsigned ia = 255 - a;
    uint32_t r = (((src >> 16) & 255) * a + ((dst >> 16) & 255) * ia) / 255;
    uint32_t g = (((src >> 8) & 255) * a + ((dst >> 8) & 255) * ia) / 255;
    uint32_t b = ((src & 255) * a + (dst & 255) * ia) / 255;
    return (r << 16) | (g << 8) | b;
}
static inline void pset(int x, int y, uint32_t c, unsigned a) {
    if ((unsigned)x >= (unsigned)W || (unsigned)y >= (unsigned)H) return;
    uint32_t *p = &backbuf[y * W + x];
    *p = a >= 255 ? c : blend(*p, c, a);
}
static void fill_rect(int x, int y, int w, int h, uint32_t c, unsigned a) {
    int x0 = x < 0 ? 0 : x, y0 = y < 0 ? 0 : y;
    int x1 = x + w > W ? W : x + w, y1 = y + h > H ? H : y + h;
    for (int j = y0; j < y1; j++) {
        uint32_t *row = &backbuf[j * W];
        if (a >= 255) for (int i = x0; i < x1; i++) row[i] = c;
        else for (int i = x0; i < x1; i++) row[i] = blend(row[i], c, a);
    }
}
static int isqrt(int v) { int r = 0; while ((r + 1) * (r + 1) <= v) r++; return r; }
/* rounded rectangle with soft (1px) corner edges */
static void fill_rrect(int x, int y, int w, int h, int r, uint32_t c, unsigned a) {
    if (r * 2 > h) r = h / 2;
    if (r * 2 > w) r = w / 2;
    for (int j = 0; j < h; j++) {
        int inset = 0;
        int dy = j < r ? r - j : (j >= h - r ? j - (h - r - 1) : 0);
        if (dy) inset = r - isqrt(r * r - (dy - 1) * (dy - 1) > 0 ? r * r - dy * dy + dy : 0);
        if (inset < 0) inset = 0;
        int ww = w - 2 * inset;
        if (ww <= 0) continue;
        fill_rect(x + inset, y + j, ww, 1, c, a);
        if (dy && inset > 0) { pset(x + inset - 1, y + j, c, a / 3); pset(x + w - inset, y + j, c, a / 3); }
    }
}
static void fill_circle(int cx, int cy, int r, uint32_t c, unsigned a) {
    for (int j = -r; j <= r; j++) for (int i = -r; i <= r; i++) {
        int d = i * i + j * j;
        if (d <= r * r - r) pset(cx + i, cy + j, c, a);
        else if (d <= r * r + r) pset(cx + i, cy + j, c, a / 2);
    }
}
static void blit(void) {
    for (int y = 0; y < H; y++) {
        uint8_t *dst = fb + (size_t)y * fb_pitch;
        uint32_t *src = &backbuf[y * W];
        if (fb_bpp == 32) memcpy(dst, src, (size_t)W * 4);
        else for (int x = 0; x < W; x++) { dst[x*3] = src[x] & 255; dst[x*3+1] = (src[x] >> 8) & 255; dst[x*3+2] = (src[x] >> 16) & 255; }
    }
}

/* ---------------------------------------------------------------- text */
static int draw_char(const font_t *f, int x, int y, char ch, uint32_t c) {
    if (ch < 32 || ch > 126) ch = '?';
    const unsigned char *g = f->px + (size_t)(ch - 32) * f->w * f->h;
    for (int j = 0; j < f->h; j++) for (int i = 0; i < f->w; i++) { unsigned a = g[j * f->w + i]; if (a) pset(x + i, y + j, c, a); }
    return f->adv[ch - 32];
}
static int draw_text(const font_t *f, int x, int y, const char *s, uint32_t c) {
    int x0 = x;
    for (; *s; s++) x += draw_char(f, x, y, *s, c);
    return x - x0;
}
static int text_width(const font_t *f, const char *s) { int w = 0; for (; *s; s++) w += f->adv[(*s < 32 || *s > 126 ? '?' : *s) - 32]; return w; }

/* ---------------------------------------------------------------- system info */
static uint64_t mem_kb;
static char boot_loader[48] = "unknown";
static char cmdline[64];
static char cpu_vendor[13], cpu_brand[49];
static uint64_t tsc_boot;

struct mb2_tag { uint32_t type, size; };
static void parse_multiboot(uint64_t info) {
    uint8_t *p = (uint8_t *)info + 8, *end = (uint8_t *)info + *(uint32_t *)info;
    while (p < end) {
        struct mb2_tag *t = (struct mb2_tag *)p;
        if (t->type == 0) break;
        if (t->type == 1) kstrcpy(cmdline, (char *)(p + 8), sizeof cmdline);
        if (t->type == 2) kstrcpy(boot_loader, (char *)(p + 8), sizeof boot_loader);
        if (t->type == 4) mem_kb = (uint64_t)*(uint32_t *)(p + 8) + *(uint32_t *)(p + 12);
        if (t->type == 8) {
            fb = (uint8_t *)(uintptr_t)*(uint64_t *)(p + 8);
            fb_pitch = *(uint32_t *)(p + 16);
            W = (int)*(uint32_t *)(p + 20);
            H = (int)*(uint32_t *)(p + 24);
            fb_bpp = *(uint8_t *)(p + 28);
        }
        p += (t->size + 7) & ~7u;
    }
}
static void read_cpu(void) {
    uint32_t a, b, c, d;
    cpuid(0, &a, &b, &c, &d);
    memcpy(cpu_vendor, &b, 4); memcpy(cpu_vendor + 4, &d, 4); memcpy(cpu_vendor + 8, &c, 4); cpu_vendor[12] = 0;
    cpuid(0x80000000, &a, &b, &c, &d);
    if (a >= 0x80000004) {
        uint32_t *o = (uint32_t *)cpu_brand;
        for (uint32_t l = 0; l < 3; l++) cpuid(0x80000002 + l, &o[l*4], &o[l*4+1], &o[l*4+2], &o[l*4+3]);
        cpu_brand[48] = 0;
        char *s = cpu_brand; while (*s == ' ') s++;
        memmove(cpu_brand, s, kstrlen(s) + 1);
    } else kstrcpy(cpu_brand, cpu_vendor, sizeof cpu_brand);
}

/* RTC */
static uint8_t cmos(uint8_t r) { outb(0x70, r); return inb(0x71); }
struct rtc { unsigned sec, min, hour, day, mon, year; };
static void rtc_read(struct rtc *t) {
    while (cmos(0x0A) & 0x80) {}
    t->sec = cmos(0); t->min = cmos(2); t->hour = cmos(4); t->day = cmos(7); t->mon = cmos(8); t->year = cmos(9);
    if (!(cmos(0x0B) & 0x04)) {
#define BCD(v) (((v) & 15) + ((v) >> 4) * 10)
        t->sec = BCD(t->sec); t->min = BCD(t->min); t->hour = BCD(t->hour & 0x7F) | (t->hour & 0x80);
        t->day = BCD(t->day); t->mon = BCD(t->mon); t->year = BCD(t->year);
    }
    t->hour &= 0x7F;
    t->year += 2000;
}
static unsigned boot_secs, uptime_base_set;
static unsigned rtc_secs_of_day(struct rtc *t) { return t->hour * 3600 + t->min * 60 + t->sec; }

/* ================================================================ shell state */
#define T(en, it) (lang ? (it) : (en))
static int lang = 1;                 /* 0 = English (default), 1 = Italiano */
static int theme_id = 2;             /* 0 Light, 1 Dawn, 2 Deep */
static int larger_text, reduce_motion;
static int welcome_open = 1;
static int cur_space;
#define NSPACES 4
static int focus_mode;           /* Focus mode: bar + dock hidden */
static unsigned hint_until;      /* RTC second-of-day until which the focus hint shows */

typedef struct {
    const char *name;
    uint32_t win, side, divider, text, muted, ink, glass, accent, accent_soft;
    unsigned glass_a;
} theme_t;
static const theme_t themes[3] = {
    { "Light", 0xF9F7F4, 0xF1EEEA, 0xE6E1DB, 0x14233A, 0x6B7684, 0x7A8591, 0x0E223B, 0x2D6877, 0xD9E3E0, 150 },
    { "Dawn",  0xFBF4EC, 0xF4E7DA, 0xE9D8C6, 0x3A2A22, 0x8A7363, 0x9A8170, 0x3A2433, 0x2D6877, 0xEBDCCB, 120 },
    { "Deep",  0x0F181F, 0x15212A, 0x22313C, 0xE6EEF2, 0x8FA3B0, 0x8FA3B0, 0x08131B, 0x34919B, 0x1E3640, 175 },
};
#define TH (&themes[theme_id])
#define F_UI (larger_text ? &font_ui_lg : &font_ui)
#define F_BOLD (larger_text ? &font_bold_lg : &font_bold)
#define TEAL_BRIGHT 0x34919B

static uint32_t blurred[MAXW * MAXH];

/* rounded-rect row inset */
static int rr_inset(int j, int h, int r) {
    int dy = j < r ? r - j : (j >= h - r ? j - (h - r - 1) : 0);
    if (!dy) return 0;
    int v = r * r - (dy - 1) * (dy - 1);
    int in = r - isqrt(v > 0 ? v : 0);
    return in < 0 ? 0 : in;
}
/* frosted glass: blurred wallpaper + tint, rounded */
static void glass_rrect(int x, int y, int w, int h, int r, uint32_t tint, unsigned a) {
    for (int j = 0; j < h; j++) {
        int yy = y + j; if ((unsigned)yy >= (unsigned)H) continue;
        int in = rr_inset(j, h, r);
        for (int i = x + in; i < x + w - in; i++) {
            if ((unsigned)i >= (unsigned)W) continue;
            backbuf[yy * W + i] = blend(blurred[yy * W + i], tint, a);
        }
    }
}
static void stroke_rrect(int x, int y, int w, int h, int r, uint32_t c, unsigned a) {
    for (int j = 0; j < h; j++) {
        int in = rr_inset(j, h, r);
        if (j == 0 || j == h - 1) fill_rect(x + in, y + j, w - 2 * in, 1, c, a);
        else { pset(x + in, y + j, c, a); pset(x + w - 1 - in, y + j, c, a); }
    }
}
static void soft_shadow(int x, int y, int w, int h, int r, int strong) {
    unsigned a = strong ? 16 : 10;
    for (int s = 16; s > 0; s -= 4) fill_rrect(x - s + 2, y - s + 10, w + 2 * s - 4, h + 2 * s - 4, r + s, 0x000000, a);
}
/* thick anti-aliased-ish line using small discs */
static void line(int x0, int y0, int x1, int y1, uint32_t c, unsigned a) {
    int dx = x1 - x0, dy = y1 - y0;
    int n = (dx < 0 ? -dx : dx) > (dy < 0 ? -dy : dy) ? (dx < 0 ? -dx : dx) : (dy < 0 ? -dy : dy);
    if (!n) n = 1;
    for (int i = 0; i <= n; i++) {
        int x = x0 + dx * i / n, y = y0 + dy * i / n;
        pset(x, y, c, a); pset(x + 1, y, c, a * 2 / 3); pset(x, y + 1, c, a * 2 / 3);
    }
}
static void ring(int cx, int cy, int r, uint32_t c, unsigned a) {
    for (int j = -r - 1; j <= r + 1; j++) for (int i = -r - 1; i <= r + 1; i++) {
        int d = i * i + j * j, lo = (r - 1) * (r - 1), hi = (r + 1) * (r + 1);
        if (d >= lo && d <= hi) pset(cx + i, cy + j, c, a);
    }
}

/* ---------------------------------------------------------------- glyphs (line icons) */
enum { APP_VELA, APP_FIND, APP_SPACES, APP_NOTES, APP_MEDIA, APP_SETTINGS, NAPPS };
static void glyph(int app, int x, int y, int s, uint32_t c) {   /* s = box size */
    int m = s / 2;
    switch (app) {
    case APP_VELA:      /* sail */
        line(x + m - 1, y + 2, x + m - 1, y + s - 6, c, 255);
        line(x + m - 1, y + 2, x + s - 4, y + s - 8, c, 255);
        line(x + s - 4, y + s - 8, x + m - 1, y + s - 8, c, 255);
        line(x + m - 4, y + 7, x + 4, y + s - 8, c, 255);
        line(x + 4, y + s - 8, x + m - 4, y + s - 8, c, 255);
        line(x + 3, y + s - 4, x + s - 3, y + s - 4, c, 255);
        break;
    case APP_FIND:
        ring(x + m - 3, y + m - 3, s / 3, c, 255);
        line(x + m + s / 5, y + m + s / 5, x + s - 3, y + s - 3, c, 255);
        break;
    case APP_SPACES:    /* layers */
        for (int k = 0; k < 3; k++) {
            int yy = y + 4 + k * 6;
            line(x + 2, yy + 6, x + m, yy, c, 255 - k * 50); line(x + m, yy, x + s - 2, yy + 6, c, 255 - k * 50);
            line(x + 2, yy + 6, x + m, yy + 12, c, 255 - k * 50); line(x + m, yy + 12, x + s - 2, yy + 6, c, 255 - k * 50);
        }
        break;
    case APP_NOTES:
        stroke_rrect(x + 4, y + 2, s - 8, s - 4, 4, c, 255); stroke_rrect(x + 5, y + 3, s - 10, s - 6, 3, c, 160);
        for (int k = 0; k < 3; k++) fill_rect(x + 9, y + 9 + k * 6, s - 18 - (k == 2 ? 6 : 0), 2, c, 255);
        break;
    case APP_MEDIA:
        ring(x + m, y + m, m - 2, c, 255);
        line(x + m - 3, y + m - 6, x + m - 3, y + m + 6, c, 255);
        line(x + m - 3, y + m - 6, x + m + 6, y + m, c, 255);
        line(x + m - 3, y + m + 6, x + m + 6, y + m, c, 255);
        break;
    case APP_SETTINGS:
        ring(x + m, y + m, m - 5, c, 255); ring(x + m, y + m, 3, c, 255);
        { static const int dx[8] = { 0, 7, 10, 7, 0, -7, -10, -7 }, dy[8] = { -10, -7, 0, 7, 10, 7, 0, -7 };
          for (int k = 0; k < 8; k++) fill_circle(x + m + dx[k] * (m - 2) / 10, y + m + dy[k] * (m - 2) / 10, 2, c, 255); }
        break;
    }
}

/* ---------------------------------------------------------------- notifications (top-right) */
#define NNOTE 3
struct note { int on; char title[40]; char body[80]; int app; };
static struct note notes[NNOTE];
static void notify(int app, const char *title, const char *body) {
    memmove(&notes[1], &notes[0], sizeof(notes[0]) * (NNOTE - 1));
    notes[0].on = 1; notes[0].app = app;
    kstrcpy(notes[0].title, title, sizeof notes[0].title); kstrcpy(notes[0].body, body, sizeof notes[0].body);
}
#define NOTE_W 380
#define NOTE_H 72
static int note_y(int k) { return 32 + 12 + k * (NOTE_H + 10); }

/* ---------------------------------------------------------------- console model (Vela app) */
#define TCOLS 72
#define TROWS 200
static char tlines[TROWS][TCOLS + 1];
static int tkind[TROWS];        /* 0 text, 1 accent, 2 muted, 3 error, 4 ok, 5 echo */
static int tcount;
static char input[TCOLS + 1]; static int input_len;
static char history[16][TCOLS + 1]; static int hist_n, hist_pos;
static void tnewline(int k) {
    if (tcount == TROWS) { memmove(tlines[0], tlines[1], sizeof(tlines[0]) * (TROWS - 1)); memmove(tkind, tkind + 1, sizeof(int) * (TROWS - 1)); tcount--; }
    tlines[tcount][0] = 0; tkind[tcount] = k; tcount++;
}
static void tprint(const char *s, int k) {
    tnewline(k);
    int col = 0;
    for (; *s; s++) {
        if (*s == '\n' || col == TCOLS) { tnewline(k); col = 0; if (*s == '\n') continue; }
        tlines[tcount - 1][col++] = *s; tlines[tcount - 1][col] = 0;
    }
}
static uint32_t tcol(int k) {
    switch (k) { case 1: return TH->accent; case 2: return TH->muted; case 3: return 0xC2453D; case 4: return 0x2E8B57; default: return TH->text; }
}

/* Notes + Find buffers */
static char note_buf[1200]; static int note_len;
static char find_q[40]; static int find_len;

/* ---------------------------------------------------------------- windows */
struct window { int x, y, w, h, open, min, max, space; int sx, sy, sw, sh; };
static struct window win[NAPPS];
static int zorder[NAPPS] = { APP_MEDIA, APP_SPACES, APP_FIND, APP_SETTINGS, APP_NOTES, APP_VELA };
static int focused = -1;
static const char *app_name(int a) {
    switch (a) {
    case APP_VELA: return "Vela"; case APP_FIND: return T("Find", "Trova"); case APP_SPACES: return "Spaces";
    case APP_NOTES: return T("Notes", "Note"); case APP_MEDIA: return T("Media", "Media"); default: return T("Settings", "Impostazioni");
    }
}
static int visible(int a) { return win[a].open && !win[a].min && win[a].space == cur_space; }
static void raise_win(int id) {
    int k = 0;
    for (int i = 0; i < NAPPS; i++) if (zorder[i] != id) zorder[k++] = zorder[i];
    zorder[k] = id; focused = id;
}
static void refocus(void) {
    focused = -1;
    for (int k = NAPPS - 1; k >= 0; k--) if (visible(zorder[k])) { focused = zorder[k]; return; }
}
static void open_app(int a) {
    if (!win[a].open) win[a].open = 1;
    win[a].min = 0; win[a].space = cur_space;
    raise_win(a);
}

static const char *fake_files[][2] = {
    { "README.txt", "Welcome to VelaOS. Type 'help' in Vela to explore." },
    { "motd", "First light: VelaOS boots natively in VirtualBox." },
    { "roadmap.txt", "next: interrupts, scheduler, storage, in-guest apps, seL4 executor bridge." },
    { "Coastal weekend.mov", "Media sample (placeholder)." },
    { "Clean energy.notes", "Draft notes for the clean energy presentation." },
};
#define NFILES (int)(sizeof fake_files / sizeof fake_files[0])

static void fmt_uptime(char *out) {
    struct rtc t; rtc_read(&t);
    unsigned now = rtc_secs_of_day(&t);
    unsigned up = now >= boot_secs ? now - boot_secs : now + 86400 - boot_secs;
    char b[8]; out[0] = 0;
    utoa(up / 3600, b); kstrcat(out, b, 32); kstrcat(out, "h ", 32);
    utoa((up / 60) % 60, b); kstrcat(out, b, 32); kstrcat(out, "m ", 32);
    utoa(up % 60, b); kstrcat(out, b, 32); kstrcat(out, "s", 32);
}
static unsigned dow(unsigned y, unsigned m, unsigned d) {   /* 0 = Sunday */
    static const int t[] = { 0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4 };
    if (m < 3) y -= 1;
    return (y + y / 4 - y / 100 + y / 400 + t[(m - 1) % 12] + d) % 7;
}
static void fmt_date(char *out, int with_secs) {
    static const char *mon_en[] = { "Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec" };
    static const char *mon_it[] = { "gen","feb","mar","apr","mag","giu","lug","ago","set","ott","nov","dic" };
    static const char *day_en[] = { "Sun","Mon","Tue","Wed","Thu","Fri","Sat" };
    static const char *day_it[] = { "dom","lun","mar","mer","gio","ven","sab" };
    struct rtc t; rtc_read(&t);
    char b[8]; out[0] = 0;
    unsigned wd = dow(t.year, t.mon, t.day);
    kstrcat(out, lang ? day_it[wd] : day_en[wd], 40); kstrcat(out, " ", 40);
    utoa(t.day, b); kstrcat(out, b, 40); kstrcat(out, " ", 40);
    kstrcat(out, lang ? mon_it[(t.mon - 1) % 12] : mon_en[(t.mon - 1) % 12], 40); kstrcat(out, "  ", 40);
    utoa2(t.hour, b); kstrcat(out, b, 40); kstrcat(out, ":", 40);
    utoa2(t.min, b); kstrcat(out, b, 40);
    if (with_secs) { kstrcat(out, ":", 40); utoa2(t.sec, b); kstrcat(out, b, 40); }
}
static unsigned now_secs(void) { struct rtc t; rtc_read(&t); return rtc_secs_of_day(&t); }

/* ---------------------------------------------------------------- power */
static void reboot(void) {
    for (int i = 0; i < 100000 && (inb(0x64) & 2); i++) {}
    outb(0x64, 0xFE);
    for (;;) __asm__ volatile("hlt");
}
static void shutdown(void) { outw(0x4004, 0x3400); outw(0x604, 0x2000); outw(0xB004, 0x2000); }

/* ---------------------------------------------------------------- wallpaper */
static void make_wallpaper(void) {
    for (int y = 0; y < H; y++) for (int x = 0; x < W; x++) {
        uint16_t pixel = vela_wallpaper[(y * WALL_H / H) * WALL_W + x * WALL_W / W];
        uint32_t color = (((pixel >> 11) * 255 / 31) << 16) | ((((pixel >> 5) & 63) * 255 / 63) << 8) | ((pixel & 31) * 255 / 31);
        backbuf[y * W + x] = theme_id == 0 ? blend(color, 0xE1E9F1, 22) : theme_id == 1 ? blend(color, 0xEBB482, 20) : color;
    }
    memcpy(wallpaper, backbuf, (size_t)W * H * 4);
    /* blurred copy for glass: 3 separable box passes */
    memcpy(blurred, wallpaper, (size_t)W * H * 4);
    const int R = 10;
    for (int pass = 0; pass < 3; pass++) {
        for (int y = 0; y < H; y++) {          /* horizontal */
            uint32_t *row = &blurred[y * W], *tmp = backbuf;
            int sr = 0, sg = 0, sb = 0, n = 0;
            for (int x = -R; x < W + R; x++) {
                if (x + R < W) { uint32_t p = row[x + R]; sr += (p >> 16) & 255; sg += (p >> 8) & 255; sb += p & 255; n++; }
                if (x - R - 1 >= 0) { uint32_t p = row[x - R - 1]; sr -= (p >> 16) & 255; sg -= (p >> 8) & 255; sb -= p & 255; n--; }
                if (x >= 0 && x < W) tmp[x] = (uint32_t)((sr / n) << 16 | (sg / n) << 8 | (sb / n));
            }
            memcpy(row, tmp, (size_t)W * 4);
        }
        for (int x = 0; x < W; x++) {          /* vertical */
            uint32_t *tmp = backbuf;
            int sr = 0, sg = 0, sb = 0, n = 0;
            for (int y = -R; y < H + R; y++) {
                if (y + R < H) { uint32_t p = blurred[(y + R) * W + x]; sr += (p >> 16) & 255; sg += (p >> 8) & 255; sb += p & 255; n++; }
                if (y - R - 1 >= 0) { uint32_t p = blurred[(y - R - 1) * W + x]; sr -= (p >> 16) & 255; sg -= (p >> 8) & 255; sb -= p & 255; n--; }
                if (y >= 0 && y < H) tmp[y] = (uint32_t)((sr / n) << 16 | (sg / n) << 8 | (sb / n));
            }
            for (int y = 0; y < H; y++) blurred[y * W + x] = tmp[y];
        }
    }
}
static void set_theme(int t) { theme_id = t; make_wallpaper(); }

/* ---------------------------------------------------------------- console commands */
static void cmd_neofetch(void) {
    char l[96], b[24];
    tprint("      /|        VelaOS " VELA_VERSION, 1);
    tprint("     / |\\       ------------------", 1);
    kstrcpy(l, "    /  | \\      kernel : vela-x86_64 (multiboot2)", sizeof l); tprint(l, 1);
    kstrcpy(l, "   /   |  \\     cpu    : ", sizeof l); kstrcat(l, cpu_brand, sizeof l); tprint(l, 1);
    utoa(mem_kb / 1024, b);
    kstrcpy(l, "  /____|___\\    memory : ", sizeof l); kstrcat(l, b, sizeof l); kstrcat(l, " MiB", sizeof l); tprint(l, 1);
    utoa((uint64_t)W, b); kstrcpy(l, "   \\_______/    display: ", sizeof l); kstrcat(l, b, sizeof l); kstrcat(l, "x", sizeof l);
    utoa((uint64_t)H, b); kstrcat(l, b, sizeof l); tprint(l, 1);
    kstrcpy(l, "    ~~~~~~~     theme  : ", sizeof l); kstrcat(l, TH->name, sizeof l); kstrcat(l, lang ? " (it)" : " (en)", sizeof l); tprint(l, 1);
    fmt_uptime(b); kstrcpy(l, "                uptime : ", sizeof l); kstrcat(l, b, sizeof l); tprint(l, 1);
}
static void toggle_focus(void);
static void run_command(const char *cmd) {
    char line[TCOLS + 8];
    kstrcpy(line, "> ", sizeof line); kstrcat(line, cmd, sizeof line);
    tprint(line, 5);
    while (*cmd == ' ') cmd++;
    if (!*cmd) return;
    if (hist_n < 16) kstrcpy(history[hist_n++], cmd, TCOLS + 1);
    else { memmove(history[0], history[1], sizeof(history[0]) * 15); kstrcpy(history[15], cmd, TCOLS + 1); }
    hist_pos = hist_n;
    char b[64];
    if (kstreq(cmd, "help")) {
        tprint(T("Commands:", "Comandi:"), 1);
        tprint("  help  about  neofetch  uname  mem  cpu  date  uptime", 0);
        tprint("  ls  cat FILE  echo TEXT  clear  notify TEXT", 0);
        tprint("  open vela|find|spaces|notes|media|settings", 0);
        tprint("  theme light|dawn|deep   lang en|it   space 1-4   focus", 0);
        tprint("  reboot  shutdown", 0);
        tprint(T("Keys: F11 Focus mode, Esc exit Focus, Ctrl+Alt+Left/Right switch Space.",
                 "Tasti: F11 modalita Focus, Esc esci, Ctrl+Alt+Sinistra/Destra cambia Space."), 2);
    } else if (kstreq(cmd, "about")) {
        tprint("VelaOS " VELA_VERSION, 1);
        tprint(T("A native x86_64 kernel and compositor shell. No host browser.", "Kernel x86_64 nativo con shell compositor. Nessun browser host."), 0);
    } else if (kstreq(cmd, "uname")) tprint("VelaOS", 0);
    else if (kstreq(cmd, "uname -a")) tprint("VelaOS velaos " VELA_VERSION " x86_64 multiboot2", 0);
    else if (kstreq(cmd, "mem")) { utoa(mem_kb / 1024, b); char l[64] = "Memory: "; kstrcat(l, b, 64); kstrcat(l, " MiB", 64); tprint(l, 0); }
    else if (kstreq(cmd, "cpu")) { char l[96] = "CPU: "; kstrcat(l, cpu_brand, 96); tprint(l, 0); }
    else if (kstreq(cmd, "date")) { fmt_date(b, 1); tprint(b, 0); }
    else if (kstreq(cmd, "uptime")) { fmt_uptime(b); tprint(b, 0); }
    else if (kstreq(cmd, "neofetch")) cmd_neofetch();
    else if (kstreq(cmd, "clear")) tcount = 0;
    else if (kstreq(cmd, "ls")) { for (int i = 0; i < NFILES; i++) tprint(fake_files[i][0], 4); }
    else if (kstartswith(cmd, "cat ")) {
        int ok = 0;
        for (int i = 0; i < NFILES; i++) if (kstreq(cmd + 4, fake_files[i][0])) { tprint(fake_files[i][1], 0); ok = 1; }
        if (!ok) tprint("cat: no such file", 3);
    } else if (kstartswith(cmd, "echo")) tprint(cmd[4] ? cmd + 5 : "", 0);
    else if (kstartswith(cmd, "notify ")) notify(APP_VELA, "Vela", cmd + 7);
    else if (kstartswith(cmd, "open ")) {
        static const char *n[] = { "vela", "find", "spaces", "notes", "media", "settings" };
        int id = -1; for (int i = 0; i < NAPPS; i++) if (kstreq(cmd + 5, n[i])) id = i;
        if (id < 0) tprint("open: unknown app", 3); else open_app(id);
    } else if (kstartswith(cmd, "theme")) {
        const char *a = cmd[5] ? cmd + 6 : "";
        int t = kstreq(a, "light") ? 0 : kstreq(a, "dawn") ? 1 : kstreq(a, "deep") ? 2 : (theme_id + 1) % 3;
        set_theme(t); tprint(T("Theme: ", "Tema: "), 1); kstrcat(tlines[tcount - 1], TH->name, TCOLS + 1);
    } else if (kstreq(cmd, "lang it")) { lang = 1; tprint("Lingua: Italiano", 1); }
    else if (kstreq(cmd, "lang en")) { lang = 0; tprint("Language: English", 1); }
    else if (kstartswith(cmd, "space ") && cmd[6] >= '1' && cmd[6] <= '4') { cur_space = cmd[6] - '1'; win[APP_VELA].space = cur_space; refocus(); raise_win(APP_VELA); }
    else if (kstreq(cmd, "focus")) toggle_focus();
    else if (kstreq(cmd, "reboot")) { tprint("Rebooting...", 1); reboot(); }
    else if (kstreq(cmd, "shutdown") || kstreq(cmd, "poweroff")) { tprint("Powering off...", 1); shutdown(); tprint("ACPI power-off not available; close the VM window.", 3); }
    else { char l[96]; kstrcpy(l, cmd, 40); kstrcat(l, T(": unknown command (try 'help')", ": comando sconosciuto (prova 'help')"), 96); tprint(l, 3); }
}

/* ---------------------------------------------------------------- top bar */
#define BAR_H 32
struct hit { int x, y, w, h; };
static struct hit hit_menu[5], hit_spaces[NSPACES];
static int nmenu;
static void draw_bar(void) {
    glass_rrect(0, 0, W, BAR_H, 0, TH->glass, TH->glass_a);
    fill_rect(0, BAR_H - 1, W, 1, 0xFFFFFF, 30);
    glyph(APP_VELA, 12, 6, 20, 0xFFFFFF);
    draw_text(&font_bold, 38, 7, "VelaOS", 0xFFFFFF);
    int x = 116;
    const char *m_app[] = { NULL, T("File", "File"), T("Edit", "Modifica"), T("View", "Vista"), T("Window", "Finestra") };
    const char *m_sys[] = { "Spaces", T("Find", "Cerca"), T("Appearance", "Aspetto"), T("Settings", "Impostazioni") };
    nmenu = 0;
    if (focused >= 0 && visible(focused)) {
        m_app[0] = app_name(focused);
        for (int i = 0; i < 5; i++) {
            int w = draw_text(i ? &font_ui : &font_bold, x, 7, m_app[i], 0xFFFFFF);
            hit_menu[nmenu++] = (struct hit){ x - 6, 0, w + 12, BAR_H }; x += w + 20;
        }
    } else for (int i = 0; i < 4; i++) {
        int w = draw_text(&font_ui, x, 7, m_sys[i], 0xFFFFFF);
        hit_menu[nmenu++] = (struct hit){ x - 6, 0, w + 12, BAR_H }; x += w + 20;
    }
    /* right side: spaces dots, search, network, volume, battery, clock */
    char d[40]; fmt_date(d, 0);
    int tw = text_width(&font_bold, d), rx = W - tw - 16;
    draw_text(&font_bold, rx, 7, d, 0xFFFFFF);
    int ix = rx - 34;
    fill_rrect(ix, 11, 20, 10, 3, 0xFFFFFF, 0); stroke_rrect(ix, 11, 20, 10, 3, 0xFFFFFF, 230);
    fill_rect(ix + 20, 14, 2, 4, 0xFFFFFF, 230); fill_rect(ix + 2, 13, 13, 6, 0xFFFFFF, 230);       /* battery */
    ix -= 30;
    fill_rect(ix, 13, 4, 6, 0xFFFFFF, 230); line(ix + 4, 13, ix + 9, 9, 0xFFFFFF, 230); line(ix + 4, 19, ix + 9, 23, 0xFFFFFF, 230);
    line(ix + 9, 9, ix + 9, 23, 0xFFFFFF, 230); ring(ix + 11, 16, 5, 0xFFFFFF, 90);                    /* volume */
    ix -= 30;
    for (int r = 3; r <= 11; r += 4) for (int a = -r; a <= r; a++) { int yy = isqrt(r * r - a * a); pset(ix + 10 + a, 22 - yy, 0xFFFFFF, 230); }
    fill_circle(ix + 10, 21, 2, 0xFFFFFF, 255);                                                     /* wifi */
    ix -= 30;
    ring(ix + 8, 14, 5, 0xFFFFFF, 230); line(ix + 12, 18, ix + 16, 22, 0xFFFFFF, 230);               /* search */
    ix -= 22 + NSPACES * 16;
    for (int sp = 0; sp < NSPACES; sp++) {
        int cx = ix + sp * 16 + 6;
        if (sp == cur_space) fill_rrect(cx - 6, 12, 16, 8, 4, 0xFFFFFF, 235);
        else fill_circle(cx + 2, 16, 3, 0xFFFFFF, 120);
        hit_spaces[sp] = (struct hit){ cx - 6, 0, 16, BAR_H };
    }
}

/* ---------------------------------------------------------------- dock */
#define DOCK_SLOT 56
#define DOCK_GAP 12
#define DOCK_H 72
static int dock_x0, dock_y0, hover_dock = -1;
static void dock_geom(void) {
    int dw = NAPPS * DOCK_SLOT + (NAPPS - 1) * DOCK_GAP + 2 * 14;
    dock_x0 = (W - dw) / 2; dock_y0 = H - DOCK_H - 16;
}
static void draw_dock(void) {
    dock_geom();
    int dw = NAPPS * DOCK_SLOT + (NAPPS - 1) * DOCK_GAP + 2 * 14;
    soft_shadow(dock_x0, dock_y0, dw, DOCK_H, 22, 0);
    glass_rrect(dock_x0, dock_y0, dw, DOCK_H, 22, 0x182532, 180);
    stroke_rrect(dock_x0, dock_y0, dw, DOCK_H, 22, 0xFFFFFF, 45);
    for (int i = 0; i < NAPPS; i++) {
        int sx = dock_x0 + 14 + i * (DOCK_SLOT + DOCK_GAP), sy = dock_y0 + 8;
        int lift = (hover_dock == i && !reduce_motion) ? 3 : 0;
        int active = (focused == i && visible(i));
        if (active) fill_rrect(sx, sy - lift, DOCK_SLOT, DOCK_SLOT, 14, 0xFFFFFF, 48);
        else if (hover_dock == i) fill_rrect(sx, sy - lift, DOCK_SLOT, DOCK_SLOT, 14, 0xFFFFFF, 22);
        glyph(i, sx + 14, sy + 14 - lift, 28, 0xFFFFFF);
        if (active) fill_rrect(sx + 16, dock_y0 + DOCK_H - 7, DOCK_SLOT - 32, 2, 1, TEAL_BRIGHT, 255);
        else if (win[i].open) fill_circle(sx + DOCK_SLOT / 2, dock_y0 + DOCK_H - 6, 2, 0xFFFFFF, 160);
        if (hover_dock == i) {
            const char *n = app_name(i); int tw = text_width(&font_ui, n);
            int tx = sx + DOCK_SLOT / 2 - tw / 2 - 10;
            glass_rrect(tx, dock_y0 - 36, tw + 20, 26, 9, 0x182532, 200);
            draw_text(&font_ui, tx + 10, dock_y0 - 32, n, 0xFFFFFF);
        }
    }
}
static int dock_hit(int x, int y) {
    if (focus_mode || welcome_open) return -1;
    for (int i = 0; i < NAPPS; i++) {
        int sx = dock_x0 + 14 + i * (DOCK_SLOT + DOCK_GAP), sy = dock_y0 + 8;
        if (x >= sx && x < sx + DOCK_SLOT && y >= sy - 4 && y < sy + DOCK_SLOT + 6) return i;
    }
    return -1;
}

/* ---------------------------------------------------------------- window geometry */
#define TITLE_H 40
static void win_rect(int a, int *x, int *y, int *w, int *h, int *r) {
    struct window *wn = &win[a];
    if (focus_mode && a == focused) { int mx2 = W * 4 / 100, my2 = H * 4 / 100; *x = mx2; *y = my2; *w = W - 2 * mx2; *h = H - 2 * my2; *r = 16; return; }
    if (wn->max) { dock_geom(); *x = 0; *y = BAR_H; *w = W; *h = dock_y0 - 10 - BAR_H; *r = 0; return; }
    *x = wn->x; *y = wn->y; *w = wn->w; *h = wn->h; *r = 16;
}
static void caption_hits(int x, int w, int y, struct hit *mn, struct hit *mx2, struct hit *cl) {
    *cl = (struct hit){ x + w - 40, y + 4, 32, 32 };
    *mx2 = (struct hit){ x + w - 76, y + 4, 32, 32 };
    *mn = (struct hit){ x + w - 112, y + 4, 32, 32 };
}
static int in_hit(struct hit *h, int x, int y) { return x >= h->x && x < h->x + h->w && y >= h->y && y < h->y + h->h; }

/* ---------------------------------------------------------------- app content */
static int cursor_on = 1;
static void draw_vela(int x, int y, int w, int h) {
    const font_t *f = &font_mono; int lh = 19;
    int top = y + TITLE_H + 10, rows = (h - TITLE_H - 60) / lh;
    int start = tcount > rows ? tcount - rows : 0;
    int yy = top;
    for (int i = start; i < tcount; i++, yy += lh) draw_text(f, x + 18, yy - 2, tlines[i], tcol(tkind[i]));
    /* prompt pill at bottom */
    int py = y + h - 46;
    fill_rrect(x + 12, py, w - 24, 34, 17, TH->side, 255);
    stroke_rrect(x + 12, py, w - 24, 34, 17, focused == APP_VELA ? TH->accent : TH->divider, 255);
    int px = x + 28;
    if (!input_len) draw_text(F_UI, px, py + 8, T("Ask Vela or type a command - try 'help'", "Chiedi a Vela o scrivi un comando - prova 'help'"), TH->muted);
    px += draw_text(f, px, py + 4, input, TH->text);
    if (cursor_on && focused == APP_VELA) fill_rect(px + 1, py + 8, 2, 18, TH->accent, 255);
}
static void draw_find(int x, int y, int w, int h) {
    int py = y + TITLE_H + 14;
    fill_rrect(x + 20, py, w - 40, 38, 19, TH->side, 255);
    stroke_rrect(x + 20, py, w - 40, 38, 19, focused == APP_FIND ? TH->accent : TH->divider, 255);
    ring(x + 40, py + 17, 6, TH->muted, 255); line(x + 44, py + 21, x + 49, py + 26, TH->muted, 255);
    int tx = x + 58;
    if (!find_len) draw_text(F_UI, tx, py + 10, T("Find by meaning...", "Trova per significato..."), TH->muted);
    tx += draw_text(F_UI, tx, py + 10, find_q, TH->text);
    if (cursor_on && focused == APP_FIND) fill_rect(tx + 1, py + 10, 2, 18, TH->accent, 255);
    int ry = py + 56, shown = 0;
    for (int i = 0; i < NFILES; i++) {
        const char *n = fake_files[i][0]; int match = !find_len;
        for (int k = 0; !match && n[k]; k++) { int j = 0; while (find_q[j] && n[k + j] && ((n[k + j] | 32) == (find_q[j] | 32))) j++; if (!find_q[j]) match = 1; }
        if (!match || ry > y + h - 60) continue;
        fill_rrect(x + 20, ry, w - 40, 50, 12, shown == 0 && find_len ? TH->accent_soft : TH->side, 255);
        draw_text(F_BOLD, x + 36, ry + 6, n, TH->text);
        char b[48]; kstrcpy(b, fake_files[i][1], 48); draw_text(F_UI, x + 36, ry + 26, b, TH->muted);
        ry += 58; shown++;
    }
    if (!shown) draw_text(F_UI, x + 24, ry, T("No results on this device.", "Nessun risultato su questo dispositivo."), TH->muted);
}
static void draw_spaces(int x, int y, int w, int h) {
    int tw = (w - 60) / 2, th = (h - TITLE_H - 60) / 2;
    for (int sp = 0; sp < NSPACES; sp++) {
        int sx = x + 20 + (sp % 2) * (tw + 20), sy = y + TITLE_H + 14 + (sp / 2) * (th + 20);
        /* thumbnail = scaled wallpaper */
        for (int j = 0; j < th; j++) for (int i = 0; i < tw; i++) pset(sx + i, sy + j, wallpaper[(j * H / th) * W + (i * W / tw)], 255);
        int n = 0;
        for (int a = 0; a < NAPPS; a++) if (win[a].open && !win[a].min && win[a].space == sp) {
            int wx = sx + win[a].x * tw / W, wy = sy + win[a].y * th / H;
            fill_rect(wx, wy, win[a].w * tw / W, win[a].h * th / H, TH->win, 230); n++;
        }
        stroke_rrect(sx - 2, sy - 2, tw + 4, th + 4, 6, sp == cur_space ? TH->accent : TH->divider, 255);
        stroke_rrect(sx - 1, sy - 1, tw + 2, th + 2, 6, sp == cur_space ? TH->accent : TH->divider, 255);
        char b[24] = "Space "; char c[4]; utoa((uint64_t)sp + 1, c); kstrcat(b, c, 24);
        fill_rrect(sx + 8, sy + 8, text_width(&font_bold, b) + 16, 24, 12, 0x0E223B, 180);
        draw_text(&font_bold, sx + 16, sy + 10, b, 0xFFFFFF);
    }
}
static void draw_notes(int x, int y, int w, int h) {
    fill_rect(x + 1, y + TITLE_H, 170, h - TITLE_H - 16, TH->side, 255);
    const char *side[] = { T("All notes", "Tutte le note"), T("Today", "Oggi"), T("Projects", "Progetti") };
    for (int i = 0; i < 3; i++) {
        if (!i) fill_rrect(x + 10, y + TITLE_H + 10, 150, 30, 10, TH->accent_soft, 255);
        draw_text(F_UI, x + 22, y + TITLE_H + 16 + i * 36, side[i], i ? TH->muted : TH->text);
    }
    int tx = x + 196, ty = y + TITLE_H + 16;
    draw_text(&font_title, tx, ty, T("Untitled note", "Nota senza titolo"), TH->text); ty += 36;
    int cx = tx, lh = F_UI->h + 3;
    for (int i = 0; i < note_len; i++) {
        char ch = note_buf[i];
        if (ch == '\n' || cx > x + w - 40) { cx = tx; ty += lh; if (ch == '\n') continue; }
        cx += draw_char(F_UI, cx, ty, ch, TH->text);
    }
    if (!note_len) draw_text(F_UI, tx, ty, T("Start typing...", "Inizia a scrivere..."), TH->muted);
    if (cursor_on && focused == APP_NOTES) fill_rect(cx + 1, ty + 1, 2, F_UI->h - 2, TH->accent, 255);
}
static void draw_media(int x, int y, int w, int h) {
    int ax = x + 24, ay = y + TITLE_H + 12, aw = w - 48, ah = h - TITLE_H - 110;
    for (int j = 0; j < ah; j++) for (int i = 0; i < aw; i++) pset(ax + i, ay + j, blurred[((j * (H - 1)) / ah) * W + (i * (W - 1)) / aw], 255);
    fill_circle(ax + aw / 2, ay + ah / 2, 30, 0xFFFFFF, 200);
    line(ax + aw / 2 - 8, ay + ah / 2 - 12, ax + aw / 2 - 8, ay + ah / 2 + 12, 0x14233A, 255);
    line(ax + aw / 2 - 8, ay + ah / 2 - 12, ax + aw / 2 + 13, ay + ah / 2, 0x14233A, 255);
    line(ax + aw / 2 - 8, ay + ah / 2 + 12, ax + aw / 2 + 13, ay + ah / 2, 0x14233A, 255);
    draw_text(F_BOLD, ax, ay + ah + 14, "Coastal weekend", TH->text);
    draw_text(F_UI, ax + aw - 90, ay + ah + 14, "00:42 / 03:10", TH->muted);
    fill_rrect(ax, ay + ah + 44, aw, 4, 2, TH->divider, 255);
    fill_rrect(ax, ay + ah + 44, aw * 22 / 100, 4, 2, TEAL_BRIGHT, 255);
    fill_circle(ax + aw * 22 / 100, ay + ah + 46, 6, TEAL_BRIGHT, 255);
}
static struct hit hit_theme[3], hit_lang[2], hit_larger, hit_motion;
static void toggle(int x, int y, int on) {
    fill_rrect(x, y, 40, 22, 11, on ? TH->accent : TH->divider, 255);
    fill_circle(on ? x + 29 : x + 11, y + 11, 8, 0xFFFFFF, 255);
}
static void draw_settings(int x, int y, int w, int h) {
    int sx = x + 24, sy = y + TITLE_H + 14;
    draw_text(F_BOLD, sx, sy, T("Appearance", "Aspetto"), TH->text); sy += 28;
    static const uint32_t sw_a[3] = { 0xF4F1EC, 0xF7C9A0, 0x0F2333 }, sw_b[3] = { 0xC9D6DC, 0x8AA0C8, 0x2D6877 };
    for (int t = 0; t < 3; t++) {
        int bx = sx + t * 120;
        for (int j = 0; j < 56; j++) fill_rect(bx, sy + j, 104, 1, blend(sw_a[t], sw_b[t], (unsigned)(j * 255 / 56)), 255);
        stroke_rrect(bx - 2, sy - 2, 108, 60, 8, t == theme_id ? TH->accent : TH->divider, 255);
        if (t == theme_id) stroke_rrect(bx - 3, sy - 3, 110, 62, 9, TH->accent, 255);
        draw_text(F_UI, bx, sy + 62, t == 0 ? T("Light", "Chiaro") : t == 1 ? T("Dawn", "Alba") : T("Deep", "Profondo"), TH->text);
        hit_theme[t] = (struct hit){ bx - 3, sy - 3, 110, 84 };
    }
    sy += 96;
    draw_text(F_BOLD, sx, sy, T("Language", "Lingua"), TH->text); sy += 26;
    for (int l = 0; l < 2; l++) {
        const char *n = l ? "Italiano" : "English"; int tw = text_width(F_UI, n) + 28;
        int bx = sx + l * 130;
        fill_rrect(bx, sy, tw, 30, 15, l == lang ? TH->accent : TH->side, 255);
        draw_text(F_UI, bx + 14, sy + 6, n, l == lang ? 0xFFFFFF : TH->text);
        hit_lang[l] = (struct hit){ bx, sy, tw, 30 };
    }
    sy += 46;
    draw_text(F_BOLD, sx, sy, T("Accessibility", "Accessibilita"), TH->text); sy += 28;
    draw_text(F_UI, sx, sy + 2, T("Larger text", "Testo piu grande"), TH->text); toggle(x + w - 70, sy, larger_text); hit_larger = (struct hit){ x + w - 70, sy, 40, 22 }; sy += 32;
    draw_text(F_UI, sx, sy + 2, T("Reduce motion", "Riduci movimento"), TH->text); toggle(x + w - 70, sy, reduce_motion); hit_motion = (struct hit){ x + w - 70, sy, 40, 22 }; sy += 36;
    fill_rect(sx, sy, w - 48, 1, TH->divider, 255); sy += 12;
    char b[64], c[24];
    kstrcpy(b, "VelaOS " VELA_VERSION "  -  ", 64); utoa(mem_kb / 1024, c); kstrcat(b, c, 64); kstrcat(b, " MiB  -  ", 64);
    utoa((uint64_t)W, c); kstrcat(b, c, 64); kstrcat(b, "x", 64); utoa((uint64_t)H, c); kstrcat(b, c, 64);
    draw_text(F_UI, sx, sy, b, TH->muted);
    if (sy + 24 < y + h - 10) { char cb[48]; kstrcpy(cb, cpu_brand, 48); draw_text(F_UI, sx, sy + 22, cb, TH->muted); }
}

static void draw_window(int a) {
    int x, y, w, h, r; win_rect(a, &x, &y, &w, &h, &r);
    int foc = a == focused;
    if (r) soft_shadow(x, y, w, h, r, foc);
    fill_rrect(x, y, w, h, r, TH->win, 255);
    stroke_rrect(x, y, w, h, r, theme_id == 2 ? 0x22313C : 0xD8D2CB, 255);
    /* integrated title row */
    glyph(a, x + 16, y + 10, 20, foc ? TH->accent : TH->muted);
    draw_text(F_BOLD, x + 44, y + 11, app_name(a), foc ? TH->text : TH->muted);
    fill_rect(x + 1, y + TITLE_H, w - 2, 1, TH->divider, 255);
    struct hit mn, mxh, cl; caption_hits(x, w, y, &mn, &mxh, &cl);
    uint32_t ink = foc ? TH->ink : TH->divider;
    fill_rect(mn.x + 10, mn.y + 16, 12, 1, ink, 255); fill_rect(mn.x + 10, mn.y + 17, 12, 1, ink, 120);
    stroke_rrect(mxh.x + 10, mxh.y + 10, 12, 12, 2, ink, 255);
    line(cl.x + 10, cl.y + 10, cl.x + 21, cl.y + 21, ink, 220); line(cl.x + 21, cl.y + 10, cl.x + 10, cl.y + 21, ink, 220);
    switch (a) {
    case APP_VELA: draw_vela(x, y, w, h); break;
    case APP_FIND: draw_find(x, y, w, h); break;
    case APP_SPACES: draw_spaces(x, y, w, h); break;
    case APP_NOTES: draw_notes(x, y, w, h); break;
    case APP_MEDIA: draw_media(x, y, w, h); break;
    case APP_SETTINGS: draw_settings(x, y, w, h); break;
    }
}

static void draw_notes_overlay(void) {
    for (int k = 0; k < NNOTE; k++) {
        if (!notes[k].on) continue;
        int x = W - NOTE_W - 16, y = note_y(k);
        soft_shadow(x, y, NOTE_W, NOTE_H, 14, 0);
        glass_rrect(x, y, NOTE_W, NOTE_H, 14, 0x04151C, 215);
        stroke_rrect(x, y, NOTE_W, NOTE_H, 14, 0xFFFFFF, 35);
        fill_rrect(x + 14, y + 16, 40, 40, 12, 0xFFFFFF, 30);
        glyph(notes[k].app, x + 22, y + 24, 24, 0xFFFFFF);
        draw_text(&font_bold, x + 66, y + 14, notes[k].title, 0xFFFFFF);
        draw_text(&font_ui, x + 66, y + 38, notes[k].body, 0xC9D6DC);
        line(x + NOTE_W - 26, y + 14, x + NOTE_W - 17, y + 23, 0xFFFFFF, 200);
        line(x + NOTE_W - 17, y + 14, x + NOTE_W - 26, y + 23, 0xFFFFFF, 200);
    }
}

#include "glass_ui.inc"

/* ---------------------------------------------------------------- welcome (first boot) */
static struct hit w_theme[3], w_lang[2], w_larger, w_motion, w_cta;
static void draw_welcome(void) {
    fill_rect(0, 0, W, H, 0x000000, 70);
    int cw = W * 62 / 100 < 760 ? (W * 62 / 100 > 680 ? W * 62 / 100 : 680) : 760, ch = 520;
    if (cw > W - 40) cw = W - 40;
    int x = (W - cw) / 2, y = (H - ch) / 2;
    soft_shadow(x, y, cw, ch, 16, 1);
    fill_rrect(x, y, cw, ch, 16, 0xFAF4E9, 255);
    glyph(APP_VELA, x + cw / 2 - 16, y + 22, 32, 0x2D6877);
    const char *title = T("Welcome to VelaOS", "Benvenuto in VelaOS");
    draw_text(&font_serif, x + (cw - text_width(&font_serif, title)) / 2, y + 60, title, 0x14233A);
    const char *sub = T("A workspace that understands what you need.", "Uno spazio di lavoro che capisce cosa ti serve.");
    draw_text(&font_ui, x + (cw - text_width(&font_ui, sub)) / 2, y + 108, sub, 0x5A6675);
    int pw = (cw - 3 * 20) / 2, ph = 150;
    const char *heads[4] = { T("Appearance", "Aspetto"), T("Language", "Lingua"), T("Privacy", "Privacy"), T("Accessibility", "Accessibilita") };
    for (int p = 0; p < 4; p++) {
        int px = x + 20 + (p % 2) * (pw + 20), py = y + 140 + (p / 2) * (ph + 14);
        fill_rrect(px, py, pw, ph, 12, 0xFFFFFF, 255); stroke_rrect(px, py, pw, ph, 12, 0xE6E1DB, 255);
        draw_text(&font_bold, px + 16, py + 12, heads[p], 0x14233A);
        if (p == 0) {
            static const uint32_t a[3] = { 0xF4F1EC, 0xF7C9A0, 0x0F2333 }, b[3] = { 0xC9D6DC, 0x8AA0C8, 0x2D6877 };
            int sw = (pw - 32 - 2 * 12) / 3;
            for (int t = 0; t < 3; t++) {
                int sx = px + 16 + t * (sw + 12), sy = py + 42;
                for (int j = 0; j < 60; j++) fill_rect(sx, sy + j, sw, 1, blend(a[t], b[t], (unsigned)(j * 255 / 60)), 255);
                stroke_rrect(sx - 2, sy - 2, sw + 4, 64, 8, t == theme_id ? 0x2D6877 : 0xE6E1DB, 255);
                if (t == theme_id) stroke_rrect(sx - 3, sy - 3, sw + 6, 66, 9, 0x2D6877, 255);
                const char *n = t == 0 ? T("Light", "Chiaro") : t == 1 ? T("Dawn", "Alba") : T("Deep", "Profondo");
                draw_text(&font_ui, sx + (sw - text_width(&font_ui, n)) / 2, sy + 70, n, 0x14233A);
                w_theme[t] = (struct hit){ sx - 3, sy - 3, sw + 6, 92 };
            }
        } else if (p == 1) {
            for (int l = 0; l < 2; l++) {
                int by = py + 44 + l * 44;
                fill_rrect(px + 16, by, pw - 32, 36, 12, l == lang ? 0xD9E3E0 : 0xF5F2EE, 255);
                if (l == lang) stroke_rrect(px + 16, by, pw - 32, 36, 12, 0x2D6877, 255);
                draw_text(&font_ui, px + 32, by + 9, l ? "Italiano" : T("English (default)", "English (predefinito)"), 0x14233A);
                ring(px + pw - 40, by + 18, 7, 0x2D6877, 255); if (l == lang) fill_circle(px + pw - 40, by + 18, 4, 0x2D6877, 255);
                w_lang[l] = (struct hit){ px + 16, by, pw - 32, 36 };
            }
        } else if (p == 2) {
            fill_rrect(px + 16, py + 44, pw - 32, 36, 12, 0xD9E3E0, 255); stroke_rrect(px + 16, py + 44, pw - 32, 36, 12, 0x2D6877, 255);
            draw_text(&font_ui, px + 32, py + 53, T("On this device (default)", "Su questo dispositivo (predef.)"), 0x14233A);
            line(px + pw - 46, py + 62, px + pw - 41, py + 67, 0x2D6877, 255); line(px + pw - 41, py + 67, px + pw - 32, py + 56, 0x2D6877, 255);
            draw_text(&font_ui, px + 16, py + 94, T("No account. No network needed.", "Nessun account. Nessuna rete."), 0x5A6675);
            draw_text(&font_ui, px + 16, py + 116, T("Your data stays private.", "I tuoi dati restano privati."), 0x5A6675);
        } else {
            draw_text(&font_ui, px + 16, py + 50, T("Larger text", "Testo piu grande"), 0x14233A);
            fill_rrect(px + pw - 60, py + 48, 40, 22, 11, larger_text ? 0x2D6877 : 0xD8D2CB, 255); fill_circle(larger_text ? px + pw - 31 : px + pw - 49, py + 59, 8, 0xFFFFFF, 255);
            w_larger = (struct hit){ px + 16, py + 44, pw - 32, 32 };
            draw_text(&font_ui, px + 16, py + 92, T("Reduce motion", "Riduci movimento"), 0x14233A);
            fill_rrect(px + pw - 60, py + 90, 40, 22, 11, reduce_motion ? 0x2D6877 : 0xD8D2CB, 255); fill_circle(reduce_motion ? px + pw - 31 : px + pw - 49, py + 101, 8, 0xFFFFFF, 255);
            w_motion = (struct hit){ px + 16, py + 86, pw - 32, 32 };
        }
    }
    const char *cta = T("Make yourself at home  ->", "Mettiti a tuo agio  ->");
    int bw = text_width(&font_bold, cta) + 48, bx = x + (cw - bw) / 2, by = y + ch - 56;
    fill_rrect(bx, by, bw, 40, 20, 0x2D6877, 255);
    draw_text(&font_bold, bx + 24, by + 10, cta, 0xFFFFFF);
    w_cta = (struct hit){ bx, by, bw, 40 };
    const char *hint = T("Press Enter to continue", "Premi Invio per continuare");
    draw_text(&font_ui, x + (cw - text_width(&font_ui, hint)) / 2, y + ch + 14, hint, 0xDDE6EE);
}
static void finish_welcome(void) {
    welcome_open = 0;
    glass_kind = 0; focused = -1;
    serial_puts("VelaOS Glass: idle desktop, no application windows resident\r\n");
    notify(APP_VELA, T("Welcome aboard", "Benvenuto a bordo"), T("Notes: Ctrl+Enter confirms. Esc closes.", "Appunti: Ctrl+Invio conferma. Esc chiude."));
}

/* ---------------------------------------------------------------- render */
static int mx, my, mbtn;
static void draw_cursor(void) {
    static const char *arrow[] = {
        "X", "XX", "X.X", "X..X", "X...X", "X....X", "X.....X", "X......X", "X.......X", "X........X",
        "X.....XXXXX", "X..X..X", "X.X X..X", "XX  X..X", "X    X..X", "     X..X", "      XX", 0 };
    for (int j = 0; arrow[j]; j++) for (int i = 0; arrow[j][i]; i++) {
        if (arrow[j][i] == 'X') pset(mx + i, my + j, 0x000000, 255);
        else if (arrow[j][i] == '.') pset(mx + i, my + j, 0xFFFFFF, 255);
    }
}
static void render(void) {
    memcpy(backbuf, wallpaper, (size_t)W * H * 4);
    if (welcome_open) { draw_welcome(); draw_cursor(); blit(); return; }
    if (focus_mode && focused >= 0 && visible(focused)) {
        fill_rect(0, 0, W, H, 0x000000, 90);
        draw_window(focused);
        if (now_secs() < hint_until) {
            const char *hnt = T("Focus mode  -  Esc or F11 to exit", "Modalita Focus  -  Esc o F11 per uscire");
            int tw = text_width(&font_ui, hnt);
            glass_rrect((W - tw) / 2 - 16, 8, tw + 32, 28, 14, 0x04151C, 215);
            draw_text(&font_ui, (W - tw) / 2, 12, hnt, 0xFFFFFF);
        }
        draw_cursor(); blit(); return;
    }
    focus_mode = 0;
    for (int k = 0; k < NAPPS; k++) if (visible(zorder[k])) draw_window(zorder[k]);
    draw_bar();
    draw_dock();
    draw_glass();
    draw_notes_overlay();
    draw_cursor();
    blit();
}

/* ---------------------------------------------------------------- input */
static void toggle_focus(void) {
    if (focus_mode) { focus_mode = 0; return; }
    if (focused < 0 || !visible(focused)) return;
    focus_mode = 1; hint_until = now_secs() + 3;
}
static void switch_space(int d) {
    cur_space = (cur_space + d + NSPACES) % NSPACES;
    focus_mode = 0; refocus();
    char b[40]; kstrcpy(b, "Space ", 40); char c[4]; utoa((uint64_t)cur_space + 1, c); kstrcat(b, c, 40);
    notes[NNOTE - 1].on = 0;
    notify(APP_SPACES, b, T("Ctrl+Alt+Left/Right to switch Spaces.", "Ctrl+Alt+Sinistra/Destra per cambiare."));
}
static const char keymap[128] = {
    0, 27, '1','2','3','4','5','6','7','8','9','0','-','=','\b','\t','q','w','e','r','t','y','u','i','o','p','[',']','\n',
    0, 'a','s','d','f','g','h','j','k','l',';','\'','`', 0,'\\','z','x','c','v','b','n','m',',','.','/', 0,'*', 0,' ' };
static const char keymap_shift[128] = {
    0, 27, '!','@','#','$','%','^','&','*','(',')','_','+','\b','\t','Q','W','E','R','T','Y','U','I','O','P','{','}','\n',
    0, 'A','S','D','F','G','H','J','K','L',':','"','~', 0,'|','Z','X','C','V','B','N','M','<','>','?', 0,'*', 0,' ' };
static int shift, caps, ext, ctrl, alt;

static void ps2_wait_write(void) { for (int i = 0; i < 100000 && (inb(0x64) & 2); i++) {} }
static void ps2_wait_read(void) { for (int i = 0; i < 100000 && !(inb(0x64) & 1); i++) {} }
static void mouse_write(uint8_t v) { ps2_wait_write(); outb(0x64, 0xD4); ps2_wait_write(); outb(0x60, v); ps2_wait_read(); (void)inb(0x60); }
static void mouse_init(void) {
    ps2_wait_write(); outb(0x64, 0xA8);
    ps2_wait_write(); outb(0x64, 0x20); ps2_wait_read(); uint8_t st = inb(0x60);
    st |= 2; st &= ~0x20;
    ps2_wait_write(); outb(0x64, 0x60); ps2_wait_write(); outb(0x60, st);
    mouse_write(0xF6); mouse_write(0xF4);
}

static int dragging = -1, drag_dx, drag_dy, prev_btn;
static void dock_click(int i) {
    if (i == APP_VELA || i == APP_FIND) { glass_open(1); return; }
    if (i == APP_NOTES) { glass_open(2); return; }
    if (i == APP_MEDIA) { glass_open(3); return; }
    glass_release();
    if (win[i].open && !win[i].min && win[i].space == cur_space && focused == i) { win[i].min = 1; refocus(); }
    else open_app(i);
}
static void mouse_click(int x, int y) {
    if (welcome_open) {
        for (int t = 0; t < 3; t++) if (in_hit(&w_theme[t], x, y)) set_theme(t);
        for (int l = 0; l < 2; l++) if (in_hit(&w_lang[l], x, y)) lang = l;
        if (in_hit(&w_larger, x, y)) larger_text ^= 1;
        if (in_hit(&w_motion, x, y)) reduce_motion ^= 1;
        if (in_hit(&w_cta, x, y)) finish_welcome();
        return;
    }
    if (!focus_mode) {
        for (int k = 0; k < NNOTE; k++) if (notes[k].on) {
            int nx = W - NOTE_W - 16, ny = note_y(k);
            if (x >= nx && x < nx + NOTE_W && y >= ny && y < ny + NOTE_H) {
                if (x > nx + NOTE_W - 36 && y < ny + 32) { memmove(&notes[k], &notes[k + 1], sizeof(notes[0]) * (NNOTE - 1 - k)); notes[NNOTE - 1].on = 0; }
                return;
            }
        }
        int d = dock_hit(x, y);
        if (d >= 0) { dock_click(d); return; }
        if (y < BAR_H) {
            char date[40]; fmt_date(date, 0);
            int date_x = W - text_width(&font_bold, date) - 16;
            if (x >= date_x - 94 && x < date_x - 64) { glass_open(4); return; }
            if (x >= date_x - 124 && x < date_x - 94) { glass_open(1); return; }
            for (int sp = 0; sp < NSPACES; sp++) if (in_hit(&hit_spaces[sp], x, y)) { switch_space(sp - cur_space); return; }
            for (int i = 0; i < nmenu; i++) if (in_hit(&hit_menu[i], x, y)) {
                if (focused >= 0 && visible(focused)) { if (i == 3) toggle_focus(); if (i == 4) win[focused].max ^= 1; }
                else { if (i == 0) open_app(APP_SPACES); if (i == 1) glass_open(1); if (i == 2 || i == 3) { glass_release(); open_app(APP_SETTINGS); } }
            }
            return;
        }
    }
    if (glass_click(x, y)) return;
    for (int k = NAPPS - 1; k >= 0; k--) {
        int id = zorder[k];
        if (!visible(id) || (focus_mode && id != focused)) continue;
        int wx, wy, ww, wh, r; win_rect(id, &wx, &wy, &ww, &wh, &r);
        if (x < wx || x >= wx + ww || y < wy || y >= wy + wh) continue;
        raise_win(id);
        if (y < wy + TITLE_H) {
            struct hit mn, mxh, cl; caption_hits(wx, ww, wy, &mn, &mxh, &cl);
            if (in_hit(&cl, x, y)) { win[id].open = 0; focus_mode = 0; refocus(); return; }
            if (in_hit(&mxh, x, y)) { if (focus_mode) focus_mode = 0; else win[id].max ^= 1; return; }
            if (in_hit(&mn, x, y)) { win[id].min = 1; focus_mode = 0; refocus(); return; }
            if (!win[id].max && !focus_mode) { dragging = id; drag_dx = x - wx; drag_dy = y - wy; }
            return;
        }
        if (id == APP_SETTINGS) {
            for (int t = 0; t < 3; t++) if (in_hit(&hit_theme[t], x, y)) set_theme(t);
            for (int l = 0; l < 2; l++) if (in_hit(&hit_lang[l], x, y)) lang = l;
            if (in_hit(&hit_larger, x, y)) larger_text ^= 1;
            if (in_hit(&hit_motion, x, y)) reduce_motion ^= 1;
        }
        if (id == APP_SPACES) {
            int tw = (ww - 60) / 2, th = (wh - TITLE_H - 60) / 2;
            for (int sp = 0; sp < NSPACES; sp++) {
                int sx = wx + 20 + (sp % 2) * (tw + 20), sy = wy + TITLE_H + 14 + (sp / 2) * (th + 20);
                if (x >= sx && x < sx + tw && y >= sy && y < sy + th && sp != cur_space) { win[APP_SPACES].space = sp; switch_space(sp - cur_space); raise_win(APP_SPACES); }
            }
        }
        return;
    }
}

static void type_char(char c) {
    if (welcome_open) { if (c == '\n') finish_welcome(); return; }
    if (glass_kind == 2 && ctrl && c == '\n') { glass_click(glass_primary.x + 5, glass_primary.y + 5); return; }
    if ((glass_kind == 4 || glass_kind == 5) && c == '\n') { glass_release(); return; }
    if (glass_type(c)) return;
    if (focused < 0 || !visible(focused)) { glass_open(1); glass_type(c); return; }
    if (focused == APP_NOTES) {
        if (c == '\b') { if (note_len) note_len--; }
        else if (note_len < (int)sizeof note_buf - 1) note_buf[note_len++] = c;
        note_buf[note_len] = 0;
        return;
    }
    if (focused == APP_FIND) {
        if (c == '\b') { if (find_len) find_q[--find_len] = 0; }
        else if (c >= 32 && find_len < 38) { find_q[find_len++] = c; find_q[find_len] = 0; }
        return;
    }
    if (focused != APP_VELA) open_app(APP_VELA);
    if (c == '\n') { input[input_len] = 0; run_command(input); input_len = 0; input[0] = 0; }
    else if (c == '\b') { if (input_len) input[--input_len] = 0; }
    else if (c >= 32 && c < 127 && input_len < TCOLS - 4) { input[input_len++] = c; input[input_len] = 0; }
}

static int mpk_i; static uint8_t mpk[3];
static int poll_input(void) {
    int dirty = 0;
    for (int n = 0; n < 256; n++) {
        uint8_t st = inb(0x64);
        if (!(st & 1)) break;
        uint8_t v = inb(0x60);
        if (st & 0x20) {
            if (mpk_i == 0 && !(v & 0x08)) continue;
            mpk[mpk_i++] = v;
            if (mpk_i < 3) continue;
            mpk_i = 0;
            int dx = (int)mpk[1] - ((mpk[0] << 4) & 0x100), dy = (int)mpk[2] - ((mpk[0] << 3) & 0x100);
            mx += dx; my -= dy;
            if (mx < 0) mx = 0;
            if (my < 0) my = 0;
            if (mx >= W) mx = W - 1;
            if (my >= H) my = H - 1;
            mbtn = mpk[0] & 1;
            if (mbtn && !prev_btn) mouse_click(mx, my);
            if (!mbtn) dragging = -1;
            if (dragging >= 0) { win[dragging].x = mx - drag_dx; win[dragging].y = my - drag_dy; if (win[dragging].y < BAR_H) win[dragging].y = BAR_H; }
            prev_btn = mbtn;
            hover_dock = dock_hit(mx, my);
            dirty = 1;
            continue;
        }
        if (v == 0xE0) { ext = 1; continue; }
        if (ext) {
            ext = 0;
            if (v == 0x1D) ctrl = 1; else if (v == 0x9D) ctrl = 0;
            else if (v == 0x38) alt = 1; else if (v == 0xB8) alt = 0;
            else if (v == 0x4B && ctrl && alt) { switch_space(-1); dirty = 1; }
            else if (v == 0x4D && ctrl && alt) { switch_space(1); dirty = 1; }
            else if (v == 0x48 && focused == APP_VELA && hist_pos > 0) { hist_pos--; kstrcpy(input, history[hist_pos], TCOLS + 1); input_len = (int)kstrlen(input); dirty = 1; }
            else if (v == 0x50 && focused == APP_VELA && hist_pos < hist_n) { hist_pos++; if (hist_pos == hist_n) input[0] = 0; else kstrcpy(input, history[hist_pos], TCOLS + 1); input_len = (int)kstrlen(input); dirty = 1; }
            continue;
        }
        if (v == 0x2A || v == 0x36) { shift = 1; continue; }
        if (v == 0xAA || v == 0xB6) { shift = 0; continue; }
        if (v == 0x1D) { ctrl = 1; continue; }
        if (v == 0x9D) { ctrl = 0; continue; }
        if (v == 0x38) { alt = 1; continue; }
        if (v == 0xB8) { alt = 0; continue; }
        if (v == 0x3A) { caps ^= 1; continue; }
        if (v == 0x57) { if (!welcome_open) toggle_focus(); dirty = 1; continue; }   /* F11 */
        if (v == 0x01) { if (glass_kind) glass_release(); focus_mode = 0; dirty = 1; continue; }                      /* Esc */
        if (v & 0x80) continue;
        char c = shift ? keymap_shift[v] : keymap[v];
        if (caps && c >= 'a' && c <= 'z' && !shift) c -= 32;
        if (!c || c == 27 || c == '\t') continue;
        type_char(c);
        cursor_on = 1;
        dirty = 1;
    }
    return dirty;
}

/* ---------------------------------------------------------------- boot splash */
static void splash_frame(int pct, const char *msg) {
    memset(backbuf, 0, (size_t)W * H * 4);
    int cx = W / 2, cy = H / 2 - 40;
    for (int r = 120; r > 0; r -= 8) fill_circle(cx, cy - 20, r, 0x2D6877, 6);
    glyph(APP_VELA, cx - 32, cy - 60, 64, 0xFFFFFF);
    int tw = text_width(&font_big, "VelaOS");
    draw_text(&font_big, cx - tw / 2, cy + 24, "VelaOS", 0xFFFFFF);
    fill_rrect(cx - 120, cy + 90, 240, 4, 2, 0x22313C, 255);
    if (pct > 0) fill_rrect(cx - 120, cy + 90, 240 * pct / 100, 4, 2, TEAL_BRIGHT, 255);
    tw = text_width(&font_ui, msg);
    draw_text(&font_ui, cx - tw / 2, cy + 106, msg, 0x8FA3B0);
    blit();
}
static void splash(void) {
    struct rtc t; rtc_read(&t);
    unsigned last = t.sec; int ticks = 0, pct = 0;
    static const char *msgs[] = { "Starting kernel...", "Probing devices...", "Starting compositor...", "Starting desktop..." };
    while (ticks < 4) {
        splash_frame(pct, msgs[ticks]);
        uint64_t t0 = rdtsc(); while (rdtsc() - t0 < 20000000ULL) __asm__ volatile("pause");
        if (pct < ticks * 25 + 24) pct++;
        rtc_read(&t);
        if (t.sec != last) { last = t.sec; ticks++; pct = ticks * 25; }
    }
    splash_frame(100, "Starting desktop...");
}

/* Bochs/QEMU/VirtualBox VBE DISPI: switch the linear framebuffer to the target mode ourselves,
 * so we are not limited to the VESA modes the BIOS lists to GRUB. */
static void dispi_w(uint16_t i, uint16_t v) { outw(0x1CE, i); outw(0x1CF, v); }
static uint16_t dispi_r(uint16_t i) { outw(0x1CE, i); return inw(0x1CF); }
static int parse_res(const char *s, int *w, int *h) {
    const char *r = s;
    while (*r && !kstartswith(r, "res=")) r++;
    if (!*r) return 0;
    r += 4; int a = 0, b = 0;
    while (*r >= '0' && *r <= '9') a = a * 10 + (*r++ - '0');
    if (*r++ != 'x') return 0;
    while (*r >= '0' && *r <= '9') b = b * 10 + (*r++ - '0');
    if (a < 640 || b < 480 || a > MAXW || b > MAXH) return 0;
    *w = a; *h = b; return 1;
}
static void set_mode(void) {
    int tw = 1920, th = 1080;
    parse_res(cmdline, &tw, &th);
    if (kstartswith(cmdline, "nomodeset") || (tw == W && th == H)) return;
    /* handshake: newest interface version the adapter accepts (QEMU std VGA, VirtualBox VBoxVGA/VMSVGA) */
    static const uint16_t ids[] = { 0xB0C5, 0xB0C4, 0xB0C3, 0xB0C2 };
    int ok = 0;
    for (int i = 0; i < 4 && !ok; i++) { dispi_w(0, ids[i]); if (dispi_r(0) == ids[i]) ok = 1; }
    if (!ok) { serial_puts("VelaOS: no DISPI, keeping firmware mode\r\n"); return; }
    dispi_w(4, 0);
    dispi_w(1, (uint16_t)tw); dispi_w(2, (uint16_t)th); dispi_w(3, 32);
    dispi_w(6, (uint16_t)tw); dispi_w(8, 0); dispi_w(9, 0);
    dispi_w(4, 0x41);
    if (dispi_r(1) != tw || dispi_r(2) != th) { serial_puts("VelaOS: DISPI mode rejected\r\n"); return; }
    W = tw; H = th; fb_pitch = (uint32_t)tw * 4; fb_bpp = 32;
    serial_puts("VelaOS: display mode set via DISPI\r\n");
}

void kmain(uint32_t magic, uint64_t info) {
    serial_init();
    serial_puts("VelaOS: kernel entered long mode\r\n");
    if (magic != 0x36D76289) { serial_puts("VelaOS: bad multiboot2 magic\r\n"); return; }
    parse_multiboot(info);
    if (!fb || W <= 0 || H <= 0 || W > MAXW || H > MAXH || (fb_bpp != 32 && fb_bpp != 24)) {
        serial_puts("VelaOS: no usable framebuffer\r\n");
        volatile uint16_t *vga = (uint16_t *)0xB8000; const char *m = "VelaOS: no graphics framebuffer (need 1024x768x32 or larger)";
        for (int i = 0; m[i]; i++) vga[i] = (uint16_t)(0x1F00 | m[i]);
        return;
    }
    set_mode();
    read_cpu();
    tsc_boot = rdtsc();
    struct rtc t; rtc_read(&t); boot_secs = rtc_secs_of_day(&t); uptime_base_set = 1;
    serial_puts("VelaOS: framebuffer ok, starting compositor\r\n");
    splash();
    make_wallpaper();
    mouse_init();
    mx = W / 2; my = H * 3 / 4;

    int vw = W * 55 / 100, vh = H * 60 / 100;
    win[APP_VELA]     = (struct window){ W * 6 / 100, BAR_H + 40, vw, vh, 0, 0, 0, 0, 0, 0, 0, 0 };
    win[APP_SETTINGS] = (struct window){ W - 470 - W * 4 / 100, BAR_H + 130, 470, 470, 0, 0, 0, 0, 0, 0, 0, 0 };
    win[APP_FIND]     = (struct window){ W * 18 / 100, BAR_H + 70, 520, 420, 0, 0, 0, 0, 0, 0, 0, 0 };
    win[APP_SPACES]   = (struct window){ W * 22 / 100, BAR_H + 90, 560, 420, 0, 0, 0, 0, 0, 0, 0, 0 };
    win[APP_NOTES]    = (struct window){ W * 14 / 100, BAR_H + 60, 620, 420, 0, 0, 0, 0, 0, 0, 0, 0 };
    win[APP_MEDIA]    = (struct window){ W * 30 / 100, BAR_H + 80, 520, 400, 0, 0, 0, 0, 0, 0, 0, 0 };
    if (H < 800) win[APP_SETTINGS].h = H - BAR_H - 150, win[APP_SETTINGS].y = BAR_H + 70;

    tprint("VelaOS " VELA_VERSION " - x86_64", 1);
    tprint("Type 'help' for commands. Try 'neofetch'.", 2);
    tprint("", 0);
    serial_puts("VelaOS: welcome screen\r\n");

    render();
    uint64_t blink = rdtsc();
    for (;;) {
        int dirty = poll_input();
        uint64_t now = rdtsc();
        if (now - blink > 1500000000ULL) { blink = now; cursor_on ^= 1; dirty = 1; }
        if (dirty) render();
        else __asm__ volatile("pause");
    }
}
