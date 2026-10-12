//! Minimal RGB PNG (stored deflate) so the ciao demo can write real images
//! without adding a crate dependency.

use super::font;
use vela_intent::ContextProposal;

const SCALE: u32 = 2;

#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

const BG: Rgb = Rgb(12, 18, 28);
const PANEL: Rgb = Rgb(18, 28, 42);
const TEXT: Rgb = Rgb(236, 242, 248);
const MUTED: Rgb = Rgb(154, 174, 196);
const ACCENT: Rgb = Rgb(126, 182, 255);
const ROW: Rgb = Rgb(30, 44, 64);
const AMBER_BG: Rgb = Rgb(62, 44, 18);
const AMBER: Rgb = Rgb(242, 196, 96);

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32, bg: Rgb) -> Self {
        let mut canvas = Self {
            w,
            h,
            px: vec![0; w as usize * h as usize * 3],
        };
        canvas.fill(bg);
        canvas
    }
    fn fill(&mut self, c: Rgb) {
        for px in self.px.chunks_exact_mut(3) {
            px[0] = c.0;
            px[1] = c.1;
            px[2] = c.2;
        }
    }
    fn rect(&mut self, x: u32, y: u32, w: u32, h: u32, c: Rgb) {
        for yy in y..y.saturating_add(h).min(self.h) {
            for xx in x..x.saturating_add(w).min(self.w) {
                let i = (yy as usize * self.w as usize + xx as usize) * 3;
                self.px[i] = c.0;
                self.px[i + 1] = c.1;
                self.px[i + 2] = c.2;
            }
        }
    }
    fn text(&mut self, x: u32, y: u32, s: &str, c: Rgb) {
        let mut cx = x;
        for ch in s.chars() {
            let glyph = font::glyph(ch);
            for row in 0..8u32 {
                let bits = glyph[row as usize];
                for col in 0..8u32 {
                    if bits & (1 << col) != 0 {
                        self.rect(cx + col * SCALE, y + row * SCALE, SCALE, SCALE, c);
                    }
                }
            }
            cx = cx.saturating_add(8 * SCALE);
        }
    }
    fn encode(&self) -> Vec<u8> {
        encode_png(self.w, self.h, &self.px)
    }
}

fn text_px(s: &str) -> u32 {
    s.chars().count() as u32 * 8 * SCALE
}

fn line_h() -> u32 {
    8 * SCALE + 6
}

pub fn proposal_list_png(p: &ContextProposal) -> Vec<u8> {
    let mut lines: Vec<(String, Rgb)> = vec![
        ("Proposal".into(), ACCENT),
        (p.mode.to_string(), TEXT),
        ("LLM not invoked".into(), AMBER),
        (format!("utterance: {}", p.utterance), TEXT),
        (format!("bm25 lexical hits: {}", p.bm25_lexical_hits), MUTED),
        (format!("rows: {}", p.candidates.len()), TEXT),
        (String::new(), TEXT),
    ];
    for (i, c) in p.candidates.iter().enumerate() {
        let version = c.expected_version.as_deref().unwrap_or("null");
        lines.push((format!("{}  source={}", i + 1, c.source.as_str()), ACCENT));
        lines.push((
            format!(
                "   type={}  kind={}  score={:.2}",
                c.signal_type, c.kind, c.score
            ),
            TEXT,
        ));
        lines.push((format!("   handle={}", c.handle), TEXT));
        lines.push((format!("   version={version}"), MUTED));
        lines.push((String::new(), TEXT));
    }
    let margin = 20u32;
    let widest = lines.iter().map(|(s, _)| text_px(s)).max().unwrap_or(0);
    let w = margin * 2 + widest.max(text_px("Proposal"));
    let h = margin * 2 + lines.len() as u32 * line_h();
    let mut canvas = Canvas::new(w, h, BG);
    canvas.rect(8, 8, w.saturating_sub(16), h.saturating_sub(16), PANEL);
    for (i, (line, color)) in lines.iter().enumerate() {
        canvas.text(margin, margin + i as u32 * line_h(), line, *color);
    }
    canvas.encode()
}

pub fn glass_surface_png(p: &ContextProposal) -> Vec<u8> {
    let margin = 22u32;
    let gap = 10u32;
    let lh = line_h();
    let header = [
        "VELA GLASS".to_string(),
        "proposal only".to_string(),
        format!("utterance: {}", p.utterance),
        format!("mode: {}", p.mode),
        "LLM not invoked".to_string(),
    ];
    let mut cards: Vec<Vec<String>> = Vec::new();
    for (i, c) in p.candidates.iter().enumerate() {
        let version = c.expected_version.as_deref().unwrap_or("null");
        cards.push(vec![
            format!("{}. {}   score={:.2}", i + 1, c.signal_type, c.score),
            format!("source={}", c.source.as_str()),
            format!("handle={}", c.handle),
            format!("version={version}"),
        ]);
    }
    let banner = ["«nessuna autorità»".to_string(), "no-authority".to_string()];
    let mut widest = text_px(&banner[0]);
    for line in header
        .iter()
        .chain(cards.iter().flatten())
        .chain(banner.iter())
    {
        widest = widest.max(text_px(line));
    }
    let inner = widest + 28;
    let w = margin * 2 + inner;
    let card_h = |n: usize| 10 + n as u32 * lh + 6;
    let header_h = header.len() as u32 * lh;
    let banner_h = 10 + banner.len() as u32 * lh + 6;
    let cards_h: u32 = cards.iter().map(|c| card_h(c.len()) + gap).sum();
    let h = margin + header_h + gap + cards_h + banner_h + margin;
    let mut canvas = Canvas::new(w, h, BG);
    canvas.rect(8, 8, w.saturating_sub(16), h.saturating_sub(16), PANEL);
    let mut y = margin;
    for (i, line) in header.iter().enumerate() {
        let color = if i == 0 {
            ACCENT
        } else if line == "LLM not invoked" {
            AMBER
        } else {
            TEXT
        };
        canvas.text(margin + 8, y, line, color);
        y += lh;
    }
    y += gap;
    for card in &cards {
        let ch = card_h(card.len());
        canvas.rect(margin, y, inner, ch, ROW);
        canvas.rect(margin, y, 4, ch, ACCENT);
        let mut ty = y + 8;
        for line in card {
            canvas.text(margin + 14, ty, line, TEXT);
            ty += lh;
        }
        y += ch + gap;
    }
    canvas.rect(margin, y, inner, banner_h, AMBER_BG);
    let mut ty = y + 8;
    for (i, line) in banner.iter().enumerate() {
        canvas.text(margin + 14, ty, line, if i == 0 { AMBER } else { TEXT });
        ty += lh;
    }
    canvas.encode()
}

fn encode_png(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    let stride = w as usize * 3;
    let mut raw = Vec::with_capacity(h as usize * (1 + stride));
    for y in 0..h as usize {
        raw.push(0);
        let start = y * stride;
        raw.extend_from_slice(&rgb[start..start + stride]);
    }
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    png.extend_from_slice(&chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut crc_src = Vec::with_capacity(4 + data.len());
    crc_src.extend_from_slice(kind);
    crc_src.extend_from_slice(data);
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_src).to_be_bytes());
    out
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut off = 0;
    loop {
        let remain = data.len() - off;
        let n = remain.min(65535);
        let last = remain <= 65535;
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!n as u16).to_le_bytes());
        out.extend_from_slice(&data[off..off + n]);
        off += n;
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}
