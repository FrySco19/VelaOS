/* Procedural signature wallpapers (brief 4.8): night sea + glowing sail (Deep),
 * golden-hour sea (Dawn), clear morning coast (Light). Rendered once per theme/space
 * and cached by the shell; no image files needed in the guest. */
#include "vs.h"
#include <math.h>
#include <stdlib.h>

typedef struct { float r, g, b; } c3;
static c3 mix3(c3 a, c3 b, float t) { c3 o = { a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t }; return o; }
static c3 hx(uint32_t h) { c3 o = { ((h >> 16) & 255) / 255.f, ((h >> 8) & 255) / 255.f, (h & 255) / 255.f }; return o; }
static float clamp01(float v) { return v < 0 ? 0 : v > 1 ? 1 : v; }
static float smooth(float a, float b, float x) { float t = clamp01((x - a) / (b - a)); return t * t * (3 - 2 * t); }

static float hash2(int x, int y) {
    unsigned h = (unsigned)x * 374761393u + (unsigned)y * 668265263u;
    h = (h ^ (h >> 13)) * 1274126177u;
    return ((h ^ (h >> 16)) & 0xffffff) / 16777215.f;
}
static float vnoise(float x, float y) {
    int xi = (int)floorf(x), yi = (int)floorf(y);
    float fx = x - xi, fy = y - yi;
    fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy);
    float a = hash2(xi, yi), b = hash2(xi + 1, yi), c = hash2(xi, yi + 1), d = hash2(xi + 1, yi + 1);
    return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}
static float fbm(float x, float y) { return vnoise(x, y) * .55f + vnoise(x * 2.1f, y * 2.1f) * .28f + vnoise(x * 4.3f, y * 4.3f) * .17f; }

typedef struct { c3 sky_top, sky_mid, horizon, sea_near, sea_far, glow, rock; float glow_k, stars, sail_bright; } palette;

static const palette pals[VS_THEME_COUNT] = {
    [VS_THEME_LIGHT] = { {0.55f,0.70f,0.82f}, {0.76f,0.85f,0.90f}, {0.93f,0.94f,0.92f}, {0.20f,0.36f,0.45f}, {0.47f,0.64f,0.72f}, {1.0f,0.98f,0.93f}, {0.17f,0.20f,0.23f}, .35f, 0.f, .9f },
    [VS_THEME_DAWN]  = { {0.36f,0.43f,0.62f}, {0.86f,0.62f,0.55f}, {1.00f,0.83f,0.60f}, {0.17f,0.20f,0.31f}, {0.55f,0.47f,0.50f}, {1.0f,0.86f,0.62f}, {0.13f,0.11f,0.15f}, .55f, 0.f, .8f },
    [VS_THEME_DEEP]  = { {0.012f,0.03f,0.07f}, {0.03f,0.08f,0.15f}, {0.08f,0.20f,0.29f}, {0.01f,0.03f,0.06f}, {0.04f,0.11f,0.17f}, {0.80f,0.95f,1.0f}, {0.01f,0.015f,0.025f}, .75f, 1.f, 1.f },
};

