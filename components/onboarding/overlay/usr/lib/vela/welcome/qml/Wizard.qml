import QtQuick
import QtQuick.Controls.Basic

Item {
    id: wiz
    objectName: "wizard"
    readonly property var steps: [
        {id: "welcome", icon: "sail", page: "PageWelcome.qml"},
        {id: "language", icon: "globe", page: "PageLanguage.qml"},
        {id: "network", icon: "wifi", page: "PageNetwork.qml"},
        {id: "appearance", icon: "sun", page: "PageAppearance.qml"},
        {id: "accessibility", icon: "a11y", page: "PageAccessibility.qml"},
        {id: "dock", icon: "dock", page: "PageDock.qml"},
        {id: "assistant", icon: "sparkle", page: "PageAssistant.qml"},
        {id: "privacy", icon: "shield", page: "PagePrivacy.qml"},
        {id: "tour", icon: "keyboard", page: "PageTour.qml"},
        {id: "done", icon: "done", page: "PageDone.qml"}
    ]
    property int current: 0
    property int reached: 0
    property bool forward: true
    readonly property bool last: current === steps.length - 1
    signal finished()

    function go(i) {
        if (i < 0 || i >= steps.length || i === current) return
        if (i > current) {
            const item = stack.currentItem
            if (item && typeof item.commit === "function") { if (item.commit() !== "") return }
        }
        forward = i > current
        current = i
        reached = Math.max(reached, i)
        stack.replace(null, Qt.resolvedUrl(steps[i].page), {}, theme.animations ? StackView.Transition : StackView.Immediate)
        focusTimer.restart()
    }
    function next() { if (last) { backend.finish(); finished() } else go(current + 1) }
    function back() { go(current - 1) }
    // for automated screenshots
    function jump(i) { forward = i >= current; current = i; reached = Math.max(reached, i)
                       stack.replace(null, Qt.resolvedUrl(steps[i].page), {}, StackView.Immediate) }

    Timer { id: focusTimer; interval: 50; onTriggered: if (stack.currentItem) stack.currentItem.forceActiveFocus() }

    // ------------------------------------------------ sidebar
    Rectangle {
        id: side
        width: 232; height: parent.height
        color: theme.c.sidebar
        Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: theme.c.border }
        Column {
            x: 16; y: 22; width: parent.width - 32
            spacing: 2
            Row {
                spacing: 10; leftPadding: 6; bottomPadding: 16
                Rectangle { width: 30; height: 30; radius: 9; color: theme.dark ? theme.c.surface2 : theme.c.navy
                            Icon { anchors.centerIn: parent; name: "sail"; size: 20; color: "#FFFFFF"; stroke: 1.7 } }
                Text { text: "Vela"; color: theme.c.text; font.family: theme.font; font.weight: Font.Bold
                       font.pixelSize: Math.round(17 * theme.scale); anchors.verticalCenter: parent.verticalCenter }
            }
            Repeater {
                model: wiz.steps
                AbstractButton {
                    id: sb
                    required property var modelData
                    required property int index
                    property bool on: index === wiz.current
                    property bool done: index < wiz.current
                    width: parent.width; height: Math.round(34 * theme.scale)
                    enabled: index <= wiz.reached
                    focusPolicy: Qt.TabFocus; hoverEnabled: true
                    Accessible.role: Accessible.PageTab
                    Accessible.name: backend.s["step." + modelData.id]
                    Accessible.selected: on
                    onClicked: wiz.go(index)
                    Keys.onReturnPressed: clicked()
                    background: Rectangle {
                        radius: theme.radius - 4
                        color: sb.on ? Qt.alpha(theme.c.accent, theme.dark ? 0.22 : 0.13) : (sb.hovered && sb.enabled ? Qt.alpha(theme.c.text, 0.05) : "transparent")
                        border.width: sb.visualFocus ? 2 : 0; border.color: theme.c.accent
                        Behavior on color { enabled: theme.animations; ColorAnimation { duration: 160 } }
                    }
                    contentItem: Item {
                        Icon { id: si; x: 8; anchors.verticalCenter: parent.verticalCenter; size: 18
                               name: sb.done ? "check" : sb.modelData.icon
                               color: sb.on || sb.done ? theme.c.accent : (sb.enabled ? theme.c.text : theme.c.muted) }
                        Text { anchors.left: si.right; anchors.leftMargin: 10; anchors.right: parent.right
                               anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight
                               text: backend.s["step." + sb.modelData.id]
                               color: sb.enabled ? theme.c.text : theme.c.muted
                               font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale)
                               font.weight: sb.on ? Font.DemiBold : Font.Normal }
                    }
                }
            }
        }
        VButton {
            x: 16; anchors.bottom: parent.bottom; anchors.bottomMargin: 18
            kind: "plain"; iconName: "a11y"; text: backend.s["a11y.button"]
            focusPolicy: Qt.TabFocus
            onClicked: { wiz.reached = Math.max(wiz.reached, 4); wiz.go(4) }
        }
    }

    // ------------------------------------------------ content
    StackView {
        id: stack
        anchors.left: side.right; anchors.right: parent.right
        anchors.top: parent.top; anchors.bottom: bar.top
        // Create the first page after the window/layout has acquired its size.
        // StackView's URL initialItem can stay invisible on first Wayland login.
        readonly property int dx: wiz.forward ? 36 : -36
        replaceEnter: Transition {
            ParallelAnimation {
                NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 240; easing.type: Easing.OutCubic }
                NumberAnimation { property: "x"; from: stack.dx; to: 0; duration: 320; easing.type: Easing.OutBack; easing.overshoot: 0.6 }
            }
        }
        replaceExit: Transition {
            ParallelAnimation {
                NumberAnimation { property: "opacity"; from: 1; to: 0; duration: 160; easing.type: Easing.InCubic }
                NumberAnimation { property: "x"; from: 0; to: -stack.dx * 0.6; duration: 200; easing.type: Easing.InCubic }
            }
        }
    }

    // ------------------------------------------------ bottom bar
    Rectangle {
        id: bar
        anchors.left: side.right; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: 68
        color: theme.c.bg
        Rectangle { width: parent.width; height: 1; color: theme.c.border }
        VButton {
            id: skipBtn
            visible: !wiz.last
            kind: "plain"; text: backend.s["nav.skipAll"]
            anchors.left: parent.left; anchors.leftMargin: 32; anchors.verticalCenter: parent.verticalCenter
            onClicked: { backend.finish(); wiz.finished() }
        }
        Text {
            anchors.centerIn: parent
            text: backend.s["nav.stepOf"].replace("%1", wiz.current + 1).replace("%2", wiz.steps.length)
            color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12 * theme.scale)
            visible: parent.width > 560
        }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 32; anchors.verticalCenter: parent.verticalCenter
            spacing: 10
            VButton { id: backBtn; objectName: "backButton"; visible: wiz.current > 0; text: backend.s["nav.back"]; onClicked: wiz.back() }
            VButton { id: nextBtn; objectName: "nextButton"; kind: "primary"
                      text: wiz.last ? backend.s["nav.start"] : backend.s["nav.next"]
                      onClicked: wiz.next() }
        }
    }

    // Return anywhere that doesn't use it itself (e.g. not in a text field) = Continua, like a default button
    Keys.onReturnPressed: wiz.next()
    Keys.onEnterPressed: wiz.next()
    Component.onCompleted: Qt.callLater(function() {
        stack.push(Qt.resolvedUrl(wiz.steps[0].page), {}, StackView.Immediate)
        focusTimer.restart()
    })
    Shortcut { sequences: ["Alt+Right"]; onActivated: wiz.next() }
    Shortcut { sequences: ["Alt+Left"]; onActivated: wiz.back() }
}
