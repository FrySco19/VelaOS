/* Software rasteriser: anti-aliased SDF shapes, soft shadows, backdrop blur. */
#include "vs.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>

static inline float clampf(float v, float a, float b) { return v < a ? a : (v > b ? b : v); }
static inline int imin(int a, int b) { return a < b ? a : b; }
static inline int imax(int a, int b) { return a > b ? a : b; }

int vs_canvas_init(vs_canvas *c, int w, int h) {
    c->w = w; c->h = h; c->stride = w;
    c->px = calloc((size_t)w * h, 4);
    return c->px ? 0 : -1;
}
void vs_canvas_free(vs_canvas *c) { free(c->px); c->px = NULL; }
void vs_copy(vs_canvas *dst, const vs_canvas *src) {
    memcpy(dst->px, src->px, (size_t)src->w * src->h * 4);
}

static int clip_on, clip_x0, clip_y0, clip_x1, clip_y1;
void vs_set_clip(const vs_rect *r) {
    if (!r) { clip_on = 0; return; }
    clip_on = 1; clip_x0 = r->x; clip_y0 = r->y; clip_x1 = r->x + r->w; clip_y1 = r->y + r->h;
}

void vs_blend_px(vs_canvas *c, int x, int y, vs_rgba col, float cov) {
    if ((unsigned)x >= (unsigned)c->w || (unsigned)y >= (unsigned)c->h) return;
    if (clip_on && (x < clip_x0 || y < clip_y0 || x >= clip_x1 || y >= clip_y1)) return;
    float a = col.a * cov;
    if (a <= 0.002f) return;
    uint32_t *p = &c->px[y * c->stride + x];
    uint32_t d = *p;
    float dr = ((d >> 16) & 255), dg = ((d >> 8) & 255), db = (d & 255);
    float r = dr + (col.r * 255.f - dr) * a;
    float g = dg + (col.g * 255.f - dg) * a;
    float b = db + (col.b * 255.f - db) * a;
    *p = 0xff000000u | ((uint32_t)(r + .5f) << 16) | ((uint32_t)(g + .5f) << 8) | (uint32_t)(b + .5f);
}

void vs_fill_rect(vs_canvas *c, vs_rect r, vs_rgba col) {
    int x0 = imax(r.x, 0), y0 = imax(r.y, 0), x1 = imin(r.x + r.w, c->w), y1 = imin(r.y + r.h, c->h);
    for (int y = y0; y < y1; y++) for (int x = x0; x < x1; x++) vs_blend_px(c, x, y, col, 1.f);
}

/* signed distance to a rounded box; (px,py) relative to box centre, (hx,hy) half extents */
static inline float sd_rrect(float px, float py, float hx, float hy, float r) {
    float qx = fabsf(px) - hx + r, qy = fabsf(py) - hy + r;
    float ox = qx > 0 ? qx : 0, oy = qy > 0 ? qy : 0;
    float inside = fminf(fmaxf(qx, qy), 0.f);
    return sqrtf(ox * ox + oy * oy) + inside - r;
}

void vs_fill_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, vs_rgba col) {
    float cx = x + w * .5f, cy = y + h * .5f;
    rad = fminf(rad, fminf(w, h) * .5f);
    int x0 = imax((int)floorf(x) - 1, 0), y0 = imax((int)floorf(y) - 1, 0);
    int x1 = imin((int)ceilf(x + w) + 1, c->w), y1 = imin((int)ceilf(y + h) + 1, c->h);
    for (int py = y0; py < y1; py++) {
        float fy = py + .5f - cy;
        for (int px = x0; px < x1; px++) {
            float d = sd_rrect(px + .5f - cx, fy, w * .5f, h * .5f, rad);
            float cov = clampf(.5f - d, 0.f, 1.f);
            if (cov > 0) vs_blend_px(c, px, py, col, cov);
        }
    }
}

