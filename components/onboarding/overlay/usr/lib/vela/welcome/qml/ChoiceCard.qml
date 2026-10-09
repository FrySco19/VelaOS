import QtQuick
import QtQuick.Controls.Basic

// Large selectable card (radio behaviour inside a ButtonGroup or via `selected`).
AbstractButton {
    id: card
    property string title
    property string hint
    property string iconName
    property bool selected: false
    property Component preview: null
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    implicitWidth: 180
    implicitHeight: content.implicitHeight + 28
    Accessible.role: Accessible.RadioButton
    Accessible.name: title
    Accessible.description: hint
    Accessible.checked: selected
    Keys.onReturnPressed: clicked()
    Keys.onEnterPressed: clicked()
    scale: down ? 0.98 : 1
    Behavior on scale { enabled: theme.animations; NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }

    background: Rectangle {
        radius: theme.radius
        color: card.selected ? Qt.alpha(theme.c.accent, theme.dark ? 0.16 : 0.08) : (card.hovered ? theme.c.surface2 : theme.c.surface)
        border.width: card.selected || card.visualFocus ? 2 : 1
        border.color: card.selected || card.visualFocus ? theme.c.accent : theme.c.border
        Behavior on color { enabled: theme.animations; ColorAnimation { duration: 150 } }
    }
    contentItem: Column {
        id: content
        spacing: 8
        topPadding: 14; leftPadding: 14; rightPadding: 14
        Loader { sourceComponent: card.preview; active: card.preview !== null; width: parent.width - 28 }
        Row {
            spacing: 10
            Icon { visible: card.iconName !== ""; name: card.iconName; size: 22; color: card.selected ? theme.c.accent : theme.c.text }
            Text { text: card.title; color: theme.c.text; font.family: theme.font; font.weight: Font.DemiBold
                   font.pixelSize: Math.round(15 * theme.scale); anchors.verticalCenter: parent.verticalCenter }
        }
        Text { text: card.hint; visible: text !== ""; width: parent.width - 28; wrapMode: Text.WordWrap
               color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
    }
}
