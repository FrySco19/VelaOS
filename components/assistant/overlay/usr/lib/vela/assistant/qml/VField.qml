import QtQuick
import QtQuick.Controls
import org.vela.tokens

TextField {
    id: f
    implicitHeight: 32
    font.family: Tokens.fontUi; font.pointSize: 10
    color: Tokens.text
    placeholderTextColor: Tokens.textSecondary
    selectByMouse: true
    leftPadding: 10; rightPadding: 10
    background: Rectangle {
        radius: Tokens.radiusControl
        color: Tokens.view
        border.width: f.activeFocus ? 2 : 1
        border.color: f.activeFocus ? Tokens.accent : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.18)
    }
}
