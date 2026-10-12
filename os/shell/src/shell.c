/* VelaOS desktop shell (OS-0 scaffold).
 * Scene graph + input routing for: wallpaper, top system bar, floating dock,
 * decorated windows (– □ × on the right), Focus mode, Spaces (virtual desktops),
 * top-right notifications, Light/Dawn/Deep themes. Rendering is a full
 * software composite into an XRGB canvas; glass = backdrop blur + tint + frost. */
#include "vs.h"
#include <math.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

const char *vs_dock_names[VS_DOCK_ITEMS] = { "Vela", "Find", "Spaces", "Notes", "Media", "Settings" };
static const vs_glyph dock_glyphs[VS_DOCK_ITEMS] = { VS_G_SAIL, VS_G_SEARCH, VS_G_SPACES, VS_G_NOTES, VS_G_MEDIA, VS_G_SETTINGS };
enum { APP_VELA, APP_FIND, APP_SPACES, APP_NOTES, APP_MEDIA, APP_SETTINGS };

/* ---------- metrics (1920x1080 reference, scaled by height) ---------- */
static float S(const vs_shell *s) { return s->h / 1080.f; }
static float T(const vs_shell *s) { return S(s) * (s->larger_text ? 1.18f : 1.f); }
static float bar_h(const vs_shell *s) { return roundf(32 * S(s)); }
static float title_h(const vs_shell *s) { return roundf(40 * S(s)); }

static void dock_geom(const vs_shell *s, float *x, float *y, float *w, float *h) {
    float k = S(s), slot = 56 * k, gap = 12 * k, pad = 12 * k;
    *w = VS_DOCK_ITEMS * slot + (VS_DOCK_ITEMS - 1) * gap + 2 * pad;
    *h = 72 * k;
    *x = roundf((s->w - *w) * .5f);
    *y = roundf(s->h - 16 * k - *h);
}

vs_rect vs_shell_dock_item_rect(const vs_shell *s, int i) {
    float x, y, w, h, k = S(s);
    dock_geom(s, &x, &y, &w, &h);
    float slot = 56 * k, gap = 12 * k, pad = 12 * k;
    vs_rect r = { (int)(x + pad + i * (slot + gap)), (int)(y + (h - slot) * .5f), (int)slot, (int)slot };
    return r;
}

static void notif_geom(const vs_shell *s, float *x, float *y, float *w, float *h) {
    float k = S(s);
    *w = 360 * k; *h = 76 * k;
    *x = s->w - 16 * k - *w;
    *y = (s->focus_mode ? 0 : bar_h(s)) + 12 * k;
}

/* effective on-screen rect for a window (maximised / Focus mode aware) */
static void win_geom(const vs_shell *s, int i, float *x, float *y, float *w, float *h, float *rad) {
    const vs_window *wn = &s->win[i];
    float k = S(s);
    if (s->focus_mode && i == s->focused) { *x = 0; *y = 0; *w = s->w; *h = s->h; *rad = 0; return; }
    if (wn->maximized) {
        float dx, dy, dw, dh; dock_geom(s, &dx, &dy, &dw, &dh);
        *x = 0; *y = bar_h(s); *w = s->w; *h = dy - 12 * k - *y; *rad = 0; return;
    }
    *x = wn->x; *y = wn->y; *w = wn->w; *h = wn->h; *rad = 16 * k;
}

vs_rect vs_shell_caption_rect(const vs_shell *s, int i, int which) {
    float x, y, w, h, rad, k = S(s);
    win_geom(s, i, &x, &y, &w, &h, &rad);
    float hit = 32 * k, right = x + w - 8 * k;
    vs_rect r = { (int)(right - (3 - which) * hit), (int)(y + (title_h(s) - hit) * .5f), (int)hit, (int)hit };
    return r;
}

static int inside(vs_rect r, float x, float y) { return x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h; }

/* ---------- state ---------- */
int vs_shell_init(vs_shell *s, int w, int h, const char *font_dir) {
    memset(s, 0, sizeof *s);
    s->w = w; s->h = h; s->focused = -1; s->hover_dock = -1;
    s->theme = VS_THEME_DEEP;            /* signature night-sail look on first run */
    s->cursor_x = w * .5f; s->cursor_y = h * .5f;
    s->now_unix = (long long)time(NULL);
    if (vs_canvas_init(&s->wallpaper, w, h)) return -1;
    return vs_text_init(font_dir);
}

void vs_shell_set_theme(vs_shell *s, vs_theme_id t) {
    if (t != s->theme) { s->theme = t; s->wallpaper_valid = 0; }
}

static void raise_win(vs_shell *s, int i) {
    int j, n = 0;
    for (j = 0; j < s->nz; j++) if (s->zorder[j] != i) s->zorder[n++] = s->zorder[j];
    s->zorder[n++] = i; s->nz = n;
    s->focused = i;
}

static int find_app_window(const vs_shell *s, int app) {
    for (int i = 0; i < VS_MAX_WINDOWS; i++)
        if (s->win[i].used && s->win[i].app == app && s->win[i].space == s->space) return i;
    return -1;
}

