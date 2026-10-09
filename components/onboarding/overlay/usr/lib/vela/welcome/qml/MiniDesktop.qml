import QtQuick
// Small schematic desktop used in previews. dark: colours; dock: bottom|left|right; split: diagonal light/dark.
Rectangle {
    id: m
    property bool dark: false
    property bool split: false
    property string dock: "bottom"
    property bool autohide: false
    property color accent: theme.c.accent
    implicitWidth: 160; implicitHeight: 100
    radius: 8
    clip: true
    color: dark ? "#13294B" : "#BFD3EA"
    border.width: 1; border.color: theme.c.border
    Accessible.ignored: true
    gradient: Gradient {
        GradientStop { position: 0; color: m.dark ? "#0E2240" : "#CFE0F3" }
        GradientStop { position: 1; color: m.dark ? "#1C3D66" : "#9DBCE0" }
    }
    Rectangle { // dark half for "auto"
        visible: m.split
        width: parent.width * 1.5; height: parent.height * 2
        x: parent.width / 2; y: -parent.height / 2
        rotation: 20; transformOrigin: Item.Left
        color: "#132B4D"
    }
    Rectangle { // top bar
        width: parent.width; height: 8
        color: Qt.alpha(m.dark ? "#0B1526" : "#FFFFFF", 0.75)
    }
    Rectangle { // window
        x: parent.width * 0.18 + (m.dock === "left" ? 8 : 0); y: 16
        width: parent.width * 0.58; height: parent.height * 0.52
        radius: 5
        color: m.dark ? "#1A2E4A" : "#FFFFFF"
        Rectangle { width: parent.width; height: 9; radius: 5; color: m.dark ? "#23395A" : "#EEF2F7" }
        Rectangle { x: 8; y: 16; width: parent.width * 0.4; height: 5; radius: 2; color: m.accent }
        Rectangle { x: 8; y: 26; width: parent.width * 0.7; height: 4; radius: 2; color: m.dark ? "#3A4D69" : "#D6DEE8" }
        Rectangle { x: 8; y: 34; width: parent.width * 0.55; height: 4; radius: 2; color: m.dark ? "#3A4D69" : "#D6DEE8" }
    }
    Rectangle { // dock
        id: dk
        opacity: m.autohide ? 0.35 : 1
        property bool vertical: m.dock !== "bottom"
        width: vertical ? 12 : parent.width * 0.5
        height: vertical ? parent.height * 0.55 : 12
        radius: 5
        x: m.dock === "bottom" ? (parent.width - width) / 2 : (m.dock === "left" ? 4 : parent.width - width - 4)
        y: m.dock === "bottom" ? parent.height - height - 4 : (parent.height - height) / 2 + 4
        color: Qt.alpha(m.dark ? "#0B1526" : "#FFFFFF", 0.8)
        Behavior on x { enabled: theme.animations; SpringAnimation { spring: 4; damping: 0.4 } }
        Behavior on y { enabled: theme.animations; SpringAnimation { spring: 4; damping: 0.4 } }
        Grid {
            anchors.centerIn: parent
            columns: dk.vertical ? 1 : 5; rows: dk.vertical ? 5 : 1
            spacing: 3
            Repeater { model: 5; Rectangle { width: 6; height: 6; radius: 2; color: index === 0 ? m.accent : (m.dark ? "#5C7393" : "#8FA3BC") } }
        }
    }
}
