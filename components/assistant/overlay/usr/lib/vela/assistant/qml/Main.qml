import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.vela.tokens

Window {
    id: win
    title: bridge.tr.app_name
    flags: Qt.FramelessWindowHint | Qt.Window
    color: "transparent"
    readonly property int panelMargin: 18          // room for our own shadow (frameless windows get none)
    readonly property bool conversation: chat ? chat.count > 0 : false
    property string placeholderOverride: ""
    property string prefill: ""
    width: 720 + 2 * panelMargin
    height: (conversation ? 600 : compact.implicitHeight) + 2 * panelMargin
    minimumWidth: 520; minimumHeight: 200
    Behavior on height { enabled: bridge.animations && win.visible; NumberAnimation { duration: Tokens.normalMs; easing.type: Easing.OutCubic } }

    function focusInput() { input.forceActiveFocus() }
    Connections {
        target: bridge
        function onFocusInput() { win.focusInput() }
        function onToast(t) { toast.show(t) }
    }
    onVisibleChanged: if (visible) { focusInput(); appear.restart() }

    Shortcut { sequence: "Escape"; onActivated: win.hide() }
    Shortcut { sequences: ["Ctrl+N"]; onActivated: bridge.newChat() }
    Shortcut { sequences: ["Ctrl+W"]; onActivated: win.hide() }
    Shortcut { sequences: ["Ctrl+,"]; onActivated: bridge.openSettings() }

    // ------------------------------------------------------------------ panel + shadow
    // soft shadow: stacked translucent rounded rects (cheap on software rendering, no blur pass)
    Repeater {
        model: 12
        Rectangle {
            x: panel.x - index; y: panel.y - index + 6; width: panel.width + 2 * index; height: panel.height + 2 * index
            radius: panel.radius + index
            color: "transparent"
            border.width: 1
            border.color: Qt.rgba(0, 0, 0, (Tokens.dark ? 0.10 : 0.045) * (1 - index / 12))
            opacity: panel.opacity
        }
    }
    Rectangle {
        x: panel.x; y: panel.y + 2; width: panel.width; height: panel.height; radius: panel.radius
        color: Qt.rgba(0, 0, 0, Tokens.dark ? 0.25 : 0.10); opacity: panel.opacity
    }

    Rectangle {
        id: panel
        anchors.fill: parent
        anchors.margins: win.panelMargin
        radius: 16
        color: bridge.blur ? Tokens.glass : Qt.rgba(Tokens.window.r, Tokens.window.g, Tokens.window.b, 0.985)
        border.width: 1
        border.color: Tokens.dark ? Qt.rgba(1, 1, 1, 0.10) : Qt.rgba(0, 0, 0, 0.10)
        clip: true
        transformOrigin: Item.Top

        ParallelAnimation {
            id: appear
            NumberAnimation { target: panel; property: "opacity"; from: bridge.animations ? 0 : 1; to: 1; duration: Tokens.normalMs; easing.type: Easing.OutCubic }
            NumberAnimation { target: panel; property: "scale"; from: bridge.animations ? 0.97 : 1; to: 1; duration: Tokens.normalMs; easing.type: Easing.OutCubic }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // ---------------------------------------------------------- header
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 48
                MouseArea {
                    anchors.fill: parent
                    onPressed: win.startSystemMove()
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16; anchors.rightMargin: 10
                    spacing: 10
                    Icon { name: "vela-assistant|help-browser"; size: 22 }
                    Text {
                        text: bridge.tr.app_name
                        color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 11; font.weight: Font.DemiBold
                    }
                    // model status pill
                    Rectangle {
                        id: pill
                        readonly property var s: bridge.status
                        visible: (s.state || "") !== ""
                        Layout.preferredHeight: 22
                        Layout.preferredWidth: pillRow.implicitWidth + 16
                        radius: 11
                        color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06)
                        RowLayout {
                            id: pillRow
                            anchors.centerIn: parent
                            spacing: 6
                            Rectangle {
                                width: 7; height: 7; radius: 3.5
                                color: pill.s.state === "ready" ? Tokens.positive
                                     : pill.s.state === "disabled" ? Tokens.textSecondary : Tokens.neutral
                            }
                            Text {
                                text: pill.s.state === "ready"
                                      ? pill.s.model + "  ·  " + (pill.s.cloud ? pill.s.host : bridge.tr.model_local)
                                      : pill.s.state === "disabled" ? bridge.tr.state_disabled : bridge.tr.setup_title
                                color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 8.5
                                elide: Text.ElideRight
                                Layout.maximumWidth: 300
                            }
                        }
                        ToolTip.visible: pillMouse.containsMouse && (pill.s.note || "") !== ""
                        ToolTip.text: pill.s.note || ""
                        MouseArea { id: pillMouse; anchors.fill: parent; hoverEnabled: true; onClicked: bridge.openSettings() }
                    }
                    Item { Layout.fillWidth: true }
                    IconButton { iconName: "document-new|list-add"; glyph: "+"; tip: bridge.tr.new_chat + " (Ctrl+N)"; visible: win.conversation
                                 onClicked: bridge.newChat() }
                    IconButton {
                        id: menuBtn
                        iconName: "open-menu-symbolic|application-menu|open-menu"; glyph: "⋯"; tip: bridge.tr.more
                        onClicked: menu.popup(menuBtn, 0, menuBtn.height + 4)
                        Menu {
                            id: menu
                            MenuItem { text: bridge.tr.open_help_title; onTriggered: bridge.openHelp() }
                            MenuItem { text: bridge.tr.setup_open_settings; onTriggered: bridge.openSettings() }
                        }
                    }
                    IconButton { iconName: "window-close-symbolic|window-close"; glyph: "✕"; tip: bridge.tr.close + " (Esc)"; onClicked: win.hide() }
                }
            }

            // ---------------------------------------------------------- context chips
            Flow {
                id: chips
                Layout.fillWidth: true
                Layout.leftMargin: 16; Layout.rightMargin: 16
                Layout.bottomMargin: visible ? 8 : 0
                spacing: 6
                visible: children.length > 1 && (bridge.windowTitle !== "" || bridge.selection !== "" || bridge.clipboard !== ""
                         || bridge.files.length > 0 || bridge.screenshot !== "" || bridge.space !== "")
                Chip { visible: bridge.windowTitle !== ""; iconName: bridge.windowIcon + "|preferences-system-windows"
                       label: bridge.tr.active_window; value: bridge.windowTitle; onRemoved: bridge.removeChip("window") }
                Chip { visible: bridge.space !== ""; iconName: "virtual-desktops|preferences-desktop-virtual"
                       label: bridge.tr.space; value: bridge.space; onRemoved: bridge.removeChip("space") }
                Chip { visible: bridge.selection !== ""; iconName: "edit-select-text|edit-select-all"
                       label: bridge.tr.attach_selection; value: "«" + bridge.selection.replace(/\s+/g, " ") + "»"
                       onRemoved: bridge.removeChip("selection") }
                Chip { visible: bridge.clipboard !== ""; iconName: "edit-paste"
                       label: bridge.tr.attach_clipboard; value: "«" + bridge.clipboard.replace(/\s+/g, " ") + "»"
                       onRemoved: bridge.removeChip("clipboard") }
                Repeater {
                    model: bridge.files
                    Chip { iconName: modelData.kind === "dir" ? "folder" : "text-x-generic"; value: modelData.name
                           onRemoved: bridge.removeChip("file:" + modelData.path) }
                }
                Chip { visible: bridge.screenshot !== ""; thumb: bridge.screenshot; label: bridge.tr.screenshot_attached
                       onRemoved: bridge.removeChip("screenshot") }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.08) }

            // ---------------------------------------------------------- conversation / empty state
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: win.conversation

                ListView {
                    id: list
                    anchors.fill: parent
                    anchors.topMargin: 12; anchors.bottomMargin: 8
                    model: chat
                    spacing: 12
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                    onCountChanged: Qt.callLater(positionViewAtEnd)
                    onContentHeightChanged: if (atYEnd || bridge.busy) Qt.callLater(positionViewAtEnd)
                    delegate: Item {
                        id: row
                        width: list.width
                        height: loader.item ? loader.item.implicitHeight : 0
                        required property string kind
                        required property string text
                        required property var card
                        required property string uid
                        required property bool streaming
                        required property var extra
                        Loader {
                            id: loader
                            x: 16; width: parent.width - 32
                            sourceComponent: row.kind === "user" ? userC : row.kind === "assistant" ? asstC
                                           : row.kind === "card" ? cardC : row.kind === "setup" ? setupC : errorC
                        }
                        Component {
                            id: userC
                            Item {
                                implicitHeight: ub.height + (uChips.visible ? uChips.height + 4 : 0)
                                Text {
                                    id: uChips
                                    visible: (row.extra.chips || []).length > 0
                                    anchors.right: parent.right
                                    text: (row.extra.chips || []).join(" · ")
                                    color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 8.5
                                }
                                Rectangle {
                                    id: ub
                                    anchors.right: parent.right
                                    anchors.top: uChips.visible ? uChips.bottom : parent.top
                                    anchors.topMargin: uChips.visible ? 4 : 0
                                    width: Math.min(ut.implicitWidth + 28, parent.width * 0.78)
                                    height: ut.implicitHeight + 16
                                    radius: 14
                                    color: Tokens.accent
                                    TextEdit {
                                        id: ut
                                        anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14; anchors.topMargin: 8
                                        text: row.text; readOnly: true; selectByMouse: true; wrapMode: Text.Wrap
                                        color: Tokens.accentText; font.family: Tokens.fontUi; font.pointSize: 10
                                        selectionColor: Qt.rgba(1, 1, 1, 0.35)
                                    }
                                }
                            }
                        }
                        Component {
                            id: asstC
                            Item {
                                implicitHeight: Math.max(at.implicitHeight, row.streaming && row.text === "" ? 24 : 0) + 8
                                HoverHandler { id: aHover }
                                TypingDots { visible: row.streaming && row.text === ""; y: 8 }
                                TextEdit {
                                    id: at
                                    width: parent.width - 36
                                    text: row.text
                                    textFormat: TextEdit.MarkdownText
                                    readOnly: true; selectByMouse: true; wrapMode: Text.Wrap
                                    color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 10.5
                                    selectionColor: Qt.rgba(Tokens.accent.r, Tokens.accent.g, Tokens.accent.b, 0.35)
                                    onLinkActivated: (link) => Qt.openUrlExternally(link)
                                }
                                IconButton {
                                    anchors.right: parent.right; anchors.top: parent.top
                                    visible: (aHover.hovered || activeFocus) && !row.streaming && row.text !== ""
                                    iconName: "edit-copy"; glyph: "⧉"; tip: bridge.tr.copy; onClicked: bridge.copy(row.text)
                                }
                            }
                        }
                        Component { id: cardC; ActionCard { c: row.card } }
                        Component { id: setupC; SetupCard { message: row.text; info: row.extra } }
                        Component {
                            id: errorC
                            Rectangle {
                                implicitHeight: er.implicitHeight + 20
                                radius: Tokens.radiusCard
                                color: Qt.rgba(Tokens.negative.r, Tokens.negative.g, Tokens.negative.b, 0.08)
                                border.color: Qt.rgba(Tokens.negative.r, Tokens.negative.g, Tokens.negative.b, 0.35)
                                RowLayout {
                                    id: er
                                    anchors.left: parent.left; anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                                    anchors.leftMargin: 12; anchors.rightMargin: 8
                                    spacing: 8
                                    Text { text: row.text; color: Tokens.text; wrapMode: Text.Wrap; Layout.fillWidth: true
                                           font.family: Tokens.fontUi; font.pointSize: 9.5
                                           ToolTip.visible: errHover.hovered && (row.extra.detail || "") !== ""
                                           ToolTip.text: bridge.tr.details + ": " + (row.extra.detail || "")
                                           HoverHandler { id: errHover } }
                                    VButton { text: bridge.tr.retry; visible: !bridge.busy; onClicked: bridge.retry() }
                                }
                            }
                        }
                    }
                }
            }

            ColumnLayout {
                id: compact
                visible: !win.conversation
                Layout.fillWidth: true
                Layout.topMargin: 18; Layout.leftMargin: 16; Layout.rightMargin: 16; Layout.bottomMargin: 6
                spacing: 12
                implicitHeight: 48 + chips.height + (chips.visible ? 8 : 0) + 1 + 18 + emptyTitle.implicitHeight + 12
                                + sugg.implicitHeight + 12 + hint.implicitHeight + 6 + inputBox.height + 24
                Text {
                    id: emptyTitle
                    text: bridge.tr.empty_title
                    color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 15; font.weight: Font.DemiBold
                }
                Flow {
                    id: sugg
                    Layout.fillWidth: true
                    spacing: 8
                    Repeater {
                        model: [bridge.tr.suggest_1, bridge.tr.suggest_2, bridge.tr.suggest_3, bridge.tr.suggest_4]
                        AbstractButton {
                            id: sg
                            hoverEnabled: true
                            implicitHeight: 30; implicitWidth: sgt.implicitWidth + 24
                            onClicked: { input.text = modelData; bridge.send(modelData); input.text = "" }
                            background: Rectangle {
                                radius: 15
                                color: sg.hovered ? Qt.rgba(Tokens.accent.r, Tokens.accent.g, Tokens.accent.b, 0.14)
                                                  : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06)
                                Behavior on color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
                            }
                            contentItem: Text { id: sgt; text: modelData; color: Tokens.text; font.family: Tokens.fontUi
                                                font.pointSize: 9.5; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                        }
                    }
                }
                Text {
                    id: hint
                    visible: (bridge.status.state || "ready") !== "ready"
                    text: bridge.tr.offline_hint
                    color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 9; wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
                Item { Layout.fillHeight: true }
            }

            // ---------------------------------------------------------- input
            Rectangle {
                id: inputBox
                Layout.fillWidth: true
                Layout.leftMargin: 12; Layout.rightMargin: 12; Layout.bottomMargin: 12; Layout.topMargin: 4
                Layout.preferredHeight: Math.min(Math.max(44, input.implicitHeight + 16), 140)
                radius: 12
                color: Tokens.view
                border.width: input.activeFocus ? 2 : 1
                border.color: input.activeFocus ? Qt.rgba(Tokens.accent.r, Tokens.accent.g, Tokens.accent.b, 0.85)
                                                : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.14)
                Behavior on border.color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 6; anchors.rightMargin: 6
                    spacing: 2
                    IconButton { iconName: "camera-photo|view-preview|spectacle"; tip: bridge.tr.look_screen + " — " + bridge.tr.look_screen_tip
                                 Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: 7
                                 enabled: !bridge.busy; onClicked: bridge.lookAtScreen() }
                    IconButton { iconName: "edit-paste"; tip: bridge.tr.use_clipboard; visible: bridge.clipboard === ""
                                 Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: 7
                                 enabled: !bridge.busy; onClicked: bridge.useClipboard() }
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.topMargin: 4; Layout.bottomMargin: 4
                        TextArea {
                            id: input
                            placeholderText: bridge.busy ? bridge.tr.placeholder_busy
                                           : (win.placeholderOverride || bridge.tr.placeholder)
                            placeholderTextColor: Tokens.textSecondary
                            color: Tokens.text
                            font.family: Tokens.fontUi; font.pointSize: 10.5
                            wrapMode: TextEdit.Wrap
                            verticalAlignment: TextEdit.AlignVCenter
                            background: null
                            selectByMouse: true
                            Accessible.name: bridge.tr.placeholder
                            Keys.onPressed: (e) => {
                                if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && !(e.modifiers & Qt.ShiftModifier)) {
                                    e.accepted = true
                                    win.submit()
                                }
                            }
                        }
                    }
                    AbstractButton {
                        id: sendBtn
                        Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: 7
                        implicitWidth: 30; implicitHeight: 30
                        enabled: bridge.busy || input.text.trim().length > 0
                        Accessible.name: bridge.busy ? bridge.tr.stop : bridge.tr.send
                        onClicked: bridge.busy ? bridge.stop() : win.submit()
                        hoverEnabled: true
                        ToolTip.visible: hovered; ToolTip.delay: 600
                        ToolTip.text: bridge.busy ? bridge.tr.stop : bridge.tr.send + " (Invio)"
                        background: Rectangle {
                            radius: 15
                            color: sendBtn.enabled ? (sendBtn.hovered ? Qt.lighter(Tokens.accent, 1.08) : Tokens.accent)
                                                   : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.12)
                            Behavior on color { enabled: bridge.animations; ColorAnimation { duration: Tokens.fastMs } }
                        }
                        contentItem: Item {
                            Rectangle { visible: bridge.busy; anchors.centerIn: parent; width: 10; height: 10; radius: 2; color: "white" }
                            Canvas {   // up arrow, drawn so it never depends on the icon theme
                                id: arrow
                                visible: !bridge.busy
                                anchors.centerIn: parent; width: 14; height: 14
                                onPaint: {
                                    var ctx = getContext("2d"); ctx.reset()
                                    ctx.strokeStyle = sendBtn.enabled ? "white" : Tokens.textSecondary
                                    ctx.lineWidth = 2; ctx.lineCap = "round"; ctx.lineJoin = "round"
                                    ctx.beginPath(); ctx.moveTo(7, 12.5); ctx.lineTo(7, 2); ctx.moveTo(2.5, 6.5); ctx.lineTo(7, 2); ctx.lineTo(11.5, 6.5); ctx.stroke()
                                }
                                Connections { target: sendBtn; function onEnabledChanged() { arrow.requestPaint() } }
                            }
                        }
                    }
                }
            }
        }

        // ---------------------------------------------------------- toast
        Rectangle {
            id: toast
            property alias text: toastText.text
            function show(t) { text = t; opacity = 1; toastTimer.restart() }
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom; anchors.bottomMargin: 70
            width: toastText.implicitWidth + 28; height: 30; radius: 15
            color: Tokens.dark ? "#E3EEF0" : "#15303A"
            opacity: 0
            Behavior on opacity { enabled: bridge.animations; NumberAnimation { duration: Tokens.normalMs } }
            Text { id: toastText; anchors.centerIn: parent; color: Tokens.dark ? "#15232B" : "#F3F6F7"
                   font.family: Tokens.fontUi; font.pointSize: 9.5 }
            Timer { id: toastTimer; interval: 1600; onTriggered: toast.opacity = 0 }
        }
    }

    function submit() {
        var t = input.text.trim()
        if (t.length === 0 || bridge.busy) return
        bridge.send(t)
        input.text = ""
        win.placeholderOverride = ""
    }
}