int vs_shell_open_app(vs_shell *s, int app) {
    if (app == APP_SPACES) { /* Spaces glyph = next virtual desktop */
        s->space = (s->space + 1) % VS_SPACES; s->wallpaper_valid = 0; s->focused = -1;
        for (int j = s->nz - 1; j >= 0; j--) { int i = s->zorder[j]; if (s->win[i].space == s->space && !s->win[i].minimized) { s->focused = i; break; } }
        return -1;
    }
    int i = find_app_window(s, app);
    if (i >= 0) {
        if (s->focused == i && !s->win[i].minimized) { s->win[i].minimized = 1; s->focused = -1; return i; } /* click focused = minimise */
        s->win[i].minimized = 0; raise_win(s, i); return i;
    }
    for (i = 0; i < VS_MAX_WINDOWS && s->win[i].used; i++) ;
    if (i == VS_MAX_WINDOWS) return -1;
    float k = S(s);
    vs_window *w = &s->win[i];
    memset(w, 0, sizeof *w);
    w->used = 1; w->id = i; w->app = app; w->space = s->space;
    static const char *titles[] = { "Vela", "Find", "Spaces", "Notes", "Media", "Settings" };
    snprintf(w->title, sizeof w->title, "%s", titles[app]);
    static const float geo[][4] = { /* x, y, w, h at reference */
        { 520, 150, 880, 560 }, { 300, 120, 1040, 680 }, { 0 }, { 1160, 170, 560, 640 }, { 760, 260, 820, 520 }, { 640, 200, 640, 560 } };
    w->x = geo[app][0] * k; w->y = geo[app][1] * k; w->w = geo[app][2] * k; w->h = geo[app][3] * k;
    raise_win(s, i);
    return i;
}

void vs_shell_notify(vs_shell *s, const char *title, const char *body, int app) {
    s->notif.used = 1; s->notif.app = app;
    snprintf(s->notif.title, sizeof s->notif.title, "%s", title);
    snprintf(s->notif.body, sizeof s->notif.body, "%s", body);
}

void vs_shell_seed_demo(vs_shell *s) {
    vs_shell_open_app(s, APP_FIND);
    vs_shell_open_app(s, APP_NOTES);
    vs_shell_notify(s, "Welcome to VelaOS", "Everything stays on this device.", APP_VELA);
}

static void close_win(vs_shell *s, int i) {
    s->win[i].used = 0;
    int n = 0;
    for (int j = 0; j < s->nz; j++) if (s->zorder[j] != i) s->zorder[n++] = s->zorder[j];
    s->nz = n;
    if (s->focused == i) {
        s->focused = -1; s->focus_mode = 0;
        for (int j = s->nz - 1; j >= 0; j--) { int q = s->zorder[j]; if (s->win[q].space == s->space && !s->win[q].minimized) { s->focused = q; break; } }
    }
}

/* ---------- input ---------- */
static int hit_window(const vs_shell *s, float px, float py) {
    for (int j = s->nz - 1; j >= 0; j--) {
        int i = s->zorder[j];
        const vs_window *w = &s->win[i];
        if (!w->used || w->minimized || w->space != s->space) continue;
        if (s->focus_mode && i != s->focused) continue;
        float x, y, ww, hh, r; win_geom(s, i, &x, &y, &ww, &hh, &r);
        if (px >= x && py >= y && px < x + ww && py < y + hh) return i;
    }
    return -1;
}

/* Settings window: theme swatches + accessibility toggles (content-local hit test) */
static vs_rect settings_swatch(const vs_shell *s, int i, int t) {
    float x, y, w, h, r, k = S(s); win_geom(s, i, &x, &y, &w, &h, &r);
    vs_rect q = { (int)(x + 40 * k + t * 150 * k), (int)(y + title_h(s) + 110 * k), (int)(120 * k), (int)(96 * k) };
    return q;
}
static vs_rect settings_toggle(const vs_shell *s, int i, int t) {
    float x, y, w, h, r, k = S(s); win_geom(s, i, &x, &y, &w, &h, &r);
    vs_rect q = { (int)(x + 40 * k), (int)(y + title_h(s) + 290 * k + t * 56 * k), (int)(w - 80 * k), (int)(44 * k) };
    return q;
}

