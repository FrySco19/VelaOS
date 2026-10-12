/* UI adapter to the Rust surface model. No network/audio/storage authority. */
extern uint32_t vela_glass_intent(const uint8_t *, size_t);
extern uint32_t vela_glass_transition(uint32_t, uint32_t);
static unsigned glass_kind;
static unsigned glass_retained_at;
static char glass_query[73]; static int glass_query_len;
static char retained_note[1200];
static struct hit glass_close, glass_primary, glass_secondary, glass_entry;
static void glass_open(unsigned kind) {
    glass_kind = vela_glass_transition(glass_kind, kind);
    focused = -1; focus_mode = 0;
    if (kind == 2 && !note_len && retained_note[0]) { kstrcpy(note_buf, retained_note, sizeof note_buf); note_len = (int)kstrlen(note_buf); }
    for (int a = 0; a < NAPPS; a++) win[a].open = 0;
    serial_puts("VelaOS Glass: surface opened\r\n");
}
static void glass_release(void) {
    glass_kind = vela_glass_transition(glass_kind, 0);
    memset(glass_query, 0, sizeof glass_query); glass_query_len = 0;
    memset(note_buf, 0, sizeof note_buf); note_len = 0;
    serial_puts("VelaOS Glass: surface released; temporary buffers cleared\r\n");
}
static void glass_button(struct hit *hit, int x, int y, int width, const char *label, int primary) {
    *hit = (struct hit){x, y, width, 40};
    fill_rrect(x, y, width, 40, 18, primary ? 0x34919B : 0xFFFFFF, primary ? 235 : 24);
    stroke_rrect(x, y, width, 40, 18, 0xBFE9EB, primary ? 150 : 60);
    draw_text(F_UI, x + (width - text_width(F_UI, label)) / 2, y + 10, label, 0xEDF5F7);
}
static void draw_glass(void) {
    if (glass_kind == 5) {
        unsigned now = now_secs();
        unsigned elapsed = now >= glass_retained_at ? now - glass_retained_at : now + 86400 - glass_retained_at;
        if (elapsed >= 3) glass_release();
    }
    glass_primary = glass_secondary = glass_close = (struct hit){0,0,0,0};
    glass_entry = (struct hit){W / 2 - 300, H / 2 - 16, 600, 56};
    if (!glass_kind) {
        if (focused >= 0) return;
        const char *title = T("Your PC. A new perspective.", "Il tuo PC. Una nuova prospettiva.");
        const font_t *hero = text_width(&font_serif, title) < W - 40 ? &font_serif : &font_title;
        draw_text(hero, (W - text_width(hero, title)) / 2, H / 2 - 80, title, 0xE7F2F6);
        glass_rrect(glass_entry.x, glass_entry.y, glass_entry.w, glass_entry.h, 24, 0x163444, 120);
        stroke_rrect(glass_entry.x, glass_entry.y, glass_entry.w, glass_entry.h, 24, 0xBFE9EB, 70);
        draw_text(F_UI, glass_entry.x + 26, glass_entry.y + 18, T("What would you like to do?", "Cosa vuoi fare?"), 0xCDE2EA);
        draw_text(F_UI, W / 2 - 235, H / 2 + 58, T("Notes     Audio     Network     Appearance", "Appunti     Audio     Rete     Aspetto"), 0xB2CDD8);
        return;
    }
    int width = glass_kind == 3 ? 440 : 620;
    if (width > W - 40) width = W - 40;
    int height = glass_kind == 2 ? 360 : glass_kind == 1 ? 300 : 220;
    int x = glass_kind == 3 ? W - width - 40 : (W - width) / 2;
    int y = glass_kind == 3 ? H - height - 118 : (H - height) / 2;
    soft_shadow(x, y, width, height, 24, 1);
    glass_rrect(x, y, width, height, 24, 0x142D40, 135);
    stroke_rrect(x, y, width, height, 24, 0xD2F4F5, 90);
    glass_close = (struct hit){x + width - 44, y + 12, 32, 32};
    line(glass_close.x + 10, glass_close.y + 10, glass_close.x + 22, glass_close.y + 22, 0xDDECF2, 230);
    line(glass_close.x + 22, glass_close.y + 10, glass_close.x + 10, glass_close.y + 22, 0xDDECF2, 230);
    const char *heading = glass_kind == 1 ? T("One intent. One surface.", "Un intento. Una superficie.") : glass_kind == 2 ? T("Your ideas", "I tuoi appunti") : glass_kind == 3 ? T("Audio", "Audio") : glass_kind == 4 ? T("Network unavailable", "Rete non disponibile") : T("Draft retained", "Bozza conservata");
    draw_text(&font_title, x + 26, y + 22, heading, 0xF2F8FA);
    if (glass_kind == 1) {
        draw_text(F_UI, x + 26, y + 70, T("Try: write notes, play audio, Wi-Fi", "Prova: appunta le idee, riproduci audio, Wi-Fi"), 0xBBD0DC);
        fill_rrect(x + 24, y + 108, width - 48, 52, 18, 0xFFFFFF, 20);
        draw_text(F_UI, x + 40, y + 124, glass_query_len ? glass_query : T("Type an intent...", "Scrivi un intento..."), 0xEAF4F8);
        draw_text(F_UI, x + 26, y + 181, T("Preview: local commands for notes, audio and network.", "Anteprima: comandi locali per appunti, audio e rete."), 0xA9C1CD);
        glass_button(&glass_primary, x + width - 190, y + height - 64, 164, T("Continue", "Continua"), 1);
    } else if (glass_kind == 2) {
        fill_rrect(x + 24, y + 76, width - 48, 176, 16, 0xFFFFFF, 18);
        int tx = x + 38, ty = y + 91, cx = tx;
        for (int i = 0; i < note_len && ty < y + 225; i++) {
            if (note_buf[i] == '\n' || cx > x + width - 60) { cx = tx; ty += 24; if (note_buf[i] == '\n') continue; }
            cx += draw_char(F_UI, cx, ty, note_buf[i], 0xF2F6FA);
        }
        if (!note_len) draw_text(F_UI, tx, ty, T("Start writing...", "Inizia a scrivere..."), 0xBBD0DC);
        if (cursor_on) fill_rect(cx + 2, ty, 2, 18, 0x8AE4E5, 255);
        draw_text(F_UI, x + 26, y + 265, T("Temporary draft. Ctrl+Enter keeps it for this session.", "Bozza temporanea. Ctrl+Invio la mantiene nella sessione."), 0xAEC7D3);
        glass_button(&glass_primary, x + width - 214, y + height - 58, 188, T("Keep draft", "Mantieni bozza"), 1);
        glass_button(&glass_secondary, x + 26, y + height - 58, 120, T("Cancel", "Annulla"), 0);
    } else if (glass_kind == 3) {
        draw_text(F_UI, x + 26, y + 72, T("Playback is not available in this preview.", "Riproduzione non disponibile in questa anteprima."), 0xBBD0DC);
        glyph(APP_MEDIA, x + width / 2 - 16, y + 108, 32, 0xBBD0DC);
        fill_rrect(x + 28, y + 164, width - 56, 4, 2, 0xBDDAE7, 90);
        draw_text(F_UI, x + 26, y + 184, T("Preview controls only - no simulated playback", "Solo anteprima - nessuna riproduzione simulata"), 0xA9C1CD);
    } else if (glass_kind == 4) {
        draw_text(F_UI, x + 26, y + 77, T("Networking is not available in this preview.", "La rete non e disponibile in questa anteprima."), 0xD5E6ED);
        draw_text(F_UI, x + 26, y + 108, T("Available networks will appear here when connected.", "Le reti disponibili appariranno qui quando integrate."), 0xBBD0DC);
        glass_button(&glass_primary, x + width - 154, y + height - 58, 128, T("Close", "Chiudi"), 1);
    } else {
        draw_text(F_UI, x + 26, y + 77, T("Kept until reboot. This draft is not saved to disk.", "Conservata fino al riavvio. Non e salvata sul disco."), 0xD5E6ED);
        glass_button(&glass_primary, x + width - 154, y + height - 58, 128, T("Close Glass", "Chiudi Glass"), 1);
    }
}
static int glass_click(int x, int y) {
    if (!glass_kind) { if (in_hit(&glass_entry, x, y) && focused < 0) { glass_open(1); return 1; } return 0; }
    if (in_hit(&glass_close, x, y) || in_hit(&glass_secondary, x, y)) { glass_release(); return 1; }
    if (in_hit(&glass_primary, x, y)) {
        if (glass_kind == 1) { unsigned next = vela_glass_intent((const uint8_t *)glass_query, glass_query_len); glass_open(next); }
        else if (glass_kind == 2) { memcpy(retained_note, note_buf, sizeof retained_note); glass_kind = vela_glass_transition(glass_kind, 5); glass_retained_at = now_secs(); memset(note_buf, 0, sizeof note_buf); note_len = 0; serial_puts("VelaOS Glass: RAM draft retained; no durable receipt claimed\r\n"); }
        else glass_release();
    }
    return 1;
}
static int glass_type(char c) {
    if (!glass_kind) return 0;
    if (glass_kind == 2) {
        if (c == '\b') { if (note_len) note_buf[--note_len] = 0; }
        else if (note_len < (int)sizeof note_buf - 1) { note_buf[note_len++] = c; note_buf[note_len] = 0; }
    } else if (glass_kind == 1) {
        if (c == '\n') { unsigned next = vela_glass_intent((const uint8_t *)glass_query, glass_query_len); glass_open(next); }
        else if (c == '\b') { if (glass_query_len) glass_query[--glass_query_len] = 0; }
        else if (c >= 32 && glass_query_len < 72) { glass_query[glass_query_len++] = c; glass_query[glass_query_len] = 0; }
    }
    return 1;
}
