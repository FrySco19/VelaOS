import QtQuick

// Renders a key combination like Meta + Space as keycaps.
Row {
    id: kc
    property var keys: []          // ["Meta","Space"]
    property real big: 1.0
    spacing: 6 * big
    Accessible.role: Accessible.StaticText
    Accessible.name: keys.map(k => backend.keyName(k)).join(" + ")
    Repeater {
        model: kc.keys
        Row {
            required property var modelData
            required property int index
            spacing: 6 * kc.big
            Text { visible: index > 0; text: "+"; color: theme.c.muted; anchors.verticalCenter: parent.verticalCenter
                   font.family: theme.font; font.pixelSize: Math.round(13 * theme.scale * kc.big) }
            Rectangle {
                height: Math.round(28 * theme.scale * kc.big)
                width: Math.max(height, t.implicitWidth + 16 * kc.big)
                radius: 6 * kc.big
                color: theme.c.surface
                border.width: 1; border.color: theme.c.border
                Rectangle { anchors.bottom: parent.bottom; anchors.left: parent.left; anchors.right: parent.right
                            anchors.margins: 1; height: 3 * kc.big; radius: 3; color: Qt.alpha(theme.c.border, 0.9) }
                Text { id: t; anchors.centerIn: parent; anchors.verticalCenterOffset: -1
                       text: (backend.lang, backend.keyName(modelData)); color: theme.c.text
                       font.family: theme.font; font.weight: Font.DemiBold
                       font.pixelSize: Math.round(13 * theme.scale * kc.big) }
            }
        }
    }
}