static int click(vs_shell *s, float px, float py) {
    float k = S(s);
    if (s->notif.used) {
        float nx, ny, nw, nh; notif_geom(s, &nx, &ny, &nw, &nh);
        if (px >= nx && py >= ny && px < nx + nw && py < ny + nh) {
            vs_rect xr = { (int)(nx + nw - 36 * k), (int)(ny + 8 * k), (int)(28 * k), (int)(28 * k) };
            if (inside(xr, px, py)) s->notif.used = 0;
            return 1;
        }
    }
    if (!s->focus_mode) {
        for (int d = 0; d < VS_DOCK_ITEMS; d++)
            if (inside(vs_shell_dock_item_rect(s, d), px, py)) { vs_shell_open_app(s, d); return 1; }
        if (py < bar_h(s)) {
            /* moon/sun: cycle Light -> Dawn -> Deep; "Spaces" menu: next Space */
            float right = s->w - 16 * k;
            float clock_w = vs_text_width(VS_FONT_UI_BOLD, 13 * T(s), "Mon 00 Oct  00:00");
            float mx = right - clock_w - 16 * k - 22 * k;
            if (px >= mx - 4 * k && px <= mx + 22 * k) { vs_shell_set_theme(s, (vs_theme_id)((s->theme + 1) % VS_THEME_COUNT)); return 1; }
            return 0;
        }
    }
    int i = hit_window(s, px, py);
    if (i < 0) { s->focused = -1; return 1; }
    raise_win(s, i);
    for (int b = 0; b < 3; b++)
        if (inside(vs_shell_caption_rect(s, i, b), px, py)) {
            if (b == 0) { s->win[i].minimized = 1; s->focused = -1; s->focus_mode = 0; }
            else if (b == 1) { if (s->focus_mode) s->focus_mode = 0; else s->win[i].maximized = !s->win[i].maximized; }
            else close_win(s, i);
            return 1;
        }
    float x, y, w, h, r; win_geom(s, i, &x, &y, &w, &h, &r);
    if (py < y + title_h(s) && !s->win[i].maximized && !s->focus_mode) {
        s->dragging = 1; s->drag_dx = px - s->win[i].x; s->drag_dy = py - s->win[i].y;
        return 1;
    }
    if (s->win[i].app == APP_SETTINGS) {
        for (int t = 0; t < VS_THEME_COUNT; t++) if (inside(settings_swatch(s, i, t), px, py)) vs_shell_set_theme(s, (vs_theme_id)t);
        if (inside(settings_toggle(s, i, 0), px, py)) s->larger_text = !s->larger_text;
        if (inside(settings_toggle(s, i, 1), px, py)) s->reduce_motion = !s->reduce_motion;
    }
    return 1;
}

int vs_shell_event(vs_shell *s, const vs_event *ev) {
    switch (ev->type) {
    case VS_EV_MOTION: {
        s->cursor_x = ev->x; s->cursor_y = ev->y;
        if (s->dragging && s->focused >= 0) {
            vs_window *w = &s->win[s->focused];
            w->x = ev->x - s->drag_dx; w->y = ev->y - s->drag_dy;
            if (w->y < bar_h(s)) w->y = bar_h(s);
            if (w->y > s->h - title_h(s)) w->y = s->h - title_h(s);
        }
        int hd = -1;
        if (!s->focus_mode) for (int d = 0; d < VS_DOCK_ITEMS; d++) if (inside(vs_shell_dock_item_rect(s, d), ev->x, ev->y)) hd = d;
        s->hover_dock = hd;
        return 1; }
    case VS_EV_DOWN: s->cursor_x = ev->x; s->cursor_y = ev->y; return click(s, ev->x, ev->y);
    case VS_EV_UP: s->dragging = 0; return 0;
    case VS_EV_KEY:
        switch (ev->key) {
        case VS_KEY_F11: if (s->focused >= 0) s->focus_mode = !s->focus_mode; return 1;
        case VS_KEY_ESC: if (s->focus_mode) { s->focus_mode = 0; return 1; } return 0;
        case VS_KEY_SPACE_LEFT: case VS_KEY_SPACE_RIGHT:
            s->space = (s->space + (ev->key == VS_KEY_SPACE_RIGHT ? 1 : VS_SPACES - 1)) % VS_SPACES;
            s->focus_mode = 0; s->wallpaper_valid = 0; s->focused = -1;
            for (int j = s->nz - 1; j >= 0; j--) { int q = s->zorder[j]; if (s->win[q].space == s->space && !s->win[q].minimized) { s->focused = q; break; } }
            return 1;
        case VS_KEY_THEME_NEXT: vs_shell_set_theme(s, (vs_theme_id)((s->theme + 1) % VS_THEME_COUNT)); return 1;
        case VS_KEY_THEME_LIGHT: vs_shell_set_theme(s, VS_THEME_LIGHT); return 1;
        case VS_KEY_THEME_DAWN: vs_shell_set_theme(s, VS_THEME_DAWN); return 1;
        case VS_KEY_THEME_DEEP: vs_shell_set_theme(s, VS_THEME_DEEP); return 1;
        case VS_KEY_NOTIFY: vs_shell_notify(s, "Transfer running", "You can keep working. It continues in the background.", APP_MEDIA); return 1;
        case VS_KEY_REDUCE_MOTION: s->reduce_motion = !s->reduce_motion; return 1;
        case VS_KEY_LARGER_TEXT: s->larger_text = !s->larger_text; return 1;
        }
    }
    return 0;
}

/* ---------- drawing ---------- */
static void text_c(vs_canvas *c, int f, float px, float cx, float base, const char *t, vs_rgba col) {
    vs_text(c, f, px, cx - vs_text_width(f, px, t) * .5f, base, t, col);
}

