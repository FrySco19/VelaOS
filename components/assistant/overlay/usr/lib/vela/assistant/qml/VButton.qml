import QtQuick
import QtQuick.Controls
import org.vela.tokens

// Text button: primary (accent), danger (red) or plain.
Button {
    id: b
    property bool primary: false
    property bool danger: false
    hoverEnabled: true
    implicitHeight: 30
    leftPadding: 14; rightPadding: 14
    font.family: Tokens.fontUi
    font.pointSize: 10
    font.weight: primary || danger ? Font.DemiBold : Font.Medium
    readonly property color base: danger ? Tokens.negative : Tokens.accent
    contentItem: Text {
        text: b.text; font: b.font; elide: Text.ElideRight
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
        color: (b.primary || b.danger) ? "white" : Tokens.text
        opacity: b.enabled ? 1 : 0.5
    }
    background: Rectangle {
        radius: Tokens.radiusControl
        color: (b.primary || b.danger)
               ? (b.pressed ? Qt.darker(b.base, 1.15) : b.hovered ? Qt.lighter(b.base, 1.08) : b.base)
               : (b.pressed ? Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.16)
                            : b.hovered ? Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.10)
                                        : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06))
        opacity: b.enabled ? 1 : 0.55
        border.width: b.visualFocus ? 2 : 0
        border.color: Tokens.accent
        Behavior on color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
    }
}
