/* Headless backend: renders deterministic preview frames to PNG and runs
 * behavioural self-tests (focus, z-order, drag, caption controls, dock,
 * Focus mode, Spaces, themes). Used by `make preview` and `make test`. */
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "third_party/stb_image_write.h"
#include "vs.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *out_dir = "out";
static int W = 1920, H = 1080;
static const char *font_dir = "assets/fonts";

static void save(vs_shell *s, vs_canvas *c, const char *name) {
    vs_shell_render(s, c);
    unsigned char *rgb = malloc((size_t)c->w * c->h * 3);
    for (int i = 0; i < c->w * c->h; i++) {
        uint32_t p = c->px[i];
        rgb[i * 3] = (p >> 16) & 255; rgb[i * 3 + 1] = (p >> 8) & 255; rgb[i * 3 + 2] = p & 255;
    }
    char path[512]; snprintf(path, sizeof path, "%s/%s", out_dir, name);
    if (!stbi_write_png(path, c->w, c->h, 3, rgb, c->w * 3)) fprintf(stderr, "write failed: %s\n", path);
    else printf("wrote %s\n", path);
    free(rgb);
}

static void fresh(vs_shell *s, vs_theme_id t) {
    if (s->wallpaper.px) vs_canvas_free(&s->wallpaper);
    if (vs_shell_init(s, W, H, font_dir)) { fprintf(stderr, "init failed (fonts in %s?)\n", font_dir); exit(2); }
    s->now_unix = 1791800640; /* Mon 12 Oct 2026 10:24 UTC: deterministic previews */
    vs_shell_set_theme(s, t);
}

static void ev(vs_shell *s, vs_ev_type t, float x, float y, int key) {
    vs_event e = { t, x, y, key }; vs_shell_event(s, &e);
}
static void clickat(vs_shell *s, float x, float y) { ev(s, VS_EV_DOWN, x, y, 0); ev(s, VS_EV_UP, x, y, 0); }
static void clickr(vs_shell *s, vs_rect r) { clickat(s, r.x + r.w * .5f, r.y + r.h * .5f); }

static int fails = 0;
#define CHECK(cond, msg) do { if (cond) printf("  pass  %s\n", msg); else { printf("  FAIL  %s\n", msg); fails++; } } while (0)

static int top_of_space(vs_shell *s) {
    for (int j = s->nz - 1; j >= 0; j--) { int i = s->zorder[j]; if (s->win[i].space == s->space && !s->win[i].minimized) return i; }
    return -1;
}

static int run_tests(void) {
    vs_shell s; memset(&s, 0, sizeof s); fresh(&s, VS_THEME_LIGHT);
    printf("velashell self-test (%dx%d)\n", W, H);
    int find = vs_shell_open_app(&s, 1), notes = vs_shell_open_app(&s, 3);
    CHECK(find >= 0 && notes >= 0, "two app windows open at once (A6)");
    CHECK(s.focused == notes && top_of_space(&s) == notes, "newest window focused and on top");
    /* click a visible part of the Find window (left edge) -> raise + focus */
    clickat(&s, s.win[find].x + 40, s.win[find].y + 200);
    CHECK(s.focused == find && top_of_space(&s) == find, "click-to-focus raises window (A6)");
    float ox = s.win[find].x, oy = s.win[find].y;
    ev(&s, VS_EV_DOWN, ox + 120, oy + 12, 0); ev(&s, VS_EV_MOTION, ox + 220, oy + 92, 0); ev(&s, VS_EV_UP, ox + 220, oy + 92, 0);
    CHECK((int)(s.win[find].x - ox) == 100 && (int)(s.win[find].y - oy) == 80, "drag by title area moves window (A7)");
    clickr(&s, vs_shell_caption_rect(&s, find, 1));
    CHECK(s.win[find].maximized, "□ maximises (A7)");
    clickr(&s, vs_shell_caption_rect(&s, find, 1));
    CHECK(!s.win[find].maximized, "□ restores");
    clickr(&s, vs_shell_caption_rect(&s, find, 0));
    CHECK(s.win[find].minimized && s.focused != find, "– minimises (A7)");
    clickr(&s, vs_shell_dock_item_rect(&s, 1));
    CHECK(!s.win[find].minimized && s.focused == find, "dock restores and focuses minimised app (A5/A7)");
    ev(&s, VS_EV_KEY, 0, 0, VS_KEY_F11);
    CHECK(s.focus_mode, "F11 enters Focus mode (bar + dock hidden)");
    ev(&s, VS_EV_KEY, 0, 0, VS_KEY_ESC);
    CHECK(!s.focus_mode, "Esc leaves Focus mode, chrome restored");
    int set = vs_shell_open_app(&s, 5);
    /* Settings swatch 2 = Deep */
    vs_rect sw; { float k = H / 1080.f; sw = (vs_rect){ (int)(s.win[set].x + 40 * k + 2 * 150 * k), (int)(s.win[set].y + 40 * k + 110 * k), (int)(120 * k), (int)(96 * k) }; }
    clickr(&s, sw);
    CHECK(s.theme == VS_THEME_DEEP, "Settings switches theme at runtime (A11)");
    ev(&s, VS_EV_KEY, 0, 0, VS_KEY_THEME_DAWN);
    CHECK(s.theme == VS_THEME_DAWN, "theme hotkey: Dawn");
    int sp = s.space;
    ev(&s, VS_EV_KEY, 0, 0, VS_KEY_SPACE_RIGHT);
    CHECK(s.space == (sp + 1) % VS_SPACES && s.focused == -1, "Space switch: new virtual desktop is empty");
    ev(&s, VS_EV_KEY, 0, 0, VS_KEY_SPACE_LEFT);
    CHECK(s.space == sp && s.focused >= 0, "Space switch back restores its windows");
    vs_shell_notify(&s, "Test", "Body", 0);
    clickat(&s, s.win[notes].x + 30, s.win[notes].y + 200);
    CHECK(s.notif.used, "notification persists across focus change (A9)");
    { float k = H / 1080.f; clickat(&s, W - 16 * k - 36 * k + 14 * k, 32 * k + 12 * k + 8 * k + 14 * k); }
    CHECK(!s.notif.used, "notification × dismisses (A9)");
    clickr(&s, vs_shell_caption_rect(&s, notes, 2));
    CHECK(!s.win[notes].used, "× closes window (A7)");
    printf("%s (%d failure%s)\n", fails ? "FAILED" : "OK", fails, fails == 1 ? "" : "s");
    return fails ? 1 : 0;
}