static void glass(vs_shell *s, vs_canvas *c, float x, float y, float w, float h, float rad, vs_rgba tint, int blur) {
    const vs_theme *th = vs_theme_get(s->theme);
    vs_backdrop_blur_rrect(c, x, y, w, h, rad, blur);
    vs_tint_rrect(c, x, y, w, h, rad, tint, th->glass_frost);
    vs_stroke_rrect(c, x, y, w, h, rad, 1.f, th->glass_stroke);
}

static void draw_bar(vs_shell *s, vs_canvas *c) {
    const vs_theme *th = vs_theme_get(s->theme);
    float k = S(s), t = T(s), bh = bar_h(s);
    vs_backdrop_blur_rrect(c, 0, 0, s->w, bh, 0, (int)(24 * k));
    vs_fill_rect(c, (vs_rect){ 0, 0, s->w, (int)bh }, th->bar_tint);
    vs_fill_rect(c, (vs_rect){ 0, (int)bh - 1, s->w, 1 }, th->glass_stroke);
    float base = bh * .5f + 4.6f * t;
    float x = 14 * k;
    vs_glyph_draw(c, VS_G_SAIL, x, (bh - 18 * k) * .5f, 18 * k, th->on_glass);
    x += 26 * k;
    x = vs_text(c, VS_FONT_UI_BOLD, 13.5f * t, x, base, "VelaOS", th->on_glass) + 22 * k;
    static const char *app_menus[] = { "File", "Edit", "View", "Spaces", "Window", "Help" };
    static const char *sys_menus[] = { "Spaces", "Files", "Apps", "Settings" };
    if (s->focused >= 0) {
        x = vs_text(c, VS_FONT_UI_BOLD, 13 * t, x, base, s->win[s->focused].title, th->on_glass) + 20 * k;
        for (int i = 0; i < 6; i++) x = vs_text(c, VS_FONT_UI, 13 * t, x, base, app_menus[i], th->on_glass_muted) + 20 * k;
    } else
        for (int i = 0; i < 4; i++) x = vs_text(c, VS_FONT_UI, 13 * t, x, base, sys_menus[i], th->on_glass_muted) + 20 * k;

    /* right cluster: Spaces dots, search, network, volume, battery, theme, clock */
    char clk[64];
    time_t now = (time_t)s->now_unix; struct tm tmv; localtime_r(&now, &tmv);
    strftime(clk, sizeof clk, "%a %d %b  %H:%M", &tmv);
    float r = s->w - 16 * k;
    float cw = vs_text_width(VS_FONT_UI_BOLD, 13 * t, "Mon 00 Oct  00:00");
    vs_text(c, VS_FONT_UI_BOLD, 13 * t, r - vs_text_width(VS_FONT_UI_BOLD, 13 * t, clk), base, clk, th->on_glass);
    r -= cw + 16 * k;
    float g = 18 * k, gy = (bh - g) * .5f;
    vs_glyph_draw(c, s->theme == VS_THEME_DEEP ? VS_G_SUN : VS_G_MOON, r - 22 * k, gy, g, th->on_glass); r -= 22 * k + 14 * k;
    vs_glyph_draw(c, VS_G_BATTERY, r - 22 * k, gy, g + 4 * k, th->on_glass); r -= 22 * k + 14 * k;
    vs_glyph_draw(c, VS_G_VOLUME, r - 20 * k, gy, g, th->on_glass); r -= 20 * k + 14 * k;
    vs_glyph_draw(c, VS_G_WIFI, r - 20 * k, gy, g, th->on_glass); r -= 20 * k + 14 * k;
    vs_glyph_draw(c, VS_G_SEARCH, r - 20 * k, gy, g, th->on_glass); r -= 20 * k + 22 * k;
    for (int i = VS_SPACES - 1; i >= 0; i--) {
        float cx = r - 4 * k;
        if (i == s->space) vs_fill_rrect(c, cx - 14 * k, bh * .5f - 3 * k, 18 * k, 6 * k, 3 * k, th->on_glass);
        else vs_disc(c, cx - 2 * k, bh * .5f, 3 * k, th->on_glass_muted);
        r -= (i == s->space ? 26 : 14) * k;
    }
}

