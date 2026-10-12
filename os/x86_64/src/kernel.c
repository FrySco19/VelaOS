/*
 * VelaOS x86_64 kernel (v0.1 "first light").
 * Boots via GRUB Multiboot2, takes a linear framebuffer and draws the Vela desktop:
 * top bar with clock, dock, movable windows, a PS/2 mouse pointer and an interactive shell.
 * No interrupts yet: keyboard, mouse and RTC are polled.
 */
#include <stdint.h>
#include <stddef.h>
#include "font.h"

#define VELA_VERSION "0.1.0-dev"

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
static void shadow(int x, int y, int w, int h, int r) {
    for (int s = 14; s > 0; s -= 2) fill_rrect(x - s + 2, y - s + 8, w + 2 * s - 4, h + 2 * s - 4, r + s, 0x000000, 10);
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
static char cpu_vendor[13], cpu_brand[49];
static uint64_t tsc_boot;

struct mb2_tag { uint32_t type, size; };
static void parse_multiboot(uint64_t info) {
    uint8_t *p = (uint8_t *)info + 8, *end = (uint8_t *)info + *(uint32_t *)info;
    while (p < end) {
        struct mb2_tag *t = (struct mb2_tag *)p;
        if (t->type == 0) break;
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

/* ---------------------------------------------------------------- terminal model */
#define TCOLS 78
#define TROWS 200
static char tlines[TROWS][TCOLS + 1];
static uint32_t tcolor[TROWS];
static int tcount;
static char input[TCOLS + 1]; static int input_len;
static char history[16][TCOLS + 1]; static int hist_n, hist_pos;
static uint32_t accent = 0x4FC3F7;

static void tnewline(uint32_t c) {
    if (tcount == TROWS) { memmove(tlines[0], tlines[1], sizeof(tlines[0]) * (TROWS - 1)); memmove(tcolor, tcolor + 1, sizeof(uint32_t) * (TROWS - 1)); tcount--; }
    tlines[tcount][0] = 0; tcolor[tcount] = c; tcount++;
}
static void tprint(const char *s, uint32_t c) {
    tnewline(c);
    int col = 0;
    for (; *s; s++) {
        if (*s == '\n' || col == TCOLS) { tnewline(c); col = 0; if (*s == '\n') continue; }
        tlines[tcount - 1][col++] = *s; tlines[tcount - 1][col] = 0;
    }
}
static void tclear(void) { tcount = 0; }

/* ---------------------------------------------------------------- windows */
enum { WIN_TERM, WIN_SYS, WIN_ABOUT, WIN_FILES, NWIN };
struct window { int x, y, w, h, open; const char *title; };
static struct window win[NWIN];
static int zorder[NWIN] = { WIN_FILES, WIN_ABOUT, WIN_SYS, WIN_TERM }; /* last = top */
static int focused = WIN_TERM;

static void raise_win(int id) {
    int k = 0;
    for (int i = 0; i < NWIN; i++) if (zorder[i] != id) zorder[k++] = zorder[i];
    zorder[k] = id; focused = id;
}

static const char *fake_files[][2] = {
    { "README.txt", "Welcome to VelaOS. Type 'help' in the shell to explore." },
    { "motd", "First light: VelaOS boots on bare (virtual) metal." },
    { "roadmap.txt", "next: interrupts, scheduler, storage, Glass UI in-guest, seL4 executor bridge." },
    { "license.txt", "VelaOS - see repository LICENSE." },
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
static void fmt_date(char *out, int with_secs) {
    static const char *mon[] = { "Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec" };
    struct rtc t; rtc_read(&t);
    char b[8]; out[0] = 0;
    utoa(t.day, b); kstrcat(out, b, 40); kstrcat(out, " ", 40);
    kstrcat(out, mon[(t.mon - 1) % 12], 40); kstrcat(out, " ", 40);
    utoa(t.year, b); kstrcat(out, b, 40); kstrcat(out, "  ", 40);
    utoa2(t.hour, b); kstrcat(out, b, 40); kstrcat(out, ":", 40);
    utoa2(t.min, b); kstrcat(out, b, 40);
    if (with_secs) { kstrcat(out, ":", 40); utoa2(t.sec, b); kstrcat(out, b, 40); }
}

/* ---------------------------------------------------------------- power */
static void reboot(void) {
    for (int i = 0; i < 100000 && (inb(0x64) & 2); i++) {}
    outb(0x64, 0xFE);
    for (;;) __asm__ volatile("hlt");
}
static void shutdown(void) {
    outw(0x4004, 0x3400); /* VirtualBox ACPI PM1a */
    outw(0x604, 0x2000);  /* QEMU */
    outw(0xB004, 0x2000); /* Bochs / older QEMU */
}

/* ---------------------------------------------------------------- shell */
static void cmd_neofetch(void) {
    char l[96], b[24];
    tprint("      /\\        VelaOS " VELA_VERSION, accent);
    tprint("     /  \\       ------------------", accent);
    kstrcpy(l, "    / /| \\      kernel : vela-x86_64 (multiboot2)", sizeof l); tprint(l, accent);
    kstrcpy(l, "   / / |  \\     cpu    : ", sizeof l); kstrcat(l, cpu_brand, sizeof l); tprint(l, accent);
    utoa(mem_kb / 1024, b);
    kstrcpy(l, "  /_/__|___\\    memory : ", sizeof l); kstrcat(l, b, sizeof l); kstrcat(l, " MiB", sizeof l); tprint(l, accent);
    utoa((uint64_t)W, b); kstrcpy(l, "  \\________/    display: ", sizeof l); kstrcat(l, b, sizeof l); kstrcat(l, "x", sizeof l);
    utoa((uint64_t)H, b); kstrcat(l, b, sizeof l); tprint(l, accent);
    kstrcpy(l, "   ~~~~~~~~     boot   : ", sizeof l); kstrcat(l, boot_loader, sizeof l); tprint(l, accent);
    fmt_uptime(b); kstrcpy(l, "                uptime : ", sizeof l); kstrcat(l, b, sizeof l); tprint(l, accent);
}

static void run_command(const char *cmd) {
    char line[TCOLS + 8];
    kstrcpy(line, "vela@velaos:~$ ", sizeof line); kstrcat(line, cmd, sizeof line);
    tprint(line, 0xE6EDF3);
    while (*cmd == ' ') cmd++;
    if (!*cmd) return;
    if (hist_n < 16) kstrcpy(history[hist_n++], cmd, TCOLS + 1);
    else { memmove(history[0], history[1], sizeof(history[0]) * 15); kstrcpy(history[15], cmd, TCOLS + 1); }
    hist_pos = hist_n;

    char b[64];
    if (kstreq(cmd, "help")) {
        tprint("Built-in commands:", 0x9CDCFE);
        tprint("  help      this list            neofetch  system summary", 0xC9D1D9);
        tprint("  about     about VelaOS         uname     kernel name", 0xC9D1D9);
        tprint("  mem       memory               cpu       processor", 0xC9D1D9);
        tprint("  date      RTC clock            uptime    time since boot", 0xC9D1D9);
        tprint("  ls        list files           cat FILE  show a file", 0xC9D1D9);
        tprint("  echo ...  print text           clear     clear the screen", 0xC9D1D9);
        tprint("  open APP  terminal|system|about|files    theme  change accent", 0xC9D1D9);
        tprint("  reboot    restart              shutdown  power off the VM", 0xC9D1D9);
        tprint("Mouse: click dock icons, drag windows by their title bar.", 0x8B949E);
    } else if (kstreq(cmd, "about")) {
        tprint("VelaOS " VELA_VERSION " - first light build.", accent);
        tprint("A tiny x86_64 kernel booted by GRUB that owns the screen, keyboard", 0xC9D1D9);
        tprint("and mouse directly. No Linux, no browser underneath.", 0xC9D1D9);
    } else if (kstreq(cmd, "uname")) tprint("VelaOS", 0xC9D1D9);
    else if (kstreq(cmd, "uname -a")) tprint("VelaOS velaos " VELA_VERSION " x86_64 multiboot2", 0xC9D1D9);
    else if (kstreq(cmd, "mem")) {
        utoa(mem_kb / 1024, b); char l[64] = "Usable memory: "; kstrcat(l, b, 64); kstrcat(l, " MiB", 64); tprint(l, 0xC9D1D9);
    } else if (kstreq(cmd, "cpu")) {
        char l[96] = "CPU: "; kstrcat(l, cpu_brand, 96); tprint(l, 0xC9D1D9);
        kstrcpy(l, "Vendor: ", 96); kstrcat(l, cpu_vendor, 96); tprint(l, 0xC9D1D9);
    } else if (kstreq(cmd, "date")) { fmt_date(b, 1); tprint(b, 0xC9D1D9); }
    else if (kstreq(cmd, "uptime")) { fmt_uptime(b); tprint(b, 0xC9D1D9); }
    else if (kstreq(cmd, "neofetch")) cmd_neofetch();
    else if (kstreq(cmd, "clear")) tclear();
    else if (kstreq(cmd, "ls")) { for (int i = 0; i < NFILES; i++) tprint(fake_files[i][0], 0x7EE787); }
    else if (kstartswith(cmd, "cat ")) {
        const char *f = cmd + 4; int ok = 0;
        for (int i = 0; i < NFILES; i++) if (kstreq(f, fake_files[i][0])) { tprint(fake_files[i][1], 0xC9D1D9); ok = 1; }
        if (!ok) tprint("cat: no such file", 0xFF7B72);
    } else if (kstartswith(cmd, "echo")) tprint(cmd[4] ? cmd + 5 : "", 0xC9D1D9);
    else if (kstartswith(cmd, "open ")) {
        const char *a = cmd + 5; int id = -1;
        if (kstreq(a, "terminal")) id = WIN_TERM; else if (kstreq(a, "system")) id = WIN_SYS;
        else if (kstreq(a, "about")) id = WIN_ABOUT; else if (kstreq(a, "files")) id = WIN_FILES;
        if (id < 0) tprint("open: unknown app", 0xFF7B72); else { win[id].open = 1; raise_win(id); if (id != WIN_TERM) raise_win(WIN_TERM); }
    } else if (kstreq(cmd, "theme")) {
        static const uint32_t th[] = { 0x4FC3F7, 0xF78C6C, 0xC3E88D, 0xC792EA };
        static int ti; ti = (ti + 1) % 4; accent = th[ti];
        tprint("Accent colour changed.", accent);
    } else if (kstreq(cmd, "reboot")) { tprint("Rebooting...", 0xFFA657); reboot(); }
    else if (kstreq(cmd, "shutdown") || kstreq(cmd, "poweroff")) { tprint("Powering off...", 0xFFA657); shutdown(); tprint("ACPI power-off not available here; close the VM window.", 0xFF7B72); }
    else { char l[96]; kstrcpy(l, cmd, 64); kstrcat(l, ": command not found (try 'help')", 96); tprint(l, 0xFF7B72); }
}

/* ---------------------------------------------------------------- drawing */
static void make_wallpaper(void) {
    for (int y = 0; y < H; y++) for (int x = 0; x < W; x++) {
        /* deep night-sea gradient */
        int t = y * 255 / H, s = x * 255 / W;
        int r = 10 + t / 10 + s / 20, g = 22 + t / 5 + s / 16, b = 48 + t / 3 + s / 12;
        wallpaper[y * W + x] = (uint32_t)(r << 16 | g << 8 | b);
    }
    memcpy(backbuf, wallpaper, (size_t)W * H * 4);
    /* soft glow */
    for (int rr = 420; rr > 0; rr -= 12) fill_circle(W * 3 / 4, H / 3, rr, 0x3A7BD5, 4);
    /* stylised sail (vela): mainsail + jib on a mast, over a hull */
    int cx = W * 3 / 4 - 40, base = H - 200, top = H / 3 - 130;
    for (int y = top; y < base; y++) {
        int k = (y - top) * 190 / (base - top);
        fill_rect(cx + 5, y, k, 1, 0xBFE6FF, 22 + (y - top) * 30 / (base - top));
        if (y > top + 40) fill_rect(cx - 4 - (y - top - 40) * 110 / (base - top), y, (y - top - 40) * 110 / (base - top), 1, 0x7FB8E8, 18);
    }
    fill_rect(cx, top - 14, 3, base - top + 26, 0xD8EEFF, 80);
    for (int i = 0; i < 18; i++) fill_rect(cx - 150 + i * 4, base + 12 + i, 330 - i * 8, 1, 0x9AD0FF, 45);
    memcpy(wallpaper, backbuf, (size_t)W * H * 4);
}

static void draw_topbar(void) {
    fill_rect(0, 0, W, 30, 0x0B1220, 200);
    fill_rect(0, 30, W, 1, 0xFFFFFF, 25);
    /* logo: small sail */
    for (int j = 0; j < 16; j++) fill_rect(14 + (16 - j) / 3, 7 + j, j * 2 / 3 + 1, 1, accent, 255);
    draw_text(&font_bold, 34, 6, "VelaOS", 0xFFFFFF);
    int x = 110;
    const char *menus[] = { "Activities", "Terminal", "Files", "Help" };
    for (int i = 0; i < 4; i++) x += draw_text(&font_ui, x, 6, menus[i], 0xC9D1D9) + 22;
    char d[40]; fmt_date(d, 0);
    int tw = text_width(&font_bold, d);
    draw_text(&font_bold, W - tw - 16, 6, d, 0xFFFFFF);
    /* status icons: net, volume, battery */
    int sx = W - tw - 90;
    for (int i = 0; i < 4; i++) fill_rect(sx + i * 5, 20 - i * 3, 3, 3 + i * 3, 0xC9D1D9, 255);
    fill_rrect(sx + 30, 11, 22, 11, 3, 0xC9D1D9, 255); fill_rect(sx + 52, 14, 2, 5, 0xC9D1D9, 255);
    fill_rrect(sx + 32, 13, 15, 7, 2, 0x7EE787, 255);
}

#define DOCK_N 5
static const char *dock_names[DOCK_N] = { "Terminal", "Files", "System", "About", "Power" };
static const uint32_t dock_cols[DOCK_N] = { 0x1F2937, 0xE3A33A, 0x3A86E3, 0x8E5BD9, 0xD9534F };
static int dock_x0, dock_y0, hover_dock = -1;
#define DOCK_ICON 52
#define DOCK_GAP 14

static void draw_dock(void) {
    int dw = DOCK_N * DOCK_ICON + (DOCK_N + 1) * DOCK_GAP, dh = DOCK_ICON + 22;
    dock_x0 = (W - dw) / 2; dock_y0 = H - dh - 12;
    fill_rrect(dock_x0, dock_y0, dw, dh, 18, 0xFFFFFF, 40);
    fill_rrect(dock_x0 + 1, dock_y0 + 1, dw - 2, dh - 2, 17, 0x0B1220, 120);
    for (int i = 0; i < DOCK_N; i++) {
        int ix = dock_x0 + DOCK_GAP + i * (DOCK_ICON + DOCK_GAP), iy = dock_y0 + 9 - (hover_dock == i ? 6 : 0);
        fill_rrect(ix, iy, DOCK_ICON, DOCK_ICON, 13, dock_cols[i], 255);
        fill_rrect(ix, iy, DOCK_ICON, DOCK_ICON / 2, 13, 0xFFFFFF, 25);
        if (i == 0) { draw_text(&font_mono, ix + 11, iy + 14, ">_", accent); }
        if (i == 1) { fill_rrect(ix + 10, iy + 15, 32, 24, 4, 0xFFF3D6, 255); fill_rrect(ix + 10, iy + 12, 14, 6, 2, 0xFFF3D6, 255); }
        if (i == 2) { fill_circle(ix + 26, iy + 26, 14, 0xFFFFFF, 255); fill_circle(ix + 26, iy + 26, 7, dock_cols[2], 255); }
        if (i == 3) { fill_circle(ix + 26, iy + 26, 15, 0xFFFFFF, 255); draw_text(&font_bold, ix + 23, iy + 16, "i", dock_cols[3]); }
        if (i == 4) { fill_circle(ix + 26, iy + 28, 13, 0xFFFFFF, 255); fill_circle(ix + 26, iy + 28, 9, dock_cols[4], 255); fill_rect(ix + 23, iy + 10, 6, 18, dock_cols[4], 255); fill_rect(ix + 25, iy + 12, 2, 16, 0xFFFFFF, 255); }
        int open = (i == 0 && win[WIN_TERM].open) || (i == 1 && win[WIN_FILES].open) || (i == 2 && win[WIN_SYS].open) || (i == 3 && win[WIN_ABOUT].open);
        if (open) fill_circle(ix + DOCK_ICON / 2, dock_y0 + dh - 6, 2, 0xFFFFFF, 230);
        if (hover_dock == i) {
            int tw = text_width(&font_ui, dock_names[i]);
            fill_rrect(ix + DOCK_ICON / 2 - tw / 2 - 8, dock_y0 - 34, tw + 16, 24, 8, 0x0B1220, 220);
            draw_text(&font_ui, ix + DOCK_ICON / 2 - tw / 2, dock_y0 - 31, dock_names[i], 0xFFFFFF);
        }
    }
}

static void draw_window_frame(struct window *w, int focus) {
    shadow(w->x, w->y, w->w, w->h, 12);
    fill_rrect(w->x, w->y, w->w, w->h, 12, 0x30363D, 255);
    fill_rrect(w->x + 1, w->y + 1, w->w - 2, w->h - 2, 11, 0x0D1117, 250);
    fill_rrect(w->x + 1, w->y + 1, w->w - 2, 34, 11, focus ? 0x1C2430 : 0x161B22, 255);
    fill_rect(w->x + 1, w->y + 20, w->w - 2, 15, focus ? 0x1C2430 : 0x161B22, 255);
    fill_rect(w->x + 1, w->y + 35, w->w - 2, 1, 0x30363D, 255);
    fill_circle(w->x + 18, w->y + 18, 6, focus ? 0xFF5F57 : 0x484F58, 255);
    fill_circle(w->x + 38, w->y + 18, 6, focus ? 0xFEBC2E : 0x484F58, 255);
    fill_circle(w->x + 58, w->y + 18, 6, focus ? 0x28C840 : 0x484F58, 255);
    int tw = text_width(&font_bold, w->title);
    draw_text(&font_bold, w->x + (w->w - tw) / 2, w->y + 9, w->title, focus ? 0xE6EDF3 : 0x8B949E);
}

static int cursor_on = 1;
static void draw_term(struct window *w) {
    int lh = 19, top = w->y + 44, rows = (w->h - 52) / lh;
    int start = tcount + 1 > rows ? tcount + 1 - rows : 0;
    int y = top;
    for (int i = start; i < tcount; i++, y += lh) draw_text(&font_mono, w->x + 14, y - 2, tlines[i], tcolor[i]);
    int x = w->x + 14;
    x += draw_text(&font_mono, x, y - 2, "vela@velaos", 0x7EE787);
    x += draw_text(&font_mono, x, y - 2, ":", 0xE6EDF3);
    x += draw_text(&font_mono, x, y - 2, "~", accent);
    x += draw_text(&font_mono, x, y - 2, "$ ", 0xE6EDF3);
    x += draw_text(&font_mono, x, y - 2, input, 0xE6EDF3);
    if (cursor_on && focused == WIN_TERM) fill_rect(x + 1, y, 8, 17, accent, 200);
}
static void kv(int x, int y, const char *k, const char *v) {
    draw_text(&font_ui, x, y, k, 0x8B949E);
    draw_text(&font_bold, x + 92, y, v, 0xE6EDF3);
}
static void draw_sys(struct window *w) {
    char b[64], c[24];
    int x = w->x + 18, y = w->y + 50;
    kv(x, y, "OS", "VelaOS " VELA_VERSION); y += 24;
    kv(x, y, "Kernel", "vela-x86_64"); y += 24;
    char cb[26]; kstrcpy(cb, cpu_brand, 26); kv(x, y, "CPU", cb); y += 24;
    utoa(mem_kb / 1024, c); kstrcpy(b, c, 64); kstrcat(b, " MiB", 64); kv(x, y, "Memory", b); y += 24;
    utoa((uint64_t)W, c); kstrcpy(b, c, 64); kstrcat(b, " x ", 64); utoa((uint64_t)H, c); kstrcat(b, c, 64); kv(x, y, "Display", b); y += 24;
    fmt_uptime(b); kv(x, y, "Uptime", b); y += 30;
    /* memory bar */
    draw_text(&font_ui, x, y, "Kernel footprint", 0x8B949E); y += 22;
    fill_rrect(x, y, w->w - 36, 10, 5, 0x30363D, 255);
    fill_rrect(x, y, (w->w - 36) / 12 + 8, 10, 5, accent, 255);
}
static void draw_about(struct window *w) {
    int cx = w->x + w->w / 2;
    for (int j = 0; j < 44; j++) fill_rect(cx - 12 + (44 - j) / 3, w->y + 52 + j, j * 2 / 3 + 1, 1, accent, 255);
    int tw = text_width(&font_big, "VelaOS");
    draw_text(&font_big, cx - tw / 2, w->y + 100, "VelaOS", 0xFFFFFF);
    const char *l[] = { "Version " VELA_VERSION " - first light", "Native x86_64 kernel, no host browser.", "Glass UI and the seL4 executor are next." };
    for (int i = 0; i < 3; i++) { tw = text_width(&font_ui, l[i]); draw_text(&font_ui, cx - tw / 2, w->y + 150 + i * 22, l[i], i ? 0x8B949E : 0xE6EDF3); }
}
static void draw_files(struct window *w) {
    int x = w->x + 18, y = w->y + 50;
    fill_rrect(w->x + 10, w->y + 44, 110, w->h - 54, 8, 0x161B22, 255);
    const char *side[] = { "Home", "Documents", "System" };
    for (int i = 0; i < 3; i++) draw_text(&font_ui, x, y + 4 + i * 24, side[i], i == 0 ? accent : 0xC9D1D9);
    for (int i = 0; i < NFILES; i++) {
        int fx = w->x + 140 + (i % 3) * 100, fy = w->y + 52 + (i / 3) * 90;
        fill_rrect(fx + 20, fy, 40, 50, 5, 0xE6EDF3, 255);
        fill_rect(fx + 27, fy + 14, 26, 2, 0x8B949E, 255); fill_rect(fx + 27, fy + 22, 26, 2, 0x8B949E, 255); fill_rect(fx + 27, fy + 30, 18, 2, 0x8B949E, 255);
        int tw = text_width(&font_ui, fake_files[i][0]);
        draw_text(&font_ui, fx + 40 - tw / 2, fy + 56, fake_files[i][0], 0xC9D1D9);
    }
}

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
    draw_topbar();
    for (int k = 0; k < NWIN; k++) {
        int id = zorder[k]; struct window *w = &win[id];
        if (!w->open) continue;
        draw_window_frame(w, id == focused);
        if (id == WIN_TERM) draw_term(w);
        if (id == WIN_SYS) draw_sys(w);
        if (id == WIN_ABOUT) draw_about(w);
        if (id == WIN_FILES) draw_files(w);
    }
    draw_dock();
    draw_cursor();
    blit();
}

/* ---------------------------------------------------------------- input */
static const char keymap[128] = {
    0, 27, '1','2','3','4','5','6','7','8','9','0','-','=','\b','\t','q','w','e','r','t','y','u','i','o','p','[',']','\n',
    0, 'a','s','d','f','g','h','j','k','l',';','\'','`', 0,'\\','z','x','c','v','b','n','m',',','.','/', 0,'*', 0,' ' };
static const char keymap_shift[128] = {
    0, 27, '!','@','#','$','%','^','&','*','(',')','_','+','\b','\t','Q','W','E','R','T','Y','U','I','O','P','{','}','\n',
    0, 'A','S','D','F','G','H','J','K','L',':','"','~', 0,'|','Z','X','C','V','B','N','M','<','>','?', 0,'*', 0,' ' };
static int shift, caps, ext;

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

static int dock_hit(int x, int y) {
    for (int i = 0; i < DOCK_N; i++) {
        int ix = dock_x0 + DOCK_GAP + i * (DOCK_ICON + DOCK_GAP), iy = dock_y0 + 9;
        if (x >= ix && x < ix + DOCK_ICON && y >= iy - 6 && y < iy + DOCK_ICON) return i;
    }
    return -1;
}
static void dock_click(int i) {
    static const int map[] = { WIN_TERM, WIN_FILES, WIN_SYS, WIN_ABOUT };
    if (i == 4) { tprint("Power: type 'shutdown' or 'reboot' in the terminal.", 0xFFA657); win[WIN_TERM].open = 1; raise_win(WIN_TERM); return; }
    int id = map[i];
    if (win[id].open && focused == id) win[id].open = 0;
    else { win[id].open = 1; raise_win(id); }
}
static void mouse_click(int x, int y) {
    int d = dock_hit(x, y);
    if (d >= 0) { dock_click(d); return; }
    for (int k = NWIN - 1; k >= 0; k--) {
        int id = zorder[k]; struct window *w = &win[id];
        if (!w->open || x < w->x || x >= w->x + w->w || y < w->y || y >= w->y + w->h) continue;
        raise_win(id);
        if (y < w->y + 36) {
            if (x >= w->x + 10 && x < w->x + 26) { w->open = 0; return; }   /* close button */
            dragging = id; drag_dx = x - w->x; drag_dy = y - w->y;
        }
        return;
    }
}

static int mpk_i; static uint8_t mpk[3];
/* returns 1 if the screen needs a redraw */
static int poll_input(void) {
    int dirty = 0;
    for (int n = 0; n < 64; n++) {
        uint8_t st = inb(0x64);
        if (!(st & 1)) break;
        uint8_t v = inb(0x60);
        if (st & 0x20) { /* mouse byte */
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
            if (dragging >= 0) { win[dragging].x = mx - drag_dx; win[dragging].y = my - drag_dy; if (win[dragging].y < 31) win[dragging].y = 31; }
            prev_btn = mbtn;
            hover_dock = dock_hit(mx, my);
            dirty = 1;
            continue;
        }
        if (v == 0xE0) { ext = 1; continue; }
        if (ext) {
            ext = 0;
            if (v == 0x48 && hist_pos > 0) { hist_pos--; kstrcpy(input, history[hist_pos], TCOLS + 1); input_len = (int)kstrlen(input); dirty = 1; }
            if (v == 0x50 && hist_pos < hist_n) { hist_pos++; if (hist_pos == hist_n) input[0] = 0; else kstrcpy(input, history[hist_pos], TCOLS + 1); input_len = (int)kstrlen(input); dirty = 1; }
            continue;
        }
        if (v == 0x2A || v == 0x36) { shift = 1; continue; }
        if (v == 0xAA || v == 0xB6) { shift = 0; continue; }
        if (v == 0x3A) { caps ^= 1; continue; }
        if (v & 0x80) continue;
        if (focused != WIN_TERM || !win[WIN_TERM].open) { win[WIN_TERM].open = 1; raise_win(WIN_TERM); }
        char c = shift ? keymap_shift[v] : keymap[v];
        if (caps && c >= 'a' && c <= 'z' && !shift) c -= 32;
        if (!c) continue;
        if (c == '\n') { input[input_len] = 0; run_command(input); input_len = 0; input[0] = 0; }
        else if (c == '\b') { if (input_len) input[--input_len] = 0; }
        else if (c >= 32 && c < 127 && input_len < TCOLS - 20) { input[input_len++] = c; input[input_len] = 0; }
        cursor_on = 1;
        dirty = 1;
    }
    return dirty;
}

/* ---------------------------------------------------------------- boot splash */
static void splash_frame(int pct, const char *msg) {
    memset(backbuf, 0, (size_t)W * H * 4);
    int cx = W / 2, cy = H / 2 - 40;
    for (int j = 0; j < 70; j++) fill_rect(cx - 20 + (70 - j) / 3, cy - 60 + j, j * 2 / 3 + 1, 1, accent, 255);
    int tw = text_width(&font_big, "VelaOS");
    draw_text(&font_big, cx - tw / 2, cy + 24, "VelaOS", 0xFFFFFF);
    fill_rrect(cx - 120, cy + 90, 240, 6, 3, 0x30363D, 255);
    if (pct > 0) fill_rrect(cx - 120, cy + 90, 240 * pct / 100, 6, 3, accent, 255);
    tw = text_width(&font_ui, msg);
    draw_text(&font_ui, cx - tw / 2, cy + 108, msg, 0x8B949E);
    blit();
}
/* boot splash: lasts ~4 RTC seconds regardless of CPU speed */
static void splash(void) {
    struct rtc t; rtc_read(&t);
    unsigned last = t.sec; int ticks = 0, pct = 0;
    static const char *msgs[] = { "Starting kernel...", "Probing devices...", "Mounting system...", "Starting desktop..." };
    while (ticks < 4) {
        splash_frame(pct, msgs[ticks]);
        uint64_t t0 = rdtsc(); while (rdtsc() - t0 < 20000000ULL) __asm__ volatile("pause");
        if (pct < ticks * 25 + 24) pct++;
        rtc_read(&t);
        if (t.sec != last) { last = t.sec; ticks++; pct = ticks * 25; }
    }
    splash_frame(100, "Starting desktop...");
}

void kmain(uint32_t magic, uint64_t info) {
    serial_init();
    serial_puts("VelaOS: kernel entered long mode\r\n");
    if (magic != 0x36D76289) { serial_puts("VelaOS: bad multiboot2 magic\r\n"); return; }
    parse_multiboot(info);
    if (!fb || W <= 0 || H <= 0 || W > MAXW || H > MAXH || (fb_bpp != 32 && fb_bpp != 24)) {
        serial_puts("VelaOS: no usable framebuffer\r\n");
        /* text-mode fallback message */
        volatile uint16_t *vga = (uint16_t *)0xB8000; const char *m = "VelaOS: no graphics framebuffer (need 1024x768x32)";
        for (int i = 0; m[i]; i++) vga[i] = (uint16_t)(0x1F00 | m[i]);
        return;
    }
    read_cpu();
    tsc_boot = rdtsc();
    struct rtc t; rtc_read(&t); boot_secs = rtc_secs_of_day(&t); uptime_base_set = 1;
    serial_puts("VelaOS: framebuffer ok, starting desktop\r\n");

    splash();
    make_wallpaper();
    mouse_init();
    mx = W / 2; my = H / 2;

    win[WIN_TERM] = (struct window){ 70, 70, 720, 470, 1, "Terminal - vela shell" };
    win[WIN_SYS] = (struct window){ W - 380, 90, 340, 270, 1, "System" };
    win[WIN_ABOUT] = (struct window){ W - 420, 300, 380, 240, 0, "About VelaOS" };
    win[WIN_FILES] = (struct window){ 160, 140, 560, 300, 0, "Files" };
    if (W < 1100) { win[WIN_TERM].x = 30; win[WIN_TERM].w = 600; win[WIN_SYS].x = W - 360; win[WIN_SYS].y = 120; }

    tprint("Welcome to VelaOS " VELA_VERSION " (x86_64)", accent);
    tprint("Type 'help' for commands. Try 'neofetch'.", 0x8B949E);
    tprint("", 0);
    serial_puts("VelaOS: desktop ready\r\n");

    struct rtc last; rtc_read(&last);
    render();
    uint64_t blink = rdtsc();
    for (;;) {
        int dirty = poll_input();
        uint64_t now = rdtsc();
        if (now - blink > 1200000000ULL) { blink = now; cursor_on ^= 1; dirty = 1; }
        if (dirty) render();
        else __asm__ volatile("pause");
    }
}
