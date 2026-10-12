/* Monochrome line glyphs (brief 4.3: six white line glyphs; section 7 #2).
 * Drawn from geometry so they stay crisp at any scale and need no icon files. */
#include "vs.h"
#include <math.h>

#define PI 3.14159265f

void vs_glyph_draw(vs_canvas *c, vs_glyph g, float x, float y, float s, vs_rgba col) {
    float lw = s / 13.f; if (lw < 1.3f) lw = 1.3f;
    float cx = x + s * .5f, cy = y + s * .5f;
    #define P(px, py) (x + (px) * s), (y + (py) * s)
    switch (g) {
    case VS_G_SAIL: {
        /* sail outline: mast + curved main + jib + hull */
        vs_line(c, P(.52f, .10f), P(.52f, .74f), lw, col);
        vs_line(c, P(.52f, .10f), P(.62f, .30f), lw, col);
        vs_line(c, P(.62f, .30f), P(.74f, .54f), lw, col);
        vs_line(c, P(.74f, .54f), P(.80f, .74f), lw, col);
        vs_line(c, P(.80f, .74f), P(.52f, .74f), lw, col);
        vs_line(c, P(.46f, .22f), P(.22f, .70f), lw, col);
        vs_line(c, P(.22f, .70f), P(.46f, .70f), lw, col);
        vs_line(c, P(.46f, .70f), P(.46f, .22f), lw, col);
        vs_line(c, P(.16f, .84f), P(.86f, .84f), lw, col);
        break; }
    case VS_G_SEARCH:
        vs_ring(c, x + s * .44f, y + s * .44f, s * .24f, lw, col);
        vs_line(c, P(.62f, .62f), P(.82f, .82f), lw * 1.15f, col);
        break;
    case VS_G_SPACES:
        /* layered panes = virtual desktops */
        vs_stroke_rrect(c, x + s * .14f, y + s * .30f, s * .52f, s * .44f, s * .08f, lw, col);
        vs_line(c, P(.30f, .22f), P(.78f, .22f), lw, col);
        vs_line(c, P(.78f, .22f), P(.78f, .58f), lw, col);
        vs_line(c, P(.40f, .13f), P(.88f, .13f), lw * .9f, (vs_rgba){ col.r, col.g, col.b, col.a * .6f });
        vs_line(c, P(.88f, .13f), P(.88f, .46f), lw * .9f, (vs_rgba){ col.r, col.g, col.b, col.a * .6f });
        break;
    case VS_G_NOTES:
        vs_stroke_rrect(c, x + s * .22f, y + s * .12f, s * .56f, s * .76f, s * .08f, lw, col);
        vs_line(c, P(.34f, .34f), P(.66f, .34f), lw, col);
        vs_line(c, P(.34f, .50f), P(.66f, .50f), lw, col);
        vs_line(c, P(.34f, .66f), P(.54f, .66f), lw, col);
        break;
    case VS_G_MEDIA: {
        vs_ring(c, cx, cy, s * .36f, lw, col);
        float t[] = { P(.42f, .34f), P(.66f, .50f), P(.42f, .66f) };
        vs_fill_poly(c, t, 3, col);
        break; }
    case VS_G_SETTINGS:
        vs_ring(c, cx, cy, s * .14f, lw, col);
        vs_ring(c, cx, cy, s * .28f, lw, col);
        for (int i = 0; i < 8; i++) {
            float a = i * PI / 4;
            vs_line(c, cx + cosf(a) * s * .30f, cy + sinf(a) * s * .30f, cx + cosf(a) * s * .40f, cy + sinf(a) * s * .40f, lw * 1.5f, col);
        }
        break;
    case VS_G_WIFI:
        for (int i = 0; i < 3; i++)
            vs_arc(c, cx, y + s * .80f, s * (.18f + .18f * i), lw, -PI * .75f, -PI * .25f, col);
        vs_disc(c, cx, y + s * .80f, lw * .9f, col);
        break;
    case VS_G_VOLUME: {
        float b[] = { P(.14f, .40f), P(.30f, .40f), P(.50f, .22f), P(.50f, .78f), P(.30f, .60f), P(.14f, .60f) };
        vs_fill_poly(c, b, 6, col);
        vs_arc(c, x + s * .50f, cy, s * .16f, lw, -PI * .3f, PI * .3f, col);
        vs_arc(c, x + s * .50f, cy, s * .30f, lw, -PI * .3f, PI * .3f, col);
        break; }
    case VS_G_BATTERY:
        vs_stroke_rrect(c, x + s * .08f, y + s * .30f, s * .74f, s * .40f, s * .08f, lw, col);
        vs_fill_rrect(c, x + s * .84f, y + s * .42f, s * .07f, s * .16f, s * .03f, col);
        vs_fill_rrect(c, x + s * .14f, y + s * .36f, s * .50f, s * .28f, s * .04f, col);
        break;
    case VS_G_MOON:
        vs_arc(c, cx, cy, s * .30f, lw, PI * .25f, PI * 1.75f + .01f, col);
        vs_arc(c, x + s * .66f, y + s * .36f, s * .22f, lw, PI * .52f, PI * 1.25f, col);
        break;
    case VS_G_SUN:
        vs_ring(c, cx, cy, s * .17f, lw, col);
        for (int i = 0; i < 8; i++) {
            float a = i * PI / 4;
            vs_line(c, cx + cosf(a) * s * .28f, cy + sinf(a) * s * .28f, cx + cosf(a) * s * .38f, cy + sinf(a) * s * .38f, lw, col);
        }
        break;
    case VS_G_MIN:
        vs_line(c, P(.22f, .52f), P(.78f, .52f), lw, col);
        break;
    case VS_G_MAX:
        vs_stroke_rrect(c, x + s * .24f, y + s * .24f, s * .52f, s * .52f, s * .06f, lw, col);
        break;
    case VS_G_CLOSE:
        vs_line(c, P(.26f, .26f), P(.74f, .74f), lw, col);
        vs_line(c, P(.74f, .26f), P(.26f, .74f), lw, col);
        break;
    case VS_G_CURSOR: {
        float a[] = { x, y, x, y + s * .92f, x + s * .24f, y + s * .70f, x + s * .40f, y + s * 1.02f,
                      x + s * .54f, y + s * .96f, x + s * .38f, y + s * .66f, x + s * .70f, y + s * .66f };
        vs_fill_poly(c, a, 7, (vs_rgba){ .06f, .09f, .13f, .96f });
        float b[] = { x + 2, y + 4.5f, x + 2, y + s * .80f, x + s * .25f, y + s * .60f, x + s * .41f, y + s * .92f,
                      x + s * .48f, y + s * .89f, x + s * .33f, y + s * .58f, x + s * .58f, y + s * .58f };
        vs_fill_poly(c, b, 7, (vs_rgba){ 1, 1, 1, 1 });
        break; }
    }
    #undef P
}