static void draw_dock(vs_shell *s, vs_canvas *c) {
    const vs_theme *th = vs_theme_get(s->theme);
    float x, y, w, h, k = S(s);
    dock_geom(s, &x, &y, &w, &h);
    vs_shadow_rrect(c, x, y, w, h, 22 * k, 18 * k, 8 * k, .22f);
    glass(s, c, x, y, w, h, 22 * k, th->dock_tint, (int)(28 * k));
    int focused_app = s->focused >= 0 ? s->win[s->focused].app : -1;
    for (int d = 0; d < VS_DOCK_ITEMS; d++) {
        vs_rect r = vs_shell_dock_item_rect(s, d);
        int running = 0;
        for (int i = 0; i < VS_MAX_WINDOWS; i++) if (s->win[i].used && s->win[i].app == d && s->win[i].space == s->space) running = 1;
        if (d == APP_SPACES) running = 0;
        if (d == focused_app) {
            /* section 7 #2: frosted light tile + 2 px teal underline */
            vs_fill_rrect(c, r.x, r.y, r.w, r.h, 14 * k, th->active_tile);
            vs_stroke_rrect(c, r.x, r.y, r.w, r.h, 14 * k, 1.f, (vs_rgba){ 1, 1, 1, .18f });
            vs_fill_rrect(c, r.x + r.w * .32f, r.y + r.h - 5 * k, r.w * .36f, 2.f * k < 2 ? 2 : 2.f * k, 1 * k, vs_hex(0x45B3BD, 1));
        } else if (d == s->hover_dock) {
            vs_fill_rrect(c, r.x, r.y, r.w, r.h, 14 * k, (vs_rgba){ 1, 1, 1, .08f });
        }
        if (running && d != focused_app) vs_disc(c, r.x + r.w * .5f, r.y + r.h - 3 * k, 2 * k, th->on_glass_muted);
        float gs = 30 * k;
        vs_glyph_draw(c, dock_glyphs[d], r.x + (r.w - gs) * .5f, r.y + (r.h - gs) * .5f - 1 * k, gs, th->on_glass);
    }
    if (s->hover_dock >= 0) { /* tooltip label above the hovered glyph */
        vs_rect r = vs_shell_dock_item_rect(s, s->hover_dock);
        const char *n = vs_dock_names[s->hover_dock];
        float tw = vs_text_width(VS_FONT_UI, 12 * T(s), n) + 20 * k;
        float tx = r.x + r.w * .5f - tw * .5f, ty = y - 36 * k;
        glass(s, c, tx, ty, tw, 26 * k, 13 * k, th->dock_tint, (int)(16 * k));
        text_c(c, VS_FONT_UI, 12 * T(s), r.x + r.w * .5f, ty + 17.5f * k, n, th->on_glass);
    }
}

static void draw_notification(vs_shell *s, vs_canvas *c) {
    if (!s->notif.used) return;
    const vs_theme *th = vs_theme_get(s->theme);
    float x, y, w, h, k = S(s), t = T(s);
    notif_geom(s, &x, &y, &w, &h);
    vs_shadow_rrect(c, x, y, w, h, 14 * k, 16 * k, 6 * k, .20f);
    glass(s, c, x, y, w, h, 14 * k, th->notif_tint, (int)(24 * k));
    vs_fill_rrect(c, x + 14 * k, y + 16 * k, 40 * k, 40 * k, 11 * k, th->accent);
    vs_glyph_draw(c, dock_glyphs[s->notif.app], x + 20 * k, y + 22 * k, 28 * k, (vs_rgba){ 1, 1, 1, .95f });
    vs_rect clip = { (int)(x + 66 * k), (int)y, (int)(w - 66 * k - 36 * k), (int)h };
    vs_set_clip(&clip);
    vs_text(c, VS_FONT_UI_BOLD, 13.5f * t, x + 66 * k, y + 32 * k, s->notif.title, th->notif_text);
    vs_text(c, VS_FONT_UI, 12.5f * t, x + 66 * k, y + 54 * k, s->notif.body, th->notif_text_muted);
    vs_set_clip(NULL);
    vs_glyph_draw(c, VS_G_CLOSE, x + w - 30 * k, y + 12 * k, 16 * k, th->notif_text_muted);
}

/* ---- window contents (fixtures only; real apps land in OS-8/OS-9) ---- */
static void content_vela(vs_shell *s, vs_canvas *c, const vs_theme *th, float x, float y, float w, float h) {
    float k = S(s), t = T(s);
    (void)h;
    vs_glyph_draw(c, VS_G_SAIL, x + w * .5f - 14 * k, y + 36 * k, 28 * k, th->accent);
    text_c(c, VS_FONT_SERIF, 30 * t, x + w * .5f, y + 106 * k, "What would you like to do?", th->text);
    static const char *ti[] = { "Find something", "Create a workspace", "Resume my work" };
    static const char *su[] = { "Files, apps, pages, ideas", "Bring together what you need.", "Pick up where you left off." };
    static const vs_glyph gi[] = { VS_G_SEARCH, VS_G_SPACES, VS_G_NOTES };
    for (int i = 0; i < 3; i++) {
        float ty = y + 140 * k + i * 78 * k, tx = x + 60 * k, tw = w - 120 * k;
        vs_fill_rrect(c, tx, ty, tw, 66 * k, 12 * k, th->sidebar);
        vs_fill_rrect(c, tx + 12 * k, ty + 11 * k, 44 * k, 44 * k, 10 * k, th->accent_tint);
        vs_glyph_draw(c, gi[i], tx + 20 * k, ty + 19 * k, 28 * k, th->accent);
        vs_text(c, VS_FONT_UI_BOLD, 15 * t, tx + 72 * k, ty + 29 * k, ti[i], th->text);
        vs_text(c, VS_FONT_UI, 13 * t, tx + 72 * k, ty + 50 * k, su[i], th->text_muted);
        vs_line(c, tx + tw - 28 * k, ty + 27 * k, tx + tw - 21 * k, ty + 33 * k, 1.6f * k, th->text_muted);
        vs_line(c, tx + tw - 21 * k, ty + 33 * k, tx + tw - 28 * k, ty + 39 * k, 1.6f * k, th->text_muted);
    }
}

