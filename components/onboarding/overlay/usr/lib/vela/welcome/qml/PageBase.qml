import QtQuick
import QtQuick.Controls.Basic

// Scrollable page with a large icon, title and one-line explanation.
Flickable {
    id: page
    property string title
    property string body
    property string iconName
    default property alias content: inner.data
    contentWidth: width
    contentHeight: col.implicitHeight + 48
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar { policy: page.contentHeight > page.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff }
    Accessible.role: Accessible.Pane
    Accessible.name: title

    Column {
        id: col
        x: 48; y: 40
        width: page.width - 96
        spacing: 6
        Rectangle {
            visible: page.iconName !== ""
            width: 52; height: 52; radius: 14
            color: Qt.alpha(theme.c.accent, theme.dark ? 0.18 : 0.10)
            Icon { anchors.centerIn: parent; name: page.iconName; size: 28; color: theme.c.accent; stroke: 1.7 }
        }
        Item { width: 1; height: 8; visible: page.iconName !== "" }
        Text {
            text: page.title; width: parent.width; wrapMode: Text.WordWrap
            color: theme.c.text; font.family: theme.font; font.weight: Font.Bold
            font.pixelSize: Math.round(26 * theme.scale)
            Accessible.role: Accessible.Heading; Accessible.name: text
        }
        Text {
            text: page.body; visible: text !== ""; width: Math.min(parent.width, 560); wrapMode: Text.WordWrap
            color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(14.5 * theme.scale)
            lineHeight: 1.15
        }
        Item { width: 1; height: 18 }
        Column { id: inner; width: parent.width; spacing: 14 }
    }
}
