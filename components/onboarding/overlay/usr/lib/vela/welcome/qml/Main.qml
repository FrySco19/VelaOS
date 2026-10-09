import QtQuick
import QtQuick.Controls.Basic

ApplicationWindow {
    id: win
    objectName: "mainWindow"
    width: 960; height: 660
    minimumWidth: 820; minimumHeight: 580
    visible: true
    title: backend.guideMode ? backend.s["guide.title"] : backend.s["app.title"]
    color: theme.c.bg
    font.family: theme.font
    palette.highlight: theme.c.accent
    palette.window: theme.c.bg
    palette.windowText: theme.c.text
    palette.base: theme.c.field
    palette.text: theme.c.text
    palette.button: theme.c.surface
    palette.buttonText: theme.c.text
    palette.toolTipBase: theme.c.surface
    palette.toolTipText: theme.c.text
    Behavior on color { enabled: theme.animations; ColorAnimation { duration: 220 } }

    Loader {
        id: view
        objectName: "view"
        anchors.fill: parent
        focus: true
        source: backend.guideMode ? "Guide.qml" : "Wizard.qml"
        onLoaded: item.focus = true
    }
    Connections {
        target: view.item
        ignoreUnknownSignals: true
        function onFinished() { win.close() }
    }
    Connections {
        target: backend
        function onMessage(m) { toast.text = m; toast.opacity = 1; toastTimer.restart() }
    }
    Rectangle {
        id: toast
        property alias text: tt.text
        opacity: 0; visible: opacity > 0
        anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; anchors.bottomMargin: 84
        width: Math.min(tt.implicitWidth + 32, parent.width - 80); height: tt.implicitHeight + 20; radius: theme.radius
        color: theme.c.text
        Text { id: tt; anchors.centerIn: parent; width: parent.width - 32; wrapMode: Text.WordWrap; color: theme.c.bg
               font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale) }
        Behavior on opacity { NumberAnimation { duration: 180 } }
        Accessible.role: Accessible.AlertMessage; Accessible.name: tt.text
    }
    Timer { id: toastTimer; interval: 5000; onTriggered: toast.opacity = 0 }
    onClosing: if (!backend.guideMode) backend.finish()
}
