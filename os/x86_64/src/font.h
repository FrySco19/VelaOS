#pragma once
typedef struct {
    int w, h;                      /* glyph cell size in pixels */
    const unsigned char *adv;      /* advance per glyph (ASCII 32..126) */
    const unsigned char *px;       /* 8-bit alpha, w*h per glyph */
} font_t;
extern const font_t font_mono, font_ui, font_bold, font_big;
