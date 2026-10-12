/* Text rendering with stb_truetype; glyph bitmaps cached per (font, size, codepoint). */
#define STB_TRUETYPE_IMPLEMENTATION
#include "third_party/stb_truetype.h"
#include "vs.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

static const char *font_files[VS_FONT_COUNT] = { "Inter-Regular.otf", "Inter-SemiBold.otf", "DejaVuSerif.ttf" };
static stbtt_fontinfo fonts[VS_FONT_COUNT];
static unsigned char *font_data[VS_FONT_COUNT];
static int fonts_ok[VS_FONT_COUNT];

typedef struct { int used, font, size10, cp, w, h, xoff, yoff; float adv; unsigned char *bmp; } glyph_e;
#define CACHE 8192
static glyph_e cache[CACHE];

static unsigned char *slurp(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
    unsigned char *b = malloc((size_t)n);
    if (b && fread(b, 1, (size_t)n, f) != (size_t)n) { free(b); b = NULL; }
    fclose(f);
    return b;
}

int vs_text_init(const char *dir) {
    int ok = 0;
    for (int i = 0; i < VS_FONT_COUNT; i++) {
        char p[512];
        snprintf(p, sizeof p, "%s/%s", dir, font_files[i]);
        font_data[i] = slurp(p);
        if (font_data[i] && stbtt_InitFont(&fonts[i], font_data[i], stbtt_GetFontOffsetForIndex(font_data[i], 0))) {
            fonts_ok[i] = 1; ok++;
        } else fprintf(stderr, "velashell: font missing: %s\n", p);
    }
    /* fall back to the regular face for any missing one */
    for (int i = 0; i < VS_FONT_COUNT; i++)
        if (!fonts_ok[i] && fonts_ok[0]) { fonts[i] = fonts[0]; fonts_ok[i] = 1; }
    return ok ? 0 : -1;
}

static int utf8_next(const char **s) {
    const unsigned char *p = (const unsigned char *)*s;
    int cp;
    if (p[0] < 0x80) { cp = p[0]; *s += 1; }
    else if ((p[0] & 0xe0) == 0xc0) { cp = ((p[0] & 0x1f) << 6) | (p[1] & 0x3f); *s += 2; }
    else if ((p[0] & 0xf0) == 0xe0) { cp = ((p[0] & 0x0f) << 12) | ((p[1] & 0x3f) << 6) | (p[2] & 0x3f); *s += 3; }
    else { cp = ((p[0] & 0x07) << 18) | ((p[1] & 0x3f) << 12) | ((p[2] & 0x3f) << 6) | (p[3] & 0x3f); *s += 4; }
    return cp;
}

static glyph_e *get_glyph(int font, float px, int cp) {
    int s10 = (int)(px * 10 + .5f);
    unsigned h = (unsigned)(font * 2654435761u) ^ (unsigned)(s10 * 40503u) ^ (unsigned)(cp * 2246822519u);
    for (int probe = 0; probe < CACHE; probe++) {
        glyph_e *e = &cache[(h + probe) & (CACHE - 1)];
        if (e->used && e->font == font && e->size10 == s10 && e->cp == cp) return e;
        if (!e->used) {
            stbtt_fontinfo *f = &fonts[font];
            float sc = stbtt_ScaleForMappingEmToPixels(f, px);
            int adv, lsb;
            stbtt_GetCodepointHMetrics(f, cp, &adv, &lsb);
            e->used = 1; e->font = font; e->size10 = s10; e->cp = cp;
            e->adv = adv * sc;
            e->bmp = stbtt_GetCodepointBitmap(f, sc, sc, cp, &e->w, &e->h, &e->xoff, &e->yoff);
            return e;
        }
    }
    return NULL;
}

float vs_text_width(int font, float px, const char *s) {
    if (!fonts_ok[font]) return 0;
    float x = 0;
    int prev = 0;
    float sc = stbtt_ScaleForMappingEmToPixels(&fonts[font], px);
    while (*s) {
        int cp = utf8_next(&s);
        if (prev) x += stbtt_GetCodepointKernAdvance(&fonts[font], prev, cp) * sc;
        glyph_e *g = get_glyph(font, px, cp);
        if (g) x += g->adv;
        prev = cp;
    }
    return x;
}

float vs_text(vs_canvas *c, int font, float px, float x, float baseline, const char *s, vs_rgba col) {
    if (!fonts_ok[font]) return x;
    int prev = 0;
    float sc = stbtt_ScaleForMappingEmToPixels(&fonts[font], px);
    while (*s) {
        int cp = utf8_next(&s);
        if (prev) x += stbtt_GetCodepointKernAdvance(&fonts[font], prev, cp) * sc;
        glyph_e *g = get_glyph(font, px, cp);
        if (!g) continue;
        int gx = (int)floorf(x + .5f) + g->xoff, gy = (int)floorf(baseline + .5f) + g->yoff;
        for (int yy = 0; yy < g->h; yy++)
            for (int xx = 0; xx < g->w; xx++) {
                unsigned char a = g->bmp[yy * g->w + xx];
                if (a) vs_blend_px(c, gx + xx, gy + yy, col, a / 255.f);
            }
        x += g->adv;
        prev = cp;
    }
    return x;
}
