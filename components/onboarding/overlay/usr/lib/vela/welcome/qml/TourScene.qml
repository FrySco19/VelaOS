import QtQuick

// Animated schematic of a desktop showing what a shortcut does. kind: launcher|search|assistant|clipboard|overview|tile|spaces|screenshot
Rectangle {
    id: sc
    property string kind: "search"
    property real t: theme.animations ? animT : 1
    implicitWidth: 360; implicitHeight: 210
    radius: 12; clip: true
    Accessible.ignored: true
    gradient: Gradient {
        GradientStop { position: 0; color: theme.dark ? "#0E2240" : "#CFE0F3" }
        GradientStop { position: 1; color: theme.dark ? "#1C3D66" : "#9DBCE0" }
    }
    border.width: 1; border.color: theme.c.border
    readonly property color win: theme.dark ? "#1A2E4A" : "#FFFFFF"
    readonly property color line: theme.dark ? "#3A4D69" : "#D6DEE8"
    readonly property color glass: Qt.alpha(theme.dark ? "#0B1526" : "#FFFFFF", 0.85)
    function ease(x) { x = Math.max(0, Math.min(1, x)); return 1 - Math.pow(1 - x, 3) }
    function seg(a, b) { return ease((t - a) / (b - a)) }

    property real animT: 0
    NumberAnimation on animT { id: anim; from: 0; to: 1; duration: 3200; loops: Animation.Infinite; running: theme.animations }
    onKindChanged: if (theme.animations) anim.restart()

    // top bar + dock (common)
    Rectangle { width: parent.width; height: 12; color: sc.glass }
    Rectangle {
        width: 150; height: 18; radius: 7; color: sc.glass
        anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; anchors.bottomMargin: 6
        Row { anchors.centerIn: parent; spacing: 6
              Repeater { model: 7; Rectangle { width: 10; height: 10; radius: 3; color: index === 0 ? theme.c.accent : (theme.dark ? "#5C7393" : "#8FA3BC") } } }
    }
    component Win: Rectangle {
        radius: 6; color: sc.win
        border.width: 1; border.color: Qt.alpha(sc.line, 0.7)
        Rectangle { width: parent.width; height: 10; radius: 6; color: theme.dark ? "#23395A" : "#EEF2F7" }
        Rectangle { x: 8; y: 18; width: parent.width * 0.45; height: 5; radius: 2; color: theme.c.accent; opacity: 0.8 }
        Rectangle { x: 8; y: 28; width: parent.width * 0.75; height: 4; radius: 2; color: sc.line }
        Rectangle { x: 8; y: 36; width: parent.width * 0.6; height: 4; radius: 2; color: sc.line }
    }

    // ---- launcher: grid of apps fades/zooms in
    Rectangle {
        visible: sc.kind === "launcher"
        anchors.fill: parent; anchors.topMargin: 12; color: Qt.alpha(sc.win, 0.55 * sc.seg(0, 0.3))
        Grid {
            anchors.centerIn: parent; columns: 6; spacing: 14
            opacity: sc.seg(0.05, 0.35); scale: 0.9 + 0.1 * sc.seg(0.05, 0.35)
            Repeater { model: 18; Rectangle { width: 26; height: 26; radius: 7
                       color: Qt.hsla((index * 0.13) % 1, 0.45, theme.dark ? 0.5 : 0.62, 1) } }
        }
    }
    // ---- search: bar drops in, text types, results appear
    Item {
        visible: sc.kind === "search"
        anchors.fill: parent
        Win { x: 30; y: 30; width: 160; height: 110; opacity: 0.6 }
        Rectangle {
            id: bar
            width: 230; height: 30; radius: 9; color: sc.glass
            border.width: 1; border.color: sc.line
            x: (sc.width - width) / 2; y: 40 - 10 * (1 - sc.seg(0, 0.15)); opacity: sc.seg(0, 0.15)
            Icon { x: 9; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 14; color: theme.c.muted }
            Text { x: 30; anchors.verticalCenter: parent.verticalCenter; color: theme.c.text; font.family: theme.font; font.pixelSize: 12
                   text: { const s = backend.lang === "it" ? "Impostazioni" : "Settings"; return s.substring(0, Math.round(s.length * sc.seg(0.2, 0.55))) } }
        }
        Rectangle {
            width: 230; height: 74; radius: 9; color: sc.glass; x: bar.x; y: bar.y + 36
            opacity: sc.seg(0.55, 0.7)
            Column { anchors.fill: parent; anchors.margins: 8; spacing: 6
                Repeater { model: 3; Row { spacing: 8
                    Rectangle { width: 14; height: 14; radius: 4; color: index === 0 ? theme.c.accent : sc.line }
                    Rectangle { width: 120 - index * 25; height: 6; radius: 3; color: sc.line; anchors.verticalCenter: parent.verticalCenter } } }
            }
            Rectangle { x: 4; y: 5; width: parent.width - 8; height: 20; radius: 6; color: Qt.alpha(theme.c.accent, 0.18) }
        }
    }
    // ---- assistant: panel slides from right with bubbles
    Item {
        visible: sc.kind === "assistant"
        anchors.fill: parent
        Win { x: 24; y: 26; width: 180; height: 130 }
        Rectangle {
            width: 130; height: 160; radius: 10; color: sc.glass; border.width: 1; border.color: sc.line
            y: 22; x: sc.width - (width + 12) * sc.seg(0, 0.2)
            Rectangle { x: 50; y: 14; width: 70; height: 18; radius: 8; color: theme.c.accent; opacity: sc.seg(0.25, 0.35) }
            Rectangle { x: 10; y: 40; width: 90; height: 34; radius: 8; color: sc.line; opacity: sc.seg(0.45, 0.6) }
            Rectangle { x: 10; y: 80; width: 60; height: 16; radius: 6; color: Qt.alpha(theme.c.accent, 0.25); opacity: sc.seg(0.65, 0.75) }
            Rectangle { x: 8; y: parent.height - 26; width: parent.width - 16; height: 18; radius: 6; color: sc.win; border.width: 1; border.color: sc.line }
        }
    }
    // ---- clipboard: popup list, highlight moves
    Item {
        visible: sc.kind === "clipboard"
        anchors.fill: parent
        Win { x: 40; y: 26; width: 200; height: 130; opacity: 0.8 }
        Rectangle {
            width: 160; height: 104; radius: 9; color: sc.glass; border.width: 1; border.color: sc.line
            x: 150; y: 60; opacity: sc.seg(0, 0.15); scale: 0.95 + 0.05 * sc.seg(0, 0.15)
            Rectangle { x: 5; width: parent.width - 10; height: 26; radius: 6; color: Qt.alpha(theme.c.accent, 0.22)
                        y: 6 + 32 * Math.min(2, Math.floor(sc.t * 3.2)) ; Behavior on y { NumberAnimation { duration: 160 } } }
            Column { x: 12; y: 13; spacing: 20
                Repeater { model: 3; Rectangle { width: 120 - index * 20; height: 6; radius: 3; color: theme.c.text; opacity: 0.45 } } }
        }
    }
    // ---- overview: windows shrink into a grid, spaces strip appears
    Item {
        visible: sc.kind === "overview"
        anchors.fill: parent
        Rectangle { anchors.fill: parent; color: Qt.alpha("#000000", 0.25 * sc.seg(0, 0.3)) }
        Row { x: (sc.width - width) / 2; y: 18; spacing: 8; opacity: sc.seg(0.2, 0.45)
              Repeater { model: 3; Rectangle { width: 54; height: 32; radius: 5; color: Qt.alpha(sc.win, 0.5)
                         border.width: index === 0 ? 2 : 0; border.color: theme.c.accent } } }
        Repeater {
            model: 4
            Win {
                property real k: sc.seg(0, 0.4)
                // start: overlapping big windows; end: 2x2 grid
                x: (40 + index * 18) * (1 - k) + (70 + (index % 2) * 120) * k
                y: (24 + index * 12) * (1 - k) + (62 + Math.floor(index / 2) * 66) * k
                width: 200 * (1 - k) + 100 * k
                height: 120 * (1 - k) + 56 * k
            }
        }
    }
    // ---- tile: two windows move to left/right halves
    Item {
        visible: sc.kind === "tile"
        anchors.fill: parent
        Win { property real k: sc.seg(0.05, 0.4)
              x: 40 * (1 - k) + 4 * k; y: 30 * (1 - k) + 16 * k; width: 170 * (1 - k) + (sc.width / 2 - 6) * k; height: 120 * (1 - k) + (sc.height - 46) * k }
        Win { property real k: sc.seg(0.45, 0.8)
              x: 140 * (1 - k) + (sc.width / 2 + 2) * k; y: 50 * (1 - k) + 16 * k; width: 170 * (1 - k) + (sc.width / 2 - 6) * k; height: 110 * (1 - k) + (sc.height - 46) * k }
    }
    // ---- spaces: the whole desktop slides to the next space
    Item {
        visible: sc.kind === "spaces"
        anchors.fill: parent; anchors.topMargin: 12
        clip: true
        Row {
            x: -sc.width * sc.seg(0.3, 0.6)
            Item { width: sc.width; height: 170; Win { x: 30; y: 20; width: 180; height: 110 } Win { x: 150; y: 50; width: 150; height: 90 } }
            Item { width: sc.width; height: 170; Win { x: 90; y: 26; width: 200; height: 120 } }
        }
        Row { anchors.horizontalCenter: parent.horizontalCenter; y: 6; spacing: 6
              Repeater { model: 2; Rectangle { width: 7; height: 7; radius: 4; color: (index === 0) === (sc.t < 0.45) ? theme.c.accent : sc.line } } }
    }
    // ---- screenshot: selection rectangle grows, flash, thumbnail
    Item {
        visible: sc.kind === "screenshot"
        anchors.fill: parent
        Win { x: 30; y: 24; width: 220; height: 130 }
        Rectangle { anchors.fill: parent; color: Qt.alpha("#000000", 0.25 * (sc.t < 0.6 ? 1 : 0)) }
        Rectangle {
            x: 60; y: 50; color: "transparent"
            width: 160 * sc.seg(0.05, 0.45); height: 80 * sc.seg(0.05, 0.45)
            border.width: 2; border.color: theme.c.accent
            visible: sc.t < 0.6
        }
        Rectangle { anchors.fill: parent; color: "#FFFFFF"; opacity: sc.t > 0.6 && sc.t < 0.7 ? 0.7 : 0 }
        Rectangle {
            width: 80; height: 46; radius: 6; color: sc.win; border.width: 2; border.color: sc.glass
            x: sc.width - 92; y: sc.height - 70 + 20 * (1 - sc.seg(0.7, 0.85)); opacity: sc.seg(0.7, 0.85)
            Icon { anchors.centerIn: parent; name: "camera"; size: 18; color: theme.c.accent }
        }
    }
}