void vs_stroke_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, float lw, vs_rgba col) {
    float cx = x + w * .5f, cy = y + h * .5f;
    rad = fminf(rad, fminf(w, h) * .5f);
    int x0 = imax((int)floorf(x) - 2, 0), y0 = imax((int)floorf(y) - 2, 0);
    int x1 = imin((int)ceilf(x + w) + 2, c->w), y1 = imin((int)ceilf(y + h) + 2, c->h);
    for (int py = y0; py < y1; py++) {
        float fy = py + .5f - cy;
        for (int px = x0; px < x1; px++) {
            float d = sd_rrect(px + .5f - cx, fy, w * .5f, h * .5f, rad);
            /* inner stroke: band [-lw, 0] */
            float cov = clampf(.5f - fabsf(d + lw * .5f) + lw * .5f, 0.f, 1.f);
            if (cov > 0) vs_blend_px(c, px, py, col, cov);
        }
    }
}

void vs_shadow_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, float sigma, float dy, float alpha) {
    float cx = x + w * .5f, cy = y + h * .5f + dy;
    float m = sigma * 3.f;
    int x0 = imax((int)(x - m), 0), y0 = imax((int)(y + dy - m), 0);
    int x1 = imin((int)(x + w + m), c->w), y1 = imin((int)(y + h + dy + m), c->h);
    vs_rgba k = { 0, 0, 0, alpha };
    float inv = 1.f / (sigma * sigma * 2.f);
    for (int py = y0; py < y1; py++) {
        float fy = py + .5f - cy;
        for (int px = x0; px < x1; px++) {
            float d = sd_rrect(px + .5f - cx, fy, w * .5f, h * .5f, rad);
            if (d < -sigma) continue; /* hidden under the surface */
            float dd = d > 0 ? d : 0;
            float a = expf(-dd * dd * inv);
            vs_blend_px(c, px, py, k, a);
        }
    }
}

static float seg_dist(float px, float py, float x0, float y0, float x1, float y1) {
    float vx = x1 - x0, vy = y1 - y0, wx = px - x0, wy = py - y0;
    float l2 = vx * vx + vy * vy;
    float t = l2 > 0 ? clampf((wx * vx + wy * vy) / l2, 0.f, 1.f) : 0.f;
    float dx = wx - t * vx, dy = wy - t * vy;
    return sqrtf(dx * dx + dy * dy);
}

void vs_line(vs_canvas *c, float x0, float y0, float x1, float y1, float lw, vs_rgba col) {
    float hw = lw * .5f;
    int bx0 = imax((int)floorf(fminf(x0, x1) - hw - 1), 0), by0 = imax((int)floorf(fminf(y0, y1) - hw - 1), 0);
    int bx1 = imin((int)ceilf(fmaxf(x0, x1) + hw + 1), c->w), by1 = imin((int)ceilf(fmaxf(y0, y1) + hw + 1), c->h);
    for (int py = by0; py < by1; py++)
        for (int px = bx0; px < bx1; px++) {
            float d = seg_dist(px + .5f, py + .5f, x0, y0, x1, y1) - hw;
            float cov = clampf(.5f - d, 0.f, 1.f);
            if (cov > 0) vs_blend_px(c, px, py, col, cov);
        }
}

void vs_arc(vs_canvas *c, float cx, float cy, float r, float lw, float a0, float a1, vs_rgba col) {
    float hw = lw * .5f;
    int x0 = imax((int)(cx - r - lw - 1), 0), y0 = imax((int)(cy - r - lw - 1), 0);
    int x1 = imin((int)(cx + r + lw + 2), c->w), y1 = imin((int)(cy + r + lw + 2), c->h);
    int full = (a1 - a0) >= 6.2831f;
    for (int py = y0; py < y1; py++)
        for (int px = x0; px < x1; px++) {
            float dx = px + .5f - cx, dy = py + .5f - cy;
            float d;
            if (full) d = fabsf(sqrtf(dx * dx + dy * dy) - r) - hw;
            else {
                float a = atan2f(dy, dx);
                while (a < a0) a += 6.2831853f;
                while (a > a0 + 6.2831853f) a -= 6.2831853f;
                if (a <= a1) d = fabsf(sqrtf(dx * dx + dy * dy) - r) - hw;
                else {
                    float ex0 = cx + r * cosf(a0), ey0 = cy + r * sinf(a0);
                    float ex1 = cx + r * cosf(a1), ey1 = cy + r * sinf(a1);
                    float d0 = sqrtf((px + .5f - ex0) * (px + .5f - ex0) + (py + .5f - ey0) * (py + .5f - ey0));
                    float d1 = sqrtf((px + .5f - ex1) * (px + .5f - ex1) + (py + .5f - ey1) * (py + .5f - ey1));
                    d = fminf(d0, d1) - hw;
                }
            }
            float cov = clampf(.5f - d, 0.f, 1.f);
            if (cov > 0) vs_blend_px(c, px, py, col, cov);
        }
}
void vs_ring(vs_canvas *c, float cx, float cy, float r, float lw, vs_rgba col) { vs_arc(c, cx, cy, r, lw, 0, 6.2832f, col); }

