import QtQuick
import QtQuick.Controls
import org.vela.tokens

// Radio with a title and a secondary line.
RadioButton {
    id: r
    property string subtitle: ""
    font.family: Tokens.fontUi; font.pointSize: 10
    spacing: 10
    indicator: Rectangle {
        x: r.leftPadding; y: r.topPadding + 2
        width: 18; height: 18; radius: 9
        color: Tokens.view
        border.width: r.checked ? 5 : 1.5
        border.color: r.checked ? Tokens.accent : (r.visualFocus ? Tokens.accent : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.35))
        Behavior on border.width { enabled: bridge.animations; NumberAnimation { duration: Tokens.fastMs } }
    }
    contentItem: Column {
        leftPadding: r.indicator.width + r.spacing
        spacing: 1
        Text { text: r.text; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 10; font.weight: Font.Medium }
        Text { text: r.subtitle; visible: text.length > 0; color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 9 }
    }
}
