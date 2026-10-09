import QtQuick
import QtQuick.Controls.Basic

// kind: "primary" | "secondary" | "plain"
Button {
    id: b
    property string kind: "secondary"
    property string iconName: ""
    property bool danger: false
    font.family: theme.font
    font.pixelSize: Math.round(14 * theme.scale)
    font.weight: kind === "primary" ? Font.DemiBold : Font.Medium
    implicitHeight: Math.round(36 * theme.scale)
    leftPadding: 16; rightPadding: 16
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.name: text
    Accessible.role: Accessible.Button
    Keys.onReturnPressed: clicked()
    Keys.onEnterPressed: clicked()

    contentItem: Row {
        spacing: 8
        Icon {
            visible: b.iconName !== ""
            name: b.iconName; size: 18
            anchors.verticalCenter: parent.verticalCenter
            color: lbl.color
        }
        Text {
            id: lbl
            text: b.text
            font: b.font
            anchors.verticalCenter: parent.verticalCenter
            color: !b.enabled ? theme.c.muted
                 : b.kind === "primary" ? theme.c.onAccent
                 : b.kind === "plain" ? (b.danger ? theme.c.danger : theme.c.accent) : theme.c.text
        }
    }
    background: Rectangle {
        radius: theme.radius - 3
        color: b.kind === "primary" ? (b.enabled ? Qt.darker(theme.c.accent, b.down ? 1.18 : (b.hovered ? 1.08 : 1.0)) : theme.c.surface2)
             : b.kind === "plain" ? (b.hovered ? theme.c.surface2 : "transparent")
             : (b.down ? Qt.darker(theme.c.surface2, 1.06) : (b.hovered ? theme.c.surface2 : theme.c.surface))
        border.width: b.visualFocus ? 2 : (b.kind === "secondary" ? 1 : 0)
        border.color: b.visualFocus ? (b.kind === "primary" ? theme.c.text : theme.c.accent) : theme.c.border
        Behavior on color { enabled: theme.animations; ColorAnimation { duration: 120 } }
    }
}