void vs_disc(vs_canvas *c, float cx, float cy, float r, vs_rgba col) {
    int x0 = imax((int)(cx - r - 1), 0), y0 = imax((int)(cy - r - 1), 0);
    int x1 = imin((int)(cx + r + 2), c->w), y1 = imin((int)(cy + r + 2), c->h);
    for (int py = y0; py < y1; py++)
        for (int px = x0; px < x1; px++) {
            float dx = px + .5f - cx, dy = py + .5f - cy;
            float cov = clampf(r - sqrtf(dx * dx + dy * dy) + .5f, 0.f, 1.f);
            if (cov > 0) vs_blend_px(c, px, py, col, cov);
        }
}

/* even-odd polygon fill with 4x4 supersampling */
void vs_fill_poly(vs_canvas *c, const float *xy, int n, vs_rgba col) {
    float minx = 1e9f, miny = 1e9f, maxx = -1e9f, maxy = -1e9f;
    for (int i = 0; i < n; i++) {
        minx = fminf(minx, xy[2 * i]); maxx = fmaxf(maxx, xy[2 * i]);
        miny = fminf(miny, xy[2 * i + 1]); maxy = fmaxf(maxy, xy[2 * i + 1]);
    }
    int x0 = imax((int)floorf(minx), 0), y0 = imax((int)floorf(miny), 0);
    int x1 = imin((int)ceilf(maxx) + 1, c->w), y1 = imin((int)ceilf(maxy) + 1, c->h);
    for (int py = y0; py < y1; py++)
        for (int px = x0; px < x1; px++) {
            int hits = 0;
            for (int sy = 0; sy < 4; sy++)
                for (int sx = 0; sx < 4; sx++) {
                    float tx = px + (sx + .5f) / 4.f, ty = py + (sy + .5f) / 4.f;
                    int in = 0;
                    for (int i = 0, j = n - 1; i < n; j = i++) {
                        float xi = xy[2 * i], yi = xy[2 * i + 1], xj = xy[2 * j], yj = xy[2 * j + 1];
                        if (((yi > ty) != (yj > ty)) && (tx < (xj - xi) * (ty - yi) / (yj - yi) + xi)) in = !in;
                    }
                    hits += in;
                }
            if (hits) vs_blend_px(c, px, py, col, hits / 16.f);
        }
}

/* three-pass separable box blur ~= gaussian; operates in place on a region */
static void box_pass(uint32_t *src, uint32_t *dst, int w, int h, int r, int horiz) {
    int n = horiz ? w : h, lines = horiz ? h : w;
    for (int l = 0; l < lines; l++) {
        int sr = 0, sg = 0, sb = 0;
        #define AT(i) (horiz ? src[l * w + (i)] : src[(i) * w + l])
        #define PUT(i, v) do { if (horiz) dst[l * w + (i)] = (v); else dst[(i) * w + l] = (v); } while (0)
        for (int i = -r; i <= r; i++) {
            uint32_t p = AT(imin(imax(i, 0), n - 1));
            sr += (p >> 16) & 255; sg += (p >> 8) & 255; sb += p & 255;
        }
        int div = 2 * r + 1;
        for (int i = 0; i < n; i++) {
            PUT(i, 0xff000000u | ((uint32_t)(sr / div) << 16) | ((uint32_t)(sg / div) << 8) | (uint32_t)(sb / div));
            uint32_t pa = AT(imin(i + r + 1, n - 1)), pr = AT(imax(i - r, 0));
            sr += (int)((pa >> 16) & 255) - (int)((pr >> 16) & 255);
            sg += (int)((pa >> 8) & 255) - (int)((pr >> 8) & 255);
            sb += (int)(pa & 255) - (int)(pr & 255);
        }
        #undef AT
        #undef PUT
    }
}