static void content_find(vs_shell *s, vs_canvas *c, const vs_theme *th, float x, float y, float w, float h) {
    float k = S(s), t = T(s), sb = 200 * k;
    vs_fill_rect(c, (vs_rect){ (int)x, (int)y, (int)sb, (int)h }, th->sidebar);
    static const char *nav[] = { "Home", "Find", "Create", "Spaces", "Me" };
    for (int i = 0; i < 5; i++) {
        float ny = y + 24 * k + i * 40 * k;
        if (i == 1) vs_fill_rrect(c, x + 12 * k, ny, sb - 24 * k, 32 * k, 16 * k, th->accent_tint);
        vs_text(c, i == 1 ? VS_FONT_UI_BOLD : VS_FONT_UI, 13.5f * t, x + 28 * k, ny + 21 * k, nav[i], i == 1 ? th->accent : th->text);
    }
    float cx = x + sb + 36 * k, cw = w - sb - 72 * k;
    vs_text(c, VS_FONT_SERIF, 30 * t, cx, y + 58 * k, "Find by meaning", th->text);
    vs_text(c, VS_FONT_UI, 14 * t, cx, y + 86 * k, "Describe what you are looking for. VelaOS understands your intent.", th->text_muted);
    vs_fill_rrect(c, cx, y + 108 * k, cw, 48 * k, 24 * k, th->window);
    vs_stroke_rrect(c, cx, y + 108 * k, cw, 48 * k, 24 * k, 1.f, th->divider);
    vs_glyph_draw(c, VS_G_SEARCH, cx + 14 * k, y + 120 * k, 24 * k, th->text_muted);
    vs_text(c, VS_FONT_UI, 15 * t, cx + 48 * k, y + 138 * k, "slides about clean energy from last week", th->text);
    vs_disc(c, cx + cw - 24 * k, y + 132 * k, 17 * k, th->accent);
    vs_line(c, cx + cw - 31 * k, y + 132 * k, cx + cw - 17 * k, y + 132 * k, 2 * k, (vs_rgba){ 1, 1, 1, 1 });
    vs_line(c, cx + cw - 23 * k, y + 125 * k, cx + cw - 17 * k, y + 132 * k, 2 * k, (vs_rgba){ 1, 1, 1, 1 });
    vs_line(c, cx + cw - 23 * k, y + 139 * k, cx + cw - 17 * k, y + 132 * k, 2 * k, (vs_rgba){ 1, 1, 1, 1 });
    static const char *chips[] = { "Documents", "Recent", "Shared with me" };
    float chx = cx;
    for (int i = 0; i < 3; i++) {
        float cwid = vs_text_width(VS_FONT_UI, 13 * t, chips[i]) + 28 * k;
        if (i == 0) vs_fill_rrect(c, chx, y + 172 * k, cwid, 30 * k, 15 * k, th->accent);
        else vs_stroke_rrect(c, chx, y + 172 * k, cwid, 30 * k, 15 * k, 1.f, th->divider);
        vs_text(c, VS_FONT_UI, 13 * t, chx + 14 * k, y + 192 * k, chips[i], i == 0 ? (vs_rgba){ 1, 1, 1, 1 } : th->text);
        chx += cwid + 10 * k;
    }
    static const char *rt[] = { "Clean Energy Draft", "Market Research 2026", "Coastal field notes" };
    static const char *rs[] = { "Presentation  ·  Edited 3 days ago", "Document  ·  Edited last week", "Note  ·  Edited yesterday" };
    for (int i = 0; i < 3; i++) {
        float ry = y + 222 * k + i * 86 * k;
        if (ry + 76 * k > y + h) break;
        vs_fill_rrect(c, cx, ry, cw, 76 * k, 12 * k, i == 0 ? th->accent_tint : th->window);
        vs_stroke_rrect(c, cx, ry, cw, 76 * k, 12 * k, 1.f, i == 0 ? th->accent : th->divider);
        vs_fill_rrect(c, cx + 12 * k, ry + 12 * k, 72 * k, 52 * k, 8 * k, th->sidebar);
        vs_glyph_draw(c, i == 2 ? VS_G_NOTES : VS_G_MEDIA, cx + 34 * k, ry + 24 * k, 28 * k, th->text_muted);
        vs_text(c, VS_FONT_UI_BOLD, 15 * t, cx + 100 * k, ry + 32 * k, rt[i], th->text);
        vs_text(c, VS_FONT_UI, 13 * t, cx + 100 * k, ry + 54 * k, rs[i], th->text_muted);
    }
}

