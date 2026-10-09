import QtQuick
import QtQuick.Controls
import org.vela.tokens

AbstractButton {
    id: b
    property string iconName: ""
    property string tip: ""
    property string glyph: ""          // shown when the icon theme has none of iconName
    property int iconSize: 16
    property bool round: false
    implicitWidth: 30; implicitHeight: 30
    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    Accessible.name: tip
    background: Rectangle {
        radius: b.round ? width / 2 : Tokens.radiusControl
        color: b.pressed ? Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.14)
             : b.hovered ? Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.08) : "transparent"
        border.width: b.visualFocus ? 2 : 0
        border.color: Tokens.accent
        Behavior on color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
    }
    contentItem: Item {
        Icon { id: ic; anchors.centerIn: parent; name: b.iconName; size: b.iconSize; opacity: b.enabled ? 0.85 : 0.35 }
        Text { anchors.centerIn: parent; visible: !ic.visible; text: b.glyph; color: Tokens.text
               opacity: b.enabled ? 0.75 : 0.35; font.pixelSize: b.iconSize + 2 }
    }
    ToolTip.visible: hovered && tip.length > 0
    ToolTip.delay: 600
    ToolTip.text: tip
}
