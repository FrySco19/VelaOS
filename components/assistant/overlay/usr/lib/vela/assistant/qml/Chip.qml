import QtQuick
import org.vela.tokens

// Context chip: shows what the assistant will see; × removes it from this question.
Rectangle {
    id: chip
    property string iconName: ""
    property string label: ""
    property string value: ""
    property string thumb: ""
    property bool removable: true
    signal removed()
    radius: Tokens.radiusSmall + 4
    height: 26
    implicitWidth: row.implicitWidth + 12
    color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06)
    border.color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.10)
    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        x: 8
        spacing: 6
        Image { visible: chip.thumb !== ""; source: chip.thumb; height: 18; width: 28; anchors.verticalCenter: parent.verticalCenter
                fillMode: Image.PreserveAspectCrop; sourceSize.height: 36 }
        Icon { visible: chip.thumb === "" && chip.iconName !== ""
               name: chip.iconName; size: 14; anchors.verticalCenter: parent.verticalCenter }
        Text { text: chip.label; color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 9
               visible: text.length > 0; anchors.verticalCenter: parent.verticalCenter }
        Text { id: val; text: chip.value; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9
               elide: Text.ElideRight; width: Math.min(implicitWidth, 200); visible: text.length > 0
               anchors.verticalCenter: parent.verticalCenter }
        IconButton { visible: chip.removable; iconName: "window-close-symbolic|window-close"; glyph: "✕"; iconSize: 10
                     implicitWidth: 18; implicitHeight: 18; round: true; tip: bridge.tr.remove; onClicked: chip.removed()
                     anchors.verticalCenter: parent.verticalCenter }
    }
}
