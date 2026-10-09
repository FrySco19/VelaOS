import QtQuick
// Rounded grouped container (like grouped rows in macOS Settings)
Rectangle {
    default property alias content: c.data
    property alias spacing: c.spacing
    width: parent ? parent.width : 400
    implicitHeight: c.implicitHeight + 8
    radius: theme.radius
    color: theme.c.surface
    border.width: 1; border.color: theme.c.border
    Column { id: c; x: 4; y: 4; width: parent.width - 8; spacing: 0 }
}
