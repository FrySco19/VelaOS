import QtQuick
import QtQuick.Controls.Basic

// Segmented control: model = [{value, label}], current = value
Rectangle {
    id: seg
    property var model: []
    property string current
    property string accessibleName
    signal picked(string value)
    implicitHeight: Math.round(34 * theme.scale)
    implicitWidth: rowItem.implicitWidth + 6
    radius: theme.radius - 4
    color: theme.c.surface2
    border.width: theme.highContrast ? 1 : 0
    border.color: theme.c.border
    Accessible.role: Accessible.PageTabList
    Accessible.name: accessibleName

    Row {
        id: rowItem
        anchors.fill: parent; anchors.margins: 3
        spacing: 0
        Repeater {
            model: seg.model
            AbstractButton {
                id: sb
                required property var modelData
                property bool on: seg.current === modelData.value
                height: parent.height
                width: Math.max(96, lbl.implicitWidth + 28)
                focusPolicy: Qt.StrongFocus
                hoverEnabled: true
                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.label
                Accessible.checked: on
                onClicked: seg.picked(modelData.value)
                Keys.onReturnPressed: clicked()
                Keys.onLeftPressed: nextItemInFocusChain(false).forceActiveFocus()
                Keys.onRightPressed: nextItemInFocusChain(true).forceActiveFocus()
                background: Rectangle {
                    radius: theme.radius - 6
                    color: sb.on ? theme.c.surface : (sb.hovered ? Qt.alpha(theme.c.surface, 0.5) : "transparent")
                    border.width: sb.visualFocus ? 2 : (sb.on && theme.highContrast ? 1 : 0)
                    border.color: sb.visualFocus ? theme.c.accent : theme.c.border
                    Behavior on color { enabled: theme.animations; ColorAnimation { duration: 140 } }
                }
                contentItem: Text {
                    id: lbl
                    text: sb.modelData.label
                    horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
                    color: theme.c.text
                    font.family: theme.font; font.pixelSize: Math.round(13 * theme.scale)
                    font.weight: sb.on ? Font.DemiBold : Font.Normal
                }
            }
        }
    }
}
