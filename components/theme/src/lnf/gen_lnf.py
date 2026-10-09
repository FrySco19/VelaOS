#!/usr/bin/env python3
"""Look-and-feel packages org.vela.desktop (light) and org.vela.desktop.dark."""
import json, os, shutil, sys
from PIL import Image, ImageDraw, ImageFilter
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
OUT = os.path.join(ROOT, 'overlay', 'usr', 'share', 'plasma', 'look-and-feel')
WP = os.path.join(ROOT, 'overlay', 'usr', 'share', 'wallpapers', 'Vela', 'contents')
sys.path.insert(0, os.path.join(HERE, '..', 'icons'))

LOGO_WHITE = '''<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
 <path d="M60 14 C38 40 28 72 26 100 L60 100 Z" fill="#FFFFFF"/>
 <path d="M68 26 C84 48 96 74 102 100 L68 100 Z" fill="#E68B75"/>
 <path d="M22 104 L106 104 C100 109 92 110 84 110 L44 110 C36 110 28 109 22 104 Z" fill="#FFFFFF"/>
</svg>
'''

SPLASH = r'''/*
    Vela splash screen. SPDX-License-Identifier: GPL-2.0-or-later
*/
import QtQuick
import org.kde.kirigami 2 as Kirigami

Rectangle {
    id: root
    property int stage
    readonly property bool motion: Kirigami.Units.longDuration > 1
    gradient: Gradient {
        GradientStop { position: 0.0; color: "#123B4D" }
        GradientStop { position: 1.0; color: "#06161F" }
    }

    onStageChanged: {
        if (stage == 2) {
            intro.running = true
        }
    }

    Item {
        id: content
        anchors.fill: parent
        opacity: root.motion ? 0 : 1

        Image {
            id: logo
            readonly property real size: Kirigami.Units.gridUnit * 6
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -Kirigami.Units.gridUnit
            source: "images/vela.svg"
            sourceSize.width: size
            sourceSize.height: size
            asynchronous: true
            scale: root.motion ? 0.94 : 1
            NumberAnimation on scale { id: grow; running: false; to: 1; duration: 600; easing.type: Easing.OutCubic }
        }
        Text {
            id: word
            anchors.top: logo.bottom
            anchors.topMargin: Kirigami.Units.largeSpacing
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Vela"
            color: "#EEF6F6"
            font.family: "Inter"
            font.pointSize: 20
            font.weight: Font.Light
            font.letterSpacing: 1
        }
        Rectangle {
            id: track
            width: Kirigami.Units.gridUnit * 8
            height: 3
            radius: 1.5
            color: "#33EEF6F6"
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: word.bottom
            anchors.topMargin: Kirigami.Units.gridUnit * 2
            Rectangle {
                height: parent.height
                radius: parent.radius
                color: "#E68B75"
                width: parent.width * Math.min(1, Math.max(0.08, root.stage / 6))
                Behavior on width { enabled: root.motion; NumberAnimation { duration: 400; easing.type: Easing.OutCubic } }
            }
        }
    }

    ParallelAnimation {
        id: intro
        running: false
        OpacityAnimator { target: content; from: content.opacity; to: 1; duration: root.motion ? 500 : 0; easing.type: Easing.OutCubic }
        ScriptAction { script: if (root.motion) grow.running = true }
    }
}
'''

def meta(pid, name_en, name_it, desc_it):
    return {"KPackageStructure": "Plasma/LookAndFeel",
            "KPlugin": {"Authors": [{"Name": "Vela"}], "Id": pid, "License": "GPL-2.0-or-later",
                        "Name": name_en, "Name[it]": name_it,
                        "Description": "Vela desktop " + ("dark" if 'dark' in pid else "light"),
                        "Description[it]": desc_it, "Website": "https://vela.invalid/"},
            "X-Plasma-APIVersion": "2"}

def defaults(dark):
    return f"""[kdeglobals][KDE]
widgetStyle=Breeze

[kdeglobals][General]
ColorScheme={'VelaDark' if dark else 'VelaLight'}

[kdeglobals][Icons]
Theme={'Vela-Dark' if dark else 'Vela'}

[plasmarc][Theme]
name=vela

[Wallpaper]
Image=Vela

[kcminputrc][Mouse]
cursorTheme=Bibata-Modern-Classic

[kwinrc][org.kde.kdecoration2]
library=org.kde.kwin.aurorae
theme=__aurorae__svg__{'VelaDark' if dark else 'VelaLight'}

[KSplash]
Theme=org.vela.desktop
"""

def preview(dark, path, size):
    src = os.path.join(WP, 'images_dark' if dark else 'images', '3840x2160.jpg')
    im = Image.open(src).convert('RGB').resize(size, Image.LANCZOS)
    d = ImageDraw.Draw(im, 'RGBA')
    w, h = size
    bar = (20, 36, 44, 190) if dark else (246, 250, 251, 190)
    d.rectangle([0, 0, w, max(3, h // 40)], fill=bar)                       # top bar
    dw, dh = w * 0.42, h * 0.075
    d.rounded_rectangle([w / 2 - dw / 2, h - dh - h * 0.02, w / 2 + dw / 2, h - h * 0.02], radius=dh / 3, fill=bar)  # dock
    win = (29, 46, 55, 255) if dark else (255, 255, 255, 255)
    hdr = (29, 46, 55, 255) if dark else (234, 240, 242, 255)
    x0, y0, x1, y1 = w * 0.18, h * 0.16, w * 0.62, h * 0.70
    d.rounded_rectangle([x0, y0, x1, y1], radius=h / 60, fill=win)
    d.rounded_rectangle([x0, y0, x1, y0 + h / 18], radius=h / 60, fill=hdr)
    d.rectangle([x0, y0 + h / 36, x1, y0 + h / 18], fill=hdr)
    for i, c in enumerate([(229, 128, 108), (227, 194, 126), (127, 180, 158)]):
        cx = x0 + h / 40 + i * h / 45; cy = y0 + h / 36
        d.ellipse([cx - h / 140, cy - h / 140, cx + h / 140, cy + h / 140], fill=c + (255,))
    im.save(path, quality=88)

def write(pid, dark, name_en, name_it, desc_it):
    d = os.path.join(OUT, pid); c = os.path.join(d, 'contents')
    for sub in ('layouts', 'previews', 'splash/images'):
        os.makedirs(os.path.join(c, sub), exist_ok=True)
    json.dump(meta(pid, name_en, name_it, desc_it), open(os.path.join(d, 'metadata.json'), 'w'), indent=2, ensure_ascii=False)
    open(os.path.join(c, 'defaults'), 'w').write(defaults(dark))
    open(os.path.join(c, 'layouts', 'org.kde.plasma.desktop-layout.js'), 'w').write(
        'loadTemplate("org.vela.desktop.defaultPanels")\n')
    open(os.path.join(c, 'splash', 'Splash.qml'), 'w').write(SPLASH)
    open(os.path.join(c, 'splash', 'images', 'vela.svg'), 'w').write(LOGO_WHITE)
    preview(dark, os.path.join(c, 'previews', 'preview.png'), (320, 180))
    preview(dark, os.path.join(c, 'previews', 'fullscreenpreview.jpg'), (1920, 1080))
    # splash preview for the KCM
    im = Image.new('RGB', (320, 180), (11, 42, 58)); im.save(os.path.join(c, 'previews', 'splash.png'))
    print('wrote', d)

write('org.vela.desktop', False, 'Vela Light', 'Vela chiaro', 'Vela con tema chiaro')
write('org.vela.desktop.dark', True, 'Vela Dark', 'Vela scuro', 'Vela con tema scuro')
