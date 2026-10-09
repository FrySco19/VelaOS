import QtQuick
import QtQuick.Controls.Basic
TextField {
    id: f
    property string label
    font.family: theme.font
    font.pixelSize: Math.round(14 * theme.scale)
    color: theme.c.text
    placeholderTextColor: theme.c.muted
    selectionColor: theme.c.accent
    selectedTextColor: "#FFFFFF"
    implicitHeight: Math.round(38 * theme.scale)
    leftPadding: 12; rightPadding: 12
    Accessible.name: label
    background: Rectangle {
        radius: theme.radius - 4
        color: theme.c.field
        border.width: f.activeFocus ? 2 : 1
        border.color: f.activeFocus ? theme.c.accent : theme.c.border
    }
}
