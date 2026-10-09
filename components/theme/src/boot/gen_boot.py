#!/usr/bin/env python3
"""Boot visuals: Plymouth theme 'vela', GRUB theme 'vela' (live EFI menu + installed), isolinux splash."""
import os, subprocess, tempfile
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
PLY = os.path.join(ROOT, 'overlay', 'usr', 'share', 'plymouth', 'themes', 'vela')
GRUB_LIVE = os.path.join(ROOT, 'boot', 'grub')
GRUB_INST = os.path.join(ROOT, 'overlay', 'usr', 'share', 'grub', 'themes', 'vela')
ISO = os.path.join(ROOT, 'boot', 'isolinux')
MKFONT = os.environ.get('GRUB_MKFONT', '/tmp/apt-theme/gc/usr/bin/grub-mkfont')
INTER = '/usr/share/fonts/opentype/inter/Inter-Regular.otf'
INTER_M = '/usr/share/fonts/opentype/inter/Inter-Medium.otf'
INTER_L = '/usr/share/fonts/opentype/inter/Inter-Light.otf'

LOGO = '''<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
 <path d="M60 14 C38 40 28 72 26 100 L60 100 Z" fill="#FFFFFF"/>
 <path d="M68 26 C84 48 96 74 102 100 L68 100 Z" fill="#E68B75"/>
 <path d="M22 104 L106 104 C100 109 92 110 84 110 L44 110 C36 110 28 109 22 104 Z" fill="#FFFFFF"/>
</svg>'''

def logo_png(size, path):
    with tempfile.TemporaryDirectory() as td:
        open(os.path.join(td, 'l.svg'), 'w').write(LOGO)
        subprocess.run(['rsvg-convert', '-w', str(size), '-h', str(size), os.path.join(td, 'l.svg'), '-o', path], check=True)
    return Image.open(path).convert('RGBA')

def gradient(w, h, top=(18, 59, 77), bottom=(6, 22, 31), glow=True):
    y = np.linspace(0, 1, h, dtype=np.float32)[:, None, None]
    t = np.array(top, np.float32); b = np.array(bottom, np.float32)
    img = t * (1 - y) + b * y
    img = np.repeat(img, w, axis=1)
    if glow:
        yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
        g = np.exp(-(((xx - w * 0.5) / (w * 0.55)) ** 2 + ((yy - h * 0.42) / (h * 0.5)) ** 2))
        img += g[..., None] * np.array([20, 40, 44], np.float32) * 0.6
    rng = np.random.default_rng(5)
    img += (rng.random((h, w, 1), dtype=np.float32) - rng.random((h, w, 1), dtype=np.float32)) * 0.9
    return Image.fromarray(np.clip(img + 0.5, 0, 255).astype(np.uint8), 'RGB')

def dot(path, d=10, col=(238, 246, 246, 255)):
    s = 4; im = Image.new('RGBA', (d * s, d * s), (0, 0, 0, 0))
    ImageDraw.Draw(im).ellipse([0, 0, d * s - 1, d * s - 1], fill=col)
    im.resize((d, d), Image.LANCZOS).save(path)

def rounded(path, w, h, r, fill, outline=None):
    s = 4; im = Image.new('RGBA', (w * s, h * s), (0, 0, 0, 0))
    ImageDraw.Draw(im).rounded_rectangle([0, 0, w * s - 1, h * s - 1], radius=r * s, fill=fill,
                                         outline=outline, width=(s if outline else 0))
    im.resize((w, h), Image.LANCZOS).save(path)