void vs_blur_region(vs_canvas *c, vs_rect r, int radius) {
    int x0 = imax(r.x, 0), y0 = imax(r.y, 0), x1 = imin(r.x + r.w, c->w), y1 = imin(r.y + r.h, c->h);
    int w = x1 - x0, h = y1 - y0;
    if (w <= 0 || h <= 0 || radius <= 0) return;
    uint32_t *a = malloc((size_t)w * h * 4), *b = malloc((size_t)w * h * 4);
    if (!a || !b) { free(a); free(b); return; }
    for (int y = 0; y < h; y++) memcpy(a + y * w, c->px + (y0 + y) * c->stride + x0, (size_t)w * 4);
    int rr = radius / 2 > 0 ? radius / 2 : 1;
    for (int k = 0; k < 3; k++) { box_pass(a, b, w, h, rr, 1); box_pass(b, a, w, h, rr, 0); }
    for (int y = 0; y < h; y++) memcpy(c->px + (y0 + y) * c->stride + x0, a + y * w, (size_t)w * 4);
    free(a); free(b);
}

/* frosted glass: blur backdrop (caller), then tint + frost inside a rounded rect */
void vs_tint_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, vs_rgba tint, float frost) {
    vs_rgba white = { 1, 1, 1, frost };
    if (frost > 0) vs_fill_rrect(c, x, y, w, h, rad, white);
    vs_fill_rrect(c, x, y, w, h, rad, tint);
}

/* Backdrop blur clipped to a rounded rect (anti-aliased edge). This is the
 * compositor's glass material: whatever is beneath (wallpaper or windows) is blurred. */
void vs_backdrop_blur_rrect(vs_canvas *c, float x, float y, float w, float h, float rad, int radius) {
    int pad = radius * 2;
    int rx0 = imax((int)floorf(x) - pad, 0), ry0 = imax((int)floorf(y) - pad, 0);
    int rx1 = imin((int)ceilf(x + w) + pad, c->w), ry1 = imin((int)ceilf(y + h) + pad, c->h);
    int bw = rx1 - rx0, bh = ry1 - ry0;
    if (bw <= 0 || bh <= 0) return;
    vs_canvas t = { bw, bh, bw, malloc((size_t)bw * bh * 4) };
    if (!t.px) return;
    for (int yy = 0; yy < bh; yy++) memcpy(t.px + yy * bw, c->px + (ry0 + yy) * c->stride + rx0, (size_t)bw * 4);
    vs_rect all = { 0, 0, bw, bh };
    vs_blur_region(&t, all, radius);
    float cx = x + w * .5f, cy = y + h * .5f;
    rad = fminf(rad, fminf(w, h) * .5f);
    for (int py = imax((int)y - 1, 0); py < imin((int)(y + h) + 2, c->h); py++)
        for (int px = imax((int)x - 1, 0); px < imin((int)(x + w) + 2, c->w); px++) {
            float d = sd_rrect(px + .5f - cx, py + .5f - cy, w * .5f, h * .5f, rad);
            float cov = clampf(.5f - d, 0.f, 1.f);
            if (cov <= 0) continue;
            uint32_t s = t.px[(py - ry0) * bw + (px - rx0)];
            vs_rgba col = { ((s >> 16) & 255) / 255.f, ((s >> 8) & 255) / 255.f, (s & 255) / 255.f, 1.f };
            vs_blend_px(c, px, py, col, cov);
        }
    free(t.px);
}