static void previews(void) {
    vs_canvas c; vs_canvas_init(&c, W, H);
    vs_shell s; memset(&s, 0, sizeof s);

    fresh(&s, VS_THEME_DEEP); vs_shell_seed_demo(&s); s.draw_cursor = 1; s.cursor_x = W * .55f; s.cursor_y = H * .82f;
    { vs_event e = { VS_EV_MOTION, s.cursor_x, s.cursor_y, 0 }; vs_shell_event(&s, &e); }
    { vs_event e = { VS_EV_MOTION, 0, 0, 0 }; vs_rect r = vs_shell_dock_item_rect(&s, 3); e.x = r.x + r.w / 2; e.y = r.y + r.h / 2; vs_shell_event(&s, &e); s.cursor_x = e.x + 6; s.cursor_y = e.y + 6; }
    save(&s, &c, "os0-deep-desktop.png");

    fresh(&s, VS_THEME_LIGHT); vs_shell_open_app(&s, 1); vs_shell_open_app(&s, 5);
    vs_shell_notify(&s, "Draft saved as a new version", "The original stays unchanged.", 3);
    save(&s, &c, "os0-light-settings.png");

    fresh(&s, VS_THEME_DAWN); vs_shell_open_app(&s, 3); vs_shell_open_app(&s, 0);
    save(&s, &c, "os0-dawn-home.png");

    fresh(&s, VS_THEME_DEEP); vs_shell_open_app(&s, 1);
    { vs_event e = { VS_EV_KEY, 0, 0, VS_KEY_F11 }; vs_shell_event(&s, &e); }
    vs_shell_notify(&s, "Focus mode", "Press F11 or Esc to show the bar and dock.", 1);
    save(&s, &c, "os0-deep-focus-mode.png");

    fresh(&s, VS_THEME_DEEP); vs_shell_open_app(&s, 3);
    { vs_event e = { VS_EV_KEY, 0, 0, VS_KEY_SPACE_RIGHT }; vs_shell_event(&s, &e); }
    vs_shell_open_app(&s, 4);
    save(&s, &c, "os0-deep-space2.png");

    fresh(&s, VS_THEME_DEEP);
    save(&s, &c, "os0-deep-empty.png");
    vs_canvas_free(&c);
}

int main(int argc, char **argv) {
    int test = 0;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--test")) test = 1;
        else if (!strcmp(argv[i], "--out") && i + 1 < argc) out_dir = argv[++i];
        else if (!strcmp(argv[i], "--fonts") && i + 1 < argc) font_dir = argv[++i];
        else if (!strcmp(argv[i], "--size") && i + 1 < argc) sscanf(argv[++i], "%dx%d", &W, &H);
        else { fprintf(stderr, "usage: %s [--test] [--out DIR] [--fonts DIR] [--size WxH]\n", argv[0]); return 2; }
    }
    if (test) return run_tests();
    previews();
    return 0;
}