static void content_notes(vs_shell *s, vs_canvas *c, const vs_theme *th, float x, float y, float w, float h) {
    float k = S(s), t = T(s), px = x + 32 * k;
    (void)w; (void)h;
    vs_text(c, VS_FONT_SERIF, 24 * t, px, y + 50 * k, "Ideas for the opening slide", th->text);
    static const char *b[] = { "Start with the coastline at dawn", "One number, large: 42% less energy", "Promise: local, private, reversible" };
    for (int i = 0; i < 3; i++) {
        vs_disc(c, px + 4 * k, y + 88 * k + i * 32 * k, 3 * k, th->text);
        vs_text(c, VS_FONT_UI, 14.5f * t, px + 18 * k, y + 93 * k + i * 32 * k, b[i], th->text);
    }
    static const char *cl[] = { "Collect three photos", "Draft speaker notes" };
    for (int i = 0; i < 2; i++) {
        float cy = y + 206 * k + i * 36 * k;
        if (i == 0) { vs_fill_rrect(c, px, cy - 9 * k, 18 * k, 18 * k, 5 * k, th->accent);
                      vs_line(c, px + 4 * k, cy, px + 8 * k, cy + 4 * k, 2 * k, (vs_rgba){ 1, 1, 1, 1 });
                      vs_line(c, px + 8 * k, cy + 4 * k, px + 14 * k, cy - 4 * k, 2 * k, (vs_rgba){ 1, 1, 1, 1 }); }
        else vs_stroke_rrect(c, px, cy - 9 * k, 18 * k, 18 * k, 5 * k, 1.5f * k, th->text_muted);
        vs_text(c, VS_FONT_UI, 14.5f * t, px + 30 * k, cy + 5 * k, cl[i], i == 0 ? th->text_muted : th->text);
    }
    vs_text(c, VS_FONT_UI, 12.5f * t, px, y + h - 22 * k, "Saved on this device", th->text_muted);
}

static void content_media(vs_shell *s, vs_canvas *c, const vs_theme *th, float x, float y, float w, float h) {
    float k = S(s);
    vs_fill_rrect(c, x + 16 * k, y + 8 * k, w - 32 * k, h - 72 * k, 12 * k, vs_hex(0x0B1620, 1));
    vs_glyph_draw(c, VS_G_MEDIA, x + w * .5f - 36 * k, y + (h - 72 * k) * .5f - 28 * k, 72 * k, (vs_rgba){ 1, 1, 1, .9f });
    vs_fill_rrect(c, x + 16 * k, y + h - 44 * k, w - 32 * k, 4 * k, 2 * k, th->divider);
    vs_fill_rrect(c, x + 16 * k, y + h - 44 * k, (w - 32 * k) * .38f, 4 * k, 2 * k, th->accent);
}

static void content_settings(vs_shell *s, vs_canvas *c, const vs_theme *th, int wi, float x, float y, float w, float h) {
    float k = S(s), t = T(s);
    (void)w; (void)h;
    vs_text(c, VS_FONT_SERIF, 26 * t, x + 40 * k, y + 50 * k, "Appearance", th->text);
    vs_text(c, VS_FONT_UI, 13.5f * t, x + 40 * k, y + 78 * k, "Make it yours. Changes apply instantly.", th->text_muted);
    static const uint32_t sw_a[] = { 0xF4F1ED, 0xF6B98E, 0x0E223B }, sw_b[] = { 0xDCE7EC, 0x8FA9D6, 0x163A4A };
    for (int i = 0; i < VS_THEME_COUNT; i++) {
        vs_rect r = settings_swatch(s, wi, i);
        float cx = r.x + r.w * .5f, cy = r.y + 30 * k;
        for (int q = 0; q < 28; q++) { /* vertical two-stop gradient disc */
            float tt = q / 27.f;
            vs_rgba a = vs_hex(sw_a[i], 1), b = vs_hex(sw_b[i], 1);
            vs_rgba m = { a.r + (b.r - a.r) * tt, a.g + (b.g - a.g) * tt, a.b + (b.b - a.b) * tt, 1 };
            vs_rect band = { (int)(cx - 26 * k), (int)(cy - 26 * k + q * 52 * k / 28), (int)(52 * k), (int)(52 * k / 28 + 1) };
            vs_set_clip(&band); vs_disc(c, cx, cy, 26 * k, m); vs_set_clip(NULL);
        }
        vs_ring(c, cx, cy, 26 * k, 1.f, th->divider);
        if (s->theme == (vs_theme_id)i) vs_ring(c, cx, cy, 32 * k, 2.5f * k, th->accent);
        text_c(c, s->theme == (vs_theme_id)i ? VS_FONT_UI_BOLD : VS_FONT_UI, 13.5f * t, cx, r.y + 84 * k, vs_theme_get((vs_theme_id)i)->name, th->text);
    }
    static const char *tg[] = { "Larger text", "Reduce motion" };
    int on[] = { s->larger_text, s->reduce_motion };
    for (int i = 0; i < 2; i++) {
        vs_rect r = settings_toggle(s, wi, i);
        vs_fill_rrect(c, r.x, r.y, r.w, r.h, 10 * k, th->sidebar);
        vs_text(c, VS_FONT_UI, 14.5f * t, r.x + 16 * k, r.y + r.h * .5f + 5 * k, tg[i], th->text);
        float sx = r.x + r.w - 60 * k, sy = r.y + (r.h - 24 * k) * .5f;
        vs_fill_rrect(c, sx, sy, 44 * k, 24 * k, 12 * k, on[i] ? th->accent : th->divider);
        vs_disc(c, on[i] ? sx + 32 * k : sx + 12 * k, sy + 12 * k, 9 * k, (vs_rgba){ 1, 1, 1, 1 });
    }
}