PLY_SCRIPT = r'''# Vela boot splash (Plymouth script plugin). Light on CPU: a slow breathing logo + 3 dots.
Window.SetBackgroundTopColor(0.071, 0.231, 0.302);
Window.SetBackgroundBottomColor(0.024, 0.086, 0.122);

W = Window.GetWidth(); H = Window.GetHeight(); X0 = Window.GetX(); Y0 = Window.GetY();

bg.image = Image("background.png");
bg.sprite = Sprite(bg.image.Scale(W, H));
bg.sprite.SetPosition(X0, Y0, -100);

if (W > 2400) logo.image = Image("logo@2x.png"); else logo.image = Image("logo.png");
logo.sprite = Sprite(logo.image);
logo.x = X0 + W / 2 - logo.image.GetWidth() / 2;
logo.y = Y0 + H / 2 - logo.image.GetHeight() / 2 - H * 0.04;
logo.sprite.SetPosition(logo.x, logo.y, 10);

dot.image = Image("dot.png");
for (i = 0; i < 3; i++) {
  dots[i].sprite = Sprite(dot.image);
  dots[i].sprite.SetPosition(X0 + W / 2 - 22 + i * 18, logo.y + logo.image.GetHeight() + H * 0.07, 10);
  dots[i].sprite.SetOpacity(0.25);
}

t = 0;
fun refresh_callback () {
  if (global.status == "password") return;
  t++;
  # 50 Hz refresh; ~1.4 s cycle
  phase = (t % 70) / 70;
  for (i = 0; i < 3; i++) {
    p = phase - i * 0.18;
    if (p < 0) p += 1;
    a = 0.25;
    if (p < 0.35) a = 0.25 + 0.75 * Math.Sin(p / 0.35 * 3.14159);
    dots[i].sprite.SetOpacity(a);
  }
  logo.sprite.SetOpacity(0.86 + 0.14 * Math.Cos(t / 50 * 3.14159 / 1.6));
}
Plymouth.SetRefreshFunction(refresh_callback);

# ---------------------------------------------------------------- password (encrypted disk)
status = "normal";
fun dialog_setup () {
  local.box; local.entry;
  box.image = Image("box.png");
  entry.image = Image("entry.png");
  box.sprite = Sprite(box.image);
  box.x = X0 + W / 2 - box.image.GetWidth() / 2;
  box.y = Y0 + H * 0.62;
  box.sprite.SetPosition(box.x, box.y, 1000);
  entry.sprite = Sprite(entry.image);
  entry.x = box.x + box.image.GetWidth() / 2 - entry.image.GetWidth() / 2;
  entry.y = box.y + box.image.GetHeight() - entry.image.GetHeight() - 18;
  entry.sprite.SetPosition(entry.x, entry.y, 1001);
  global.dialog.box = box; global.dialog.entry = entry;
  global.dialog.bullet_image = Image("bullet.png");
  global.dialog.label = Sprite();
  global.dialog.label.SetPosition(box.x + 24, box.y + 18, 1001);
}
fun dialog_opacity (o) {
  dialog.box.sprite.SetOpacity(o); dialog.entry.sprite.SetOpacity(o); dialog.label.SetOpacity(o);
  for (i = 0; dialog.bullet[i]; i++) dialog.bullet[i].sprite.SetOpacity(o);
}
fun display_normal_callback () {
  global.status = "normal";
  if (global.dialog) dialog_opacity(0);
  for (i = 0; i < 3; i++) dots[i].sprite.SetOpacity(0.25);
}
fun display_password_callback (prompt, bullets) {
  global.status = "password";
  if (!global.dialog) dialog_setup(); else dialog_opacity(1);
  for (i = 0; i < 3; i++) dots[i].sprite.SetOpacity(0);
  text = "Password per sbloccare il disco";
  dialog.label.SetImage(Image.Text(text, 0.93, 0.96, 0.96, 1, "Inter 12"));
  for (i = 0; dialog.bullet[i] || i < bullets; i++) {
    if (!dialog.bullet[i]) {
      dialog.bullet[i].sprite = Sprite(dialog.bullet_image);
      dialog.bullet[i].sprite.SetPosition(dialog.entry.x + 14 + i * 14, dialog.entry.y + dialog.entry.image.GetHeight() / 2 - 4, 1002);
    }
    if (i < bullets && i < 30) dialog.bullet[i].sprite.SetOpacity(1); else dialog.bullet[i].sprite.SetOpacity(0);
  }
}
Plymouth.SetDisplayNormalFunction(display_normal_callback);
Plymouth.SetDisplayPasswordFunction(display_password_callback);

# ---------------------------------------------------------------- messages (e.g. fsck)
msg.sprite = Sprite();
msg.sprite.SetPosition(X0 + W * 0.1, Y0 + H * 0.9, 10);
fun message_callback (text) {
  msg.sprite.SetImage(Image.Text(text, 0.75, 0.84, 0.86, 1, "Inter 11"));
}
Plymouth.SetMessageFunction(message_callback);
fun quit_callback () { logo.sprite.SetOpacity(1); }
Plymouth.SetQuitFunction(quit_callback);
'''

