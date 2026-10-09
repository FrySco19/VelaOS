import QtQuick

// Theme icon through the Python image provider: name may be "a|b|c" (first available wins).
Item {
    id: root
    property string name: ""
    property int size: 16
    width: size; height: size
    implicitWidth: size; implicitHeight: size
    visible: name !== "" && bridge.hasIcon(name)
    Image {
        anchors.fill: parent
        sourceSize.width: root.size * 2; sourceSize.height: root.size * 2
        source: root.name ? "image://icon/" + root.name + "?g=" + bridge.iconGen : ""
        fillMode: Image.PreserveAspectFit
        smooth: true
    }
}