static void draw_window(vs_shell *s, vs_canvas *c, int i) {
    const vs_theme *th = vs_theme_get(s->theme);
    const vs_window *wn = &s->win[i];
    float x, y, w, h, rad, k = S(s), t = T(s);
    win_geom(s, i, &x, &y, &w, &h, &rad);
    int active = (i == s->focused);
    if (rad > 0) vs_shadow_rrect(c, x, y, w, h, rad, (active ? 28 : 18) * k, (active ? 14 : 8) * k, th->shadow_alpha * (active ? 1.f : .6f));
    vs_fill_rrect(c, x, y, w, h, rad, th->window);
    float th_h = title_h(s);
    /* integrated title area: same surface, hairline divider, glyph + title, – □ × right */
    vs_rect clip = { (int)x, (int)y, (int)w, (int)h };
    vs_set_clip(&clip);
    vs_rect tclip = { (int)x, (int)y, (int)w, (int)th_h };
    vs_set_clip(&tclip);
    vs_fill_rrect(c, x, y, w, h, rad, th->title_area);
    vs_set_clip(&clip);
    vs_fill_rect(c, (vs_rect){ (int)x, (int)(y + th_h), (int)w, 1 }, th->divider);
    vs_rgba ink = active ? th->caption_ink : th->caption_ink_inactive;
    vs_glyph_draw(c, dock_glyphs[wn->app], x + 14 * k, y + (th_h - 18 * k) * .5f, 18 * k, active ? th->accent : ink);
    vs_text(c, VS_FONT_UI_BOLD, 13 * t, x + 40 * k, y + th_h * .5f + 4.5f * t, wn->title, active ? th->text : th->text_muted);
    static const vs_glyph cap[3] = { VS_G_MIN, VS_G_MAX, VS_G_CLOSE };
    for (int b = 0; b < 3; b++) {
        vs_rect r = vs_shell_caption_rect(s, i, b);
        float gsz = 14 * k;
        vs_glyph_draw(c, cap[b], r.x + (r.w - gsz) * .5f, r.y + (r.h - gsz) * .5f, gsz, ink);
    }
    vs_rect body = { (int)x, (int)(y + th_h + 1), (int)w, (int)(h - th_h - 1) };
    vs_set_clip(&body);
    float bx = x, by = y + th_h + 1, bw = w, bh = h - th_h - 1;
    switch (wn->app) {
    case APP_VELA: content_vela(s, c, th, bx, by, bw, bh); break;
    case APP_FIND: content_find(s, c, th, bx, by, bw, bh); break;
    case APP_NOTES: content_notes(s, c, th, bx, by, bw, bh); break;
    case APP_MEDIA: content_media(s, c, th, bx, by, bw, bh); break;
    case APP_SETTINGS: content_settings(s, c, th, i, bx, by, bw, bh); break;
    }
    vs_set_clip(NULL);
    if (rad > 0) vs_stroke_rrect(c, x, y, w, h, rad, 1.f, s->theme == VS_THEME_DEEP ? (vs_rgba){ 1, 1, 1, .08f } : (vs_rgba){ 0, 0, 0, .06f });
}

void vs_shell_render(vs_shell *s, vs_canvas *out) {
    if (!s->wallpaper_valid) { vs_wallpaper_render(&s->wallpaper, s->theme, s->space); s->wallpaper_valid = 1; }
    vs_copy(out, &s->wallpaper);
    for (int j = 0; j < s->nz; j++) {
        int i = s->zorder[j];
        if (!s->win[i].used || s->win[i].minimized || s->win[i].space != s->space) continue;
        if (s->focus_mode && i == s->focused) continue; /* drawn last, full screen */
        draw_window(s, out, i);
    }
    if (s->focus_mode && s->focused >= 0) draw_window(s, out, s->focused);
    else { draw_bar(s, out); draw_dock(s, out); }
    draw_notification(s, out);
    if (s->draw_cursor) vs_glyph_draw(out, VS_G_CURSOR, s->cursor_x, s->cursor_y, 22 * S(s), (vs_rgba){ 1, 1, 1, 1 });
}
