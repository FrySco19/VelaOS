/* Linux guest backend: draws the shell into /dev/fb0 (DRM fbdev emulation or
 * vesafb) and reads pointer/keyboard from /dev/input/event*. Runs as the only
 * graphical process in the VelaOS guest; there is no browser and no HTTP. */
#include "vs.h"
#include <errno.h>
#include <fcntl.h>
#include <linux/fb.h>
#include <linux/input.h>
#include <linux/kd.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <dirent.h>

#define MAX_IN 16
typedef struct { int fd, absx_min, absx_max, absy_min, absy_max, has_abs; } input_dev;

static long mem_available_kb(void) {
    FILE *f = fopen("/proc/meminfo", "r"); if (!f) return -1;
    char line[256]; long total = -1, avail = -1;
    while (fgets(line, sizeof line, f)) {
        sscanf(line, "MemTotal: %ld kB", &total);
        sscanf(line, "MemAvailable: %ld kB", &avail);
    }
    fclose(f);
    return total >= 0 && avail >= 0 ? total - avail : -1;
}

int main(int argc, char **argv) {
    const char *fbpath = "/dev/fb0", *fonts = "/usr/share/vela/fonts";
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--fb") && i + 1 < argc) fbpath = argv[++i];
        else if (!strcmp(argv[i], "--fonts") && i + 1 < argc) fonts = argv[++i];
    }
    int fb = open(fbpath, O_RDWR);
    if (fb < 0) { perror(fbpath); return 1; }
    struct fb_var_screeninfo vi; struct fb_fix_screeninfo fi;
    if (ioctl(fb, FBIOGET_VSCREENINFO, &vi) || ioctl(fb, FBIOGET_FSCREENINFO, &fi)) { perror("fb ioctl"); return 1; }
    if (vi.bits_per_pixel != 32) { fprintf(stderr, "velashell: need 32 bpp framebuffer, got %u\n", vi.bits_per_pixel); return 1; }
    int W = vi.xres, H = vi.yres;
    uint8_t *fbmem = mmap(NULL, fi.line_length * vi.yres_virtual, PROT_READ | PROT_WRITE, MAP_SHARED, fb, 0);
    if (fbmem == MAP_FAILED) { perror("mmap"); return 1; }
    fprintf(stderr, "velashell: framebuffer %dx%d stride %u\n", W, H, fi.line_length);

    /* stop the text console from drawing over the compositor */
    int tty = open("/dev/tty0", O_RDWR);
    if (tty >= 0) ioctl(tty, KDSETMODE, KD_GRAPHICS);

    input_dev in[MAX_IN]; int nin = 0;
    DIR *d = opendir("/dev/input");
    struct dirent *de;
    while (d && (de = readdir(d)) && nin < MAX_IN) {
        if (strncmp(de->d_name, "event", 5)) continue;
        char p[300]; snprintf(p, sizeof p, "/dev/input/%s", de->d_name);
        int fd = open(p, O_RDONLY | O_NONBLOCK);
        if (fd < 0) continue;
        input_dev *x = &in[nin++]; memset(x, 0, sizeof *x); x->fd = fd;
        struct input_absinfo ax, ay;
        if (!ioctl(fd, EVIOCGABS(ABS_X), &ax) && !ioctl(fd, EVIOCGABS(ABS_Y), &ay) && ax.maximum > ax.minimum) {
            x->has_abs = 1; x->absx_min = ax.minimum; x->absx_max = ax.maximum; x->absy_min = ay.minimum; x->absy_max = ay.maximum;
        }
    }
    if (d) closedir(d);
    fprintf(stderr, "velashell: %d input devices\n", nin);

    static vs_shell s;
    if (vs_shell_init(&s, W, H, fonts)) { fprintf(stderr, "velashell: init failed (fonts at %s)\n", fonts); return 1; }
    s.draw_cursor = 1;
    const char *demo = getenv("VELA_DEMO");
    if (!demo || strcmp(demo, "0")) vs_shell_seed_demo(&s);
    const char *th = getenv("VELA_THEME");
    if (th && !strcmp(th, "light")) vs_shell_set_theme(&s, VS_THEME_LIGHT);
    if (th && !strcmp(th, "dawn")) vs_shell_set_theme(&s, VS_THEME_DAWN);

    vs_canvas c; vs_canvas_init(&c, W, H);
    int dirty = 1, ctrl = 0, alt = 0, meta = 0; long last_min = -1, frames = 0;
    struct pollfd pf[MAX_IN];
    for (int i = 0; i < nin; i++) { pf[i].fd = in[i].fd; pf[i].events = POLLIN; }
    for (;;) {
        s.now_unix = time(NULL);
        if (s.now_unix / 60 != last_min) { last_min = s.now_unix / 60; dirty = 1; }
        if (dirty) {
            vs_shell_render(&s, &c);
            for (int y = 0; y < H; y++) memcpy(fbmem + (size_t)y * fi.line_length, c.px + (size_t)y * W, (size_t)W * 4);
            dirty = 0;
            if (++frames == 1) fprintf(stderr, "velashell: first frame drawn; guest memory in use %ld kB\n", mem_available_kb());
        }
        int r = poll(pf, nin, 1000);
        if (r <= 0) continue;
        for (int i = 0; i < nin; i++) {
            if (!(pf[i].revents & POLLIN)) continue;
            struct input_event e;
            while (read(in[i].fd, &e, sizeof e) == sizeof e) {
                vs_event ve = { VS_EV_MOTION, s.cursor_x, s.cursor_y, 0 };
                int send = 0;
                if (e.type == EV_REL) {
                    if (e.code == REL_X) ve.x += e.value; else if (e.code == REL_Y) ve.y += e.value; else continue;
                    send = 1;
                } else if (e.type == EV_ABS && in[i].has_abs) {
                    if (e.code == ABS_X) ve.x = (float)(e.value - in[i].absx_min) * W / (in[i].absx_max - in[i].absx_min + 1);
                    else if (e.code == ABS_Y) ve.y = (float)(e.value - in[i].absy_min) * H / (in[i].absy_max - in[i].absy_min + 1);
                    else continue;
                    send = 1;
                } else if (e.type == EV_KEY) {
                    int down = e.value != 0;
                    if (e.code == KEY_LEFTCTRL || e.code == KEY_RIGHTCTRL) { ctrl = down; continue; }
                    if (e.code == KEY_LEFTALT || e.code == KEY_RIGHTALT) { alt = down; continue; }
                    if (e.code == KEY_LEFTMETA || e.code == KEY_RIGHTMETA) { meta = down; continue; }
                    if (e.code == BTN_LEFT) { ve.type = down ? VS_EV_DOWN : VS_EV_UP; send = 1; }
                    else if (e.value == 1) {
                        int combo = (ctrl && alt) || meta;
                        ve.type = VS_EV_KEY;
                        switch (e.code) {
                        case KEY_F11: ve.key = VS_KEY_F11; break;
                        case KEY_ESC: ve.key = VS_KEY_ESC; break;
                        case KEY_LEFT: if (combo) ve.key = VS_KEY_SPACE_LEFT; break;
                        case KEY_RIGHT: if (combo) ve.key = VS_KEY_SPACE_RIGHT; break;
                        case KEY_T: if (combo) ve.key = VS_KEY_THEME_NEXT; break;
                        case KEY_1: if (combo) ve.key = VS_KEY_THEME_LIGHT; break;
                        case KEY_2: if (combo) ve.key = VS_KEY_THEME_DAWN; break;
                        case KEY_3: if (combo) ve.key = VS_KEY_THEME_DEEP; break;
                        case KEY_N: if (combo) ve.key = VS_KEY_NOTIFY; break;
                        case KEY_M: if (combo) ve.key = VS_KEY_REDUCE_MOTION; break;
                        case KEY_EQUAL: if (combo) ve.key = VS_KEY_LARGER_TEXT; break;
                        }
                        send = ve.key != 0;
                    }
                }
                if (!send) continue;
                if (ve.x < 0) ve.x = 0;
                if (ve.y < 0) ve.y = 0;
                if (ve.x > W - 1) ve.x = W - 1;
                if (ve.y > H - 1) ve.y = H - 1;
                vs_shell_event(&s, &ve);
                s.cursor_x = ve.x; s.cursor_y = ve.y;
                dirty = 1;
            }
        }
    }
}