def plymouth():
    os.makedirs(PLY, exist_ok=True)
    logo_png(112, os.path.join(PLY, 'logo.png')); logo_png(224, os.path.join(PLY, 'logo@2x.png'))
    gradient(960, 540).save(os.path.join(PLY, 'background.png'), optimize=True)
    dot(os.path.join(PLY, 'dot.png'), 8)
    dot(os.path.join(PLY, 'bullet.png'), 8)
    rounded(os.path.join(PLY, 'box.png'), 360, 104, 14, (16, 32, 42, 235), (255, 255, 255, 30))
    rounded(os.path.join(PLY, 'entry.png'), 312, 36, 8, (255, 255, 255, 26), (255, 255, 255, 60))
    open(os.path.join(PLY, 'vela.script'), 'w').write(PLY_SCRIPT)
    open(os.path.join(PLY, 'vela.plymouth'), 'w').write(
        "[Plymouth Theme]\nName=Vela\nDescription=Vela boot splash\nModuleName=script\n\n[script]\n"
        "ImageDir=/usr/share/plymouth/themes/vela\nScriptFile=/usr/share/plymouth/themes/vela/vela.script\n")
    print('plymouth ok')

def grub_theme(dest, live):
    os.makedirs(dest, exist_ok=True)
    W, H = 1920, 1080
    bg = gradient(W, H)
    lg = logo_png(132, '/tmp/_grub_logo.png')
    bg.paste(lg, (W // 2 - 66, int(H * 0.17)), lg)
    d = ImageDraw.Draw(bg)
    f = ImageFont.truetype(INTER_L, 46)
    tw = d.textlength('Vela', font=f); d.text(((W - tw) / 2, H * 0.17 + 146), 'Vela', font=f, fill=(238, 246, 246))
    bg.save(os.path.join(dest, 'background.png'), optimize=True)
    # selection pill 3-slice (48 px high)
    hgt = 44
    s = 4
    pill = Image.new('RGBA', (hgt * s * 2, hgt * s), (0, 0, 0, 0))
    ImageDraw.Draw(pill).rounded_rectangle([0, 0, hgt * s * 2 - 1, hgt * s - 1], radius=10 * s, fill=(255, 255, 255, 34), outline=(255, 255, 255, 50), width=s)
    pill = pill.resize((hgt * 2, hgt), Image.LANCZOS)
    pill.crop((0, 0, 12, hgt)).save(os.path.join(dest, 'select_w.png'))
    pill.crop((hgt - 2, 0, hgt + 2, hgt)).save(os.path.join(dest, 'select_c.png'))
    pill.crop((hgt * 2 - 12, 0, hgt * 2, hgt)).save(os.path.join(dest, 'select_e.png'))
    for name, fam, path, size in (('inter-regular-20', 'Inter', INTER, 20), ('inter-regular-16', 'Inter', INTER, 16), ('inter-medium-20', 'Inter Medium', INTER_M, 20)):
        out = os.path.join(dest, name + '.pf2')
        subprocess.run([MKFONT, '-n', fam, '-s', str(size), '-o', out, path], check=True, stderr=subprocess.DEVNULL)
    hint = 'Frecce ↑ ↓ per scegliere, Invio per avviare' if live else 'Frecce ↑ ↓ per scegliere, Invio per avviare, E per modificare'
    theme = f'''# Vela GRUB theme
title-text: ""
desktop-image: "background.png"
desktop-image-scale-method: "stretch"
desktop-color: "#0B2A3A"
terminal-font: "Inter Regular 16"
terminal-box: "terminal_box_*.png"

+ boot_menu {{
    left = 50%-320
    top = 50%+20
    width = 640
    height = 300
    item_font = "Inter Regular 20"
    selected_item_font = "Inter Medium Regular 20"
    item_color = "#B9CFD4"
    selected_item_color = "#FFFFFF"
    item_height = 44
    item_padding = 16
    item_spacing = 6
    icon_width = 0
    icon_height = 0
    item_icon_space = 0
    selected_item_pixmap_style = "select_*.png"
    scrollbar = false
}}

+ label {{
    top = 100%-70
    left = 0
    width = 100%
    align = "center"
    color = "#7FA0A8"
    font = "Inter Regular 16"
    text = "{hint}"
}}

+ progress_bar {{
    id = "__timeout__"
    top = 100%-110
    left = 50%-200
    width = 400
    height = 22
    align = "center"
    font = "Inter Regular 16"
    text_color = "#9FB8BE"
    fg_color = "#E68B75"
    bg_color = "#0B2A3A"
    border_color = "#0B2A3A"
    text = "@TIMEOUT_NOTIFICATION_SHORT@"
}}
'''
    open(os.path.join(dest, 'theme.txt'), 'w').write(theme)
    # terminal box (for the command line / edit screen): dark rounded
    for part, box in (('c', None),):
        pass
    tb = Image.new('RGBA', (8, 8), (6, 22, 31, 230)); 
    for p in ('nw', 'n', 'ne', 'w', 'c', 'e', 'sw', 's', 'se'):
        tb.save(os.path.join(dest, f'terminal_box_{p}.png'))
    print('grub ok', dest)

def isolinux():
    os.makedirs(ISO, exist_ok=True)
    W, H = 640, 480
    bg = gradient(W, H)
    lg = logo_png(72, '/tmp/_iso_logo.png')
    bg.paste(lg, (W // 2 - 36, 70), lg)
    d = ImageDraw.Draw(bg)
    f = ImageFont.truetype(INTER_L, 26)
    tw = d.textlength('Vela', font=f); d.text(((W - tw) / 2, 148), 'Vela', font=f, fill=(238, 246, 246))
    bg.save(os.path.join(ISO, 'splash.png'), optimize=True)
    open(os.path.join(ISO, 'vela.cfg'), 'w').write('''# Vela colors for syslinux vesamenu (include from the live menu: "include vela.cfg")
menu background splash.png
menu width 50
menu margin 14
menu rows 6
menu vshift 11
menu tabmsgrow 18
menu cmdlinerow 18
menu timeoutrow 17
menu helpmsgrow 19
menu color screen      37;40   #00000000 #00000000 none
menu color border      30;44   #00000000 #00000000 none
menu color title       1;36;44 #00000000 #00000000 none
menu color sel         7;37;40 #ffffffff #40ffffff std
menu color unsel       37;44   #ffb9cfd4 #00000000 none
menu color hotsel      1;7;37;40 #ffffffff #40ffffff std
menu color hotkey      1;37;44 #ffe68b75 #00000000 none
menu color tabmsg      31;40   #ff7fa0a8 #00000000 none
menu color timeout_msg 37;40   #ff7fa0a8 #00000000 none
menu color timeout     1;37;40 #ffe68b75 #00000000 none
menu color help        37;40   #ffb9cfd4 #00000000 none
menu color cmdline     37;40   #ffeef6f6 #00000000 none
''')
    print('isolinux ok')

plymouth()
grub_theme(GRUB_LIVE, True)
grub_theme(GRUB_INST, False)
isolinux()