void vs_wallpaper_render(vs_canvas *c, vs_theme_id theme, int space) {
    const palette *p = &pals[(unsigned)theme < VS_THEME_COUNT ? theme : 0];
    int W = c->w, H = c->h;
    float hz = H * 0.63f;                         /* horizon line */
    float sail_x = W * (0.70f + 0.06f * (space % 3) - 0.06f); /* each Space nudges the sail */
    float sail_base = hz + H * 0.015f, sail_top = H * 0.24f;
    float sail_w = W * 0.085f;

    for (int y = 0; y < H; y++) {
        for (int x = 0; x < W; x++) {
            c3 col;
            float fx = (float)x / W, fy = (float)y / H;
            float dxs = (x - sail_x) / (float)W, dys = (y - (sail_top + sail_base) * .5f) / (float)H;
            float glow = expf(-(dxs * dxs * 9.f + dys * dys * 5.f) * 6.f) * p->glow_k;
            if (y < hz) {
                float t = (float)y / hz;
                col = t < .55f ? mix3(p->sky_top, p->sky_mid, smooth(0, .55f, t)) : mix3(p->sky_mid, p->horizon, smooth(.55f, 1.f, t));
                /* soft clouds */
                float cl = fbm(fx * 3.f + space * 7.f, fy * 9.f) ;
                col = mix3(col, p->horizon, clamp01((cl - .55f) * 1.2f) * .25f);
                /* stars (Deep only) */
                if (p->stars > 0 && t < .8f) {
                    float s = hash2(x, y);
                    if (s > .9993f) { float k = (s - .9993f) / .0007f * (1 - t) ; col = mix3(col, hx(0xffffff), clamp01(k)); }
                }
            } else {
                float t = (y - hz) / (H - hz);
                col = mix3(p->sea_far, p->sea_near, smooth(0, 1, powf(t, .6f)));
                /* waves: horizontal shimmer stretched in x */
                float wv = fbm(fx * 6.f + space, (y - hz) * .09f / (1.f + t * 3.f));
                col = mix3(col, p->horizon, clamp01((wv - .5f) * 1.6f) * (.18f * (1 - t) + .04f));
                /* reflection streak below the sail */
                float rx = (x - sail_x) / (W * (0.018f + 0.05f * t));
                float streak = expf(-rx * rx) * (1.f - t * .7f);
                float brk = clamp01(vnoise(x * .05f, y * .55f) * 1.8f - .5f);
                col = mix3(col, p->glow, clamp01(streak * brk * .85f * p->sail_bright));
            }
            col = mix3(col, p->glow, clamp01(glow * .55f));
            /* rocky shore on the left */
            float rock_top = hz + H * 0.02f - H * (0.20f * expf(-fx * 5.5f)) + (fbm(fx * 14.f, 3.3f) - .5f) * H * .05f
                           + smooth(.18f, .44f, fx) * H * .45f;
            if (fx < .46f && y > rock_top) {
                float k = smooth(0, 3.f, y - rock_top);
                c3 r = mix3(p->rock, p->sea_near, (fbm(fx * 30.f, fy * 30.f)) * .25f);
                col = mix3(col, r, k);
            }
            /* gentle vignette */
            float vx = fx - .5f, vy = fy - .5f;
            float v = 1.f - (vx * vx + vy * vy) * .55f;
            col.r *= v; col.g *= v; col.b *= v;
            uint32_t R = (uint32_t)(clamp01(col.r) * 255.f + .5f), G = (uint32_t)(clamp01(col.g) * 255.f + .5f), B = (uint32_t)(clamp01(col.b) * 255.f + .5f);
            c->px[y * c->stride + x] = 0xff000000u | (R << 16) | (G << 8) | B;
        }
    }
    /* the sail: luminous main sail + small jib, hull line, then bloom */
    vs_rgba sail = { 1.f, 1.f, 1.f, .96f };
    if (theme == VS_THEME_DAWN) sail = (vs_rgba){ 1.f, .95f, .86f, .95f };
    float m[] = {
        sail_x, sail_top,
        sail_x + sail_w * .10f, sail_top + (sail_base - sail_top) * .25f,
        sail_x + sail_w * .38f, sail_top + (sail_base - sail_top) * .62f,
        sail_x + sail_w * .62f, sail_base - H * .025f,
        sail_x, sail_base - H * .025f,
    };
    float jib[] = {
        sail_x - W * .004f, sail_top + H * .05f,
        sail_x - W * .004f, sail_base - H * .028f,
        sail_x - sail_w * .42f, sail_base - H * .028f,
    };
    vs_canvas glow = { 0 };
    if (vs_canvas_init(&glow, W, H) == 0) {
        vs_fill_poly(&glow, m, 5, sail); vs_fill_poly(&glow, jib, 3, sail);
        vs_rect r = { (int)(sail_x - sail_w), (int)(sail_top - H * .08f), (int)(sail_w * 2.2f), (int)(sail_base - sail_top + H * .16f) };
        vs_blur_region(&glow, r, (int)(W * .018f));
        for (int y = r.y < 0 ? 0 : r.y; y < r.y + r.h && y < H; y++)
            for (int x = r.x < 0 ? 0 : r.x; x < r.x + r.w && x < W; x++) {
                uint32_t g = glow.px[y * W + x], d = c->px[y * c->stride + x];
                int add = (int)(((g >> 8) & 255) * (.9f * p->sail_bright));
                int R = ((d >> 16) & 255) + add * 9 / 10, G = ((d >> 8) & 255) + add, B = (d & 255) + add;
                c->px[y * c->stride + x] = 0xff000000u | ((uint32_t)(R > 255 ? 255 : R) << 16) | ((uint32_t)(G > 255 ? 255 : G) << 8) | (uint32_t)(B > 255 ? 255 : B);
            }
        vs_canvas_free(&glow);
    }
    vs_fill_poly(c, m, 5, sail);
    vs_fill_poly(c, jib, 3, (vs_rgba){ sail.r, sail.g, sail.b, .85f });
    vs_line(c, sail_x, sail_top - H * .01f, sail_x, sail_base - H * .01f, 2.f, (vs_rgba){ .9f, .95f, 1.f, .7f });
    float hull[] = { sail_x - sail_w * .55f, sail_base - H * .022f, sail_x + sail_w * .72f, sail_base - H * .022f,
                     sail_x + sail_w * .55f, sail_base, sail_x - sail_w * .45f, sail_base };
    vs_fill_poly(c, hull, 4, (vs_rgba){ .05f, .08f, .12f, .9f });
}
