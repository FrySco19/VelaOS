import QtQuick
import QtQuick.Controls
import org.vela.tokens

CheckBox {
    id: c
    property color textColor: Tokens.text
    font.family: Tokens.fontUi; font.pointSize: 9.5
    spacing: 8
    indicator: Rectangle {
        x: c.leftPadding; y: c.topPadding + (c.availableHeight - height) / 2
        width: 18; height: 18; radius: 5
        color: c.checked ? (c.textColor === Tokens.negative ? Tokens.negative : Tokens.accent) : Tokens.view
        border.width: c.checked ? 0 : 1.5
        border.color: c.visualFocus ? Tokens.accent : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.35)
        Behavior on color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
        Canvas {
            anchors.fill: parent; visible: c.checked
            onPaint: { var x = getContext("2d"); x.reset(); x.strokeStyle = "white"; x.lineWidth = 2; x.lineCap = "round"
                       x.lineJoin = "round"; x.beginPath(); x.moveTo(4.5, 9.5); x.lineTo(7.5, 12.5); x.lineTo(13.5, 5.5); x.stroke() }
        }
    }
    contentItem: Text {
        text: c.text; font: c.font; color: c.textColor; wrapMode: Text.Wrap
        leftPadding: c.indicator.width + c.spacing; verticalAlignment: Text.AlignVCenter
    }
}
