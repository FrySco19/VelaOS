/* VelaOS shell (OS-0) - shared types.
 * Software compositor core: draws into a 32-bit XRGB canvas.
 * Backends (headless PNG, Linux fbdev+evdev) live in src/main_*.c. */
#ifndef VS_H
#define VS_H
#include <stdint.h>

typedef struct { int w, h, stride; uint32_t *px; } vs_canvas;
typedef struct { int x, y, w, h; } vs_rect;
typedef struct { float r, g, b, a; } vs_rgba; /* linear-ish 0..1, a = opacity */

static inline vs_rgba vs_hex(uint32_t hex, float a) {
    vs_rgba c = { ((hex >> 16) & 255) / 255.f, ((hex >> 8) & 255) / 255.f, (hex & 255) / 255.f, a };
    return c;
}

/* ---- gfx.c ---- */
int  vs_canvas_init(vs_canvas *c, int w, int h);
void vs_canvas_free(vs_canvas *c);
void vs_set_clip(const vs_rect *r); /* NULL = no clip */
void vs_blend_px(vs_canvas *c, int x, int y, vs_rgba col, float cov);
void vs_fill_rect(vs_canvas *c, vs_rect r, vs_rgba col);
void vs_fill_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, vs_rgba col);
void vs_stroke_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, float lw, vs_rgba col);
void vs_shadow_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, float sigma, float dy, float alpha);
void vs_line(vs_canvas *c, float x0, float y0, float x1, float y1, float lw, vs_rgba col);
void vs_ring(vs_canvas *c, float cx, float cy, float r, float lw, vs_rgba col);
void vs_arc(vs_canvas *c, float cx, float cy, float r, float lw, float a0, float a1, vs_rgba col);
void vs_disc(vs_canvas *c, float cx, float cy, float r, vs_rgba col);
void vs_fill_poly(vs_canvas *c, const float *xy, int n, vs_rgba col);
void vs_blur_region(vs_canvas *c, vs_rect r, int radius);
void vs_copy(vs_canvas *dst, const vs_canvas *src);
void vs_tint_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, vs_rgba tint, float frost);
void vs_backdrop_blur_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, int radius);

/* ---- text.c ---- */
enum { VS_FONT_UI = 0, VS_FONT_UI_BOLD = 1, VS_FONT_SERIF = 2, VS_FONT_COUNT };
int   vs_text_init(const char *font_dir);
float vs_text(vs_canvas *c, int font, float px, float x, float baseline, const char *utf8, vs_rgba col);
float vs_text_width(int font, float px, const char *utf8);

/* ---- theme.c ---- */
typedef enum { VS_THEME_LIGHT = 0, VS_THEME_DAWN = 1, VS_THEME_DEEP = 2, VS_THEME_COUNT } vs_theme_id;
typedef struct {
    const char *name;
    vs_rgba bar_tint, dock_tint, notif_tint;
    float glass_frost;               /* white mix into frosted glass 0..1 */
    vs_rgba glass_stroke;
    vs_rgba on_glass, on_glass_muted;
    vs_rgba window, sidebar, divider, title_area;
    vs_rgba text, text_muted, caption_ink, caption_ink_inactive;
    vs_rgba accent, accent_bright, accent_tint;
    vs_rgba active_tile;             /* dock active: frosted light tile */
    vs_rgba notif_text, notif_text_muted;
    float shadow_alpha;
} vs_theme;
const vs_theme *vs_theme_get(vs_theme_id id);

/* ---- wallpaper.c ---- */
void vs_wallpaper_render(vs_canvas *c, vs_theme_id theme, int space);

/* ---- glyphs.c : monochrome line glyphs, drawn in a size x size box ---- */
typedef enum {
    VS_G_SAIL, VS_G_SEARCH, VS_G_SPACES, VS_G_NOTES, VS_G_MEDIA, VS_G_SETTINGS,
    VS_G_WIFI, VS_G_VOLUME, VS_G_BATTERY, VS_G_MOON, VS_G_SUN,
    VS_G_MIN, VS_G_MAX, VS_G_CLOSE, VS_G_CURSOR
} vs_glyph;
void vs_glyph_draw(vs_canvas *c, vs_glyph g, float x, float y, float size, vs_rgba col);

/* ---- shell.c ---- */
#define VS_MAX_WINDOWS 16
#define VS_SPACES 3
#define VS_DOCK_ITEMS 6

typedef struct {
    int id, used, space, app;        /* app = dock index */
    char title[48];
    float x, y, w, h;
    int minimized, maximized;
} vs_window;

typedef struct {
    int used; char title[64]; char body[96]; int app;
} vs_notification;

typedef enum { VS_EV_MOTION, VS_EV_DOWN, VS_EV_UP, VS_EV_KEY } vs_ev_type;
enum { VS_KEY_F11 = 1, VS_KEY_SPACE_LEFT, VS_KEY_SPACE_RIGHT, VS_KEY_THEME_NEXT,
       VS_KEY_THEME_LIGHT, VS_KEY_THEME_DAWN, VS_KEY_THEME_DEEP, VS_KEY_ESC, VS_KEY_NOTIFY,
       VS_KEY_REDUCE_MOTION, VS_KEY_LARGER_TEXT };
typedef struct { vs_ev_type type; float x, y; int key; } vs_event;

typedef struct {
    int w, h;
    vs_theme_id theme;
    int space;                       /* 0..VS_SPACES-1, virtual desktops */
    vs_window win[VS_MAX_WINDOWS];
    int zorder[VS_MAX_WINDOWS];      /* indices into win, back -> front */
    int nz;
    int focused;                     /* index into win or -1 */
    int focus_mode;                  /* Focus mode: hides bar + dock */
    int reduce_motion, larger_text;
    vs_notification notif;
    float cursor_x, cursor_y;
    int dragging; float drag_dx, drag_dy;
    int hover_dock;
    long long now_unix;              /* injected clock (seconds) */
    int wallpaper_valid;
    vs_canvas wallpaper;             /* cached full-screen wallpaper */
    int draw_cursor;
} vs_shell;

int  vs_shell_init(vs_shell *s, int w, int h, const char *font_dir);
void vs_shell_seed_demo(vs_shell *s);
void vs_shell_set_theme(vs_shell *s, vs_theme_id t);
int  vs_shell_event(vs_shell *s, const vs_event *ev); /* returns 1 if redraw needed */
void vs_shell_render(vs_shell *s, vs_canvas *out);
int  vs_shell_open_app(vs_shell *s, int app);
void vs_shell_notify(vs_shell *s, const char *title, const char *body, int app);
vs_rect vs_shell_dock_item_rect(const vs_shell *s, int i);
vs_rect vs_shell_caption_rect(const vs_shell *s, int win, int which); /* 0 min 1 max 2 close */
extern const char *vs_dock_names[VS_DOCK_ITEMS];
#endif
