/* Theme tokens: Light / Dawn / Deep (brief 4.6, 4.7; section 7 decisions). */
#include "vs.h"

#define H(x, a) { ((x >> 16) & 255) / 255.f, ((x >> 8) & 255) / 255.f, (x & 255) / 255.f, a }

static const vs_theme themes[VS_THEME_COUNT] = {
    [VS_THEME_LIGHT] = {
        .name = "Light",
        .bar_tint = H(0x0E223B, .38f), .dock_tint = H(0x182532, .52f), .notif_tint = H(0xF7F4F0, .80f),
        .glass_frost = .10f, .glass_stroke = H(0xFFFFFF, .22f),
        .on_glass = H(0xFFFFFF, .96f), .on_glass_muted = H(0xFFFFFF, .70f),
        .window = H(0xF9F7F4, 1), .sidebar = H(0xF1EEEA, 1), .divider = H(0xE6E1DB, 1), .title_area = H(0xF4F1ED, 1),
        .text = H(0x14233A, 1), .text_muted = H(0x5E6A78, 1),
        .caption_ink = H(0x4A5563, .85f), .caption_ink_inactive = H(0x9AA3AD, .70f),
        .accent = H(0x2D6877, 1), .accent_bright = H(0x34919B, 1), .accent_tint = H(0xD9E3E0, 1),
        .active_tile = H(0xFFFFFF, .20f),
        .notif_text = H(0x14233A, 1), .notif_text_muted = H(0x4F5B69, 1),
        .shadow_alpha = .26f,
    },
    [VS_THEME_DAWN] = {
        .name = "Dawn",
        .bar_tint = H(0x3B2A3F, .30f), .dock_tint = H(0x2B2236, .46f), .notif_tint = H(0xFFF6EE, .78f),
        .glass_frost = .12f, .glass_stroke = H(0xFFF1E6, .28f),
        .on_glass = H(0xFFFFFF, .97f), .on_glass_muted = H(0xFFF3EA, .74f),
        .window = H(0xFCF6F0, 1), .sidebar = H(0xF5EBE1, 1), .divider = H(0xEADBCD, 1), .title_area = H(0xF8EFE6, 1),
        .text = H(0x2A2233, 1), .text_muted = H(0x6E6170, 1),
        .caption_ink = H(0x5A4B5C, .85f), .caption_ink_inactive = H(0xAE9FA6, .70f),
        .accent = H(0x2D6877, 1), .accent_bright = H(0x34919B, 1), .accent_tint = H(0xF2DCCB, 1),
        .active_tile = H(0xFFFFFF, .22f),
        .notif_text = H(0x2A2233, 1), .notif_text_muted = H(0x6E6170, 1),
        .shadow_alpha = .24f,
    },
    [VS_THEME_DEEP] = {
        .name = "Deep",
        .bar_tint = H(0x0B1A2C, .62f), .dock_tint = H(0x101D29, .66f), .notif_tint = H(0x04151C, .82f),
        .glass_frost = .05f, .glass_stroke = H(0xBFE6EC, .14f),
        .on_glass = H(0xEAF4F6, .96f), .on_glass_muted = H(0xBFD0D6, .70f),
        .window = H(0x0F181F, 1), .sidebar = H(0x131F28, 1), .divider = H(0x22313B, 1), .title_area = H(0x121C24, 1),
        .text = H(0xE6EEF2, 1), .text_muted = H(0x8EA0AB, 1),
        .caption_ink = H(0xB9C6CE, .85f), .caption_ink_inactive = H(0x5E6D77, .80f),
        .accent = H(0x34919B, 1), .accent_bright = H(0x45B3BD, 1), .accent_tint = H(0x183A42, 1),
        .active_tile = H(0xFFFFFF, .14f),
        .notif_text = H(0xEAF4F6, 1), .notif_text_muted = H(0x9DB2BA, 1),
        .shadow_alpha = .45f,
    },
};

const vs_theme *vs_theme_get(vs_theme_id id) {
    if ((unsigned)id >= VS_THEME_COUNT) id = VS_THEME_LIGHT;
    return &themes[id];
}
