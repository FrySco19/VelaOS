import QtQuick
import QtQuick.Controls.Basic

// Row with icon, title, explanation and a switch. Whole row is clickable and keyboard-toggleable.
AbstractButton {
    id: row
    property string title
    property string hint
    property string iconName: ""
    checkable: true
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    implicitWidth: 520
    implicitHeight: Math.max(Math.round(56 * theme.scale), col.implicitHeight + 20)
    Accessible.role: Accessible.CheckBox
    Accessible.name: title
    Accessible.description: hint
    Accessible.checked: checked
    Keys.onReturnPressed: toggle()
    Keys.onEnterPressed: toggle()

    background: Rectangle {
        radius: theme.radius - 4
        color: row.hovered ? theme.c.surface2 : "transparent"
        border.width: row.visualFocus ? 2 : 0
        border.color: theme.c.accent
    }
    contentItem: Item {
        Icon {
            id: ic
            visible: row.iconName !== ""
            name: row.iconName; size: 22
            color: theme.c.accent
            anchors.left: parent.left; anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
        }
        Column {
            id: col
            anchors.left: ic.visible ? ic.right : parent.left
            anchors.leftMargin: 12
            anchors.right: sw.left; anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text { text: row.title; width: parent.width; wrapMode: Text.WordWrap; color: theme.c.text
                   font.family: theme.font; font.pixelSize: Math.round(14 * theme.scale); font.weight: Font.Medium }
            Text { text: row.hint; visible: text !== ""; width: parent.width; wrapMode: Text.WordWrap; color: theme.c.muted
                   font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
        }
        Rectangle {
            id: sw
            width: 44; height: 26; radius: 13
            anchors.right: parent.right; anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            color: row.checked ? theme.c.accent : (theme.dark ? "#3A4D69" : "#C5CFDB")
            border.width: theme.highContrast ? 2 : 0
            border.color: theme.c.text
            Behavior on color { enabled: theme.animations; ColorAnimation { duration: 150 } }
            Rectangle {
                width: 22; height: 22; radius: 11
                y: 2; x: row.checked ? parent.width - width - 2 : 2
                color: "#FFFFFF"
                border.width: theme.highContrast ? 1 : 0; border.color: "#000000"
                Behavior on x { enabled: theme.animations; SpringAnimation { spring: 5; damping: 0.38; epsilon: 0.25 } }
            }
        }
    }
}
