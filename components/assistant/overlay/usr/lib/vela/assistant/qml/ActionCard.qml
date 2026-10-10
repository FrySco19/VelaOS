import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.vela.tokens

// Preview card for an action. State-changing actions wait here for the user's confirmation.
Rectangle {
    id: card
    property var c: ({})
    readonly property string st: c.state || ""
    readonly property var pv: c.preview || ({})
    readonly property bool danger: !!pv.danger
    readonly property var d: c.data || ({})
    property bool dangerAck: false
    implicitHeight: col.implicitHeight + 24
    radius: Tokens.radiusCard
    color: Tokens.view
    border.width: 1
    border.color: danger && st === "pending" ? Tokens.negative
                 : Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.10)

    ColumnLayout {
        id: col
        anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
        anchors.margins: 12
        spacing: 8

        RowLayout {
            spacing: 10
            Layout.fillWidth: true
            Rectangle {
                Layout.preferredWidth: 32; Layout.preferredHeight: 32; radius: Tokens.radiusControl
                color: Qt.rgba(Tokens.accent.r, Tokens.accent.g, Tokens.accent.b, 0.12)
                Icon { anchors.centerIn: parent; name: (card.c.icon || "") + "|dialog-information"; size: 18 }
            }
            ColumnLayout {
                spacing: 2
                Layout.fillWidth: true
                Text {
                    text: card.pv.title || ""
                    color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 10; font.weight: Font.DemiBold
                    wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                Repeater {
                    model: card.pv.detail || []
                    Text {
                        text: modelData; color: index === 0 || !card.danger ? Tokens.textSecondary : Tokens.negative
                        font.family: Tokens.fontUi; font.pointSize: 9; wrapMode: Text.Wrap; Layout.fillWidth: true
                    }
                }
            }
            // state badge
            Text {
                visible: card.st === "pending" && card.c.kind === "change"
                text: bridge.tr.card_pending; color: Tokens.neutral
                font.family: Tokens.fontUi; font.pointSize: 8; font.weight: Font.DemiBold
                Layout.alignment: Qt.AlignTop
            }
        }

        // command preview
        Rectangle {
            visible: !!card.pv.code
            Layout.fillWidth: true
            implicitHeight: codeText.implicitHeight + 16
            radius: Tokens.radiusControl
            color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06)
            TextEdit {
                id: codeText
                anchors.fill: parent; anchors.margins: 8
                text: card.pv.code || ""; readOnly: true; selectByMouse: true; wrapMode: Text.WrapAnywhere
                color: Tokens.text; font.family: Tokens.fontMono; font.pointSize: 9.5
            }
        }

        // danger acknowledgement
        VCheck {
            id: ack
            visible: card.danger && card.st === "pending"
            text: bridge.tr.confirm_danger
            textColor: Tokens.negative
            checked: card.dangerAck
            onToggled: card.dangerAck = checked
        }

        // pending: buttons
        RowLayout {
            visible: card.st === "pending"
            Layout.fillWidth: true
            spacing: 8
            Item { Layout.fillWidth: true }
            VButton { text: bridge.tr.cancel; onClicked: bridge.decline(card.c.id) }
            VButton {
                text: card.pv.confirm || bridge.tr.btn_confirm
                primary: !card.danger; danger: card.danger
                enabled: !card.danger || card.dangerAck
                onClicked: bridge.confirm(card.c.id, card.dangerAck)
            }
        }

        // running
        RowLayout {
            visible: card.st === "running"
            spacing: 8
            BusyIndicator { running: visible; implicitWidth: 18; implicitHeight: 18 }
            Text { text: bridge.tr.working; color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 9 }
            VButton { text: "Interrompi / Stop"; onClicked: bridge.cancelOperation(card.c.id) }
        }

        // result line
        RowLayout {
            visible: ["done", "failed", "declined", "undone"].indexOf(card.st) >= 0 && (card.c.message || "") !== ""
            Layout.fillWidth: true
            spacing: 6
            Text {
                text: card.st === "done" ? "✓" : card.st === "failed" ? "✕" : "↶"
                color: card.st === "done" ? Tokens.positive : card.st === "failed" ? Tokens.negative : Tokens.textSecondary
                font.pointSize: 10; font.weight: Font.Bold
            }
            Text {
                text: card.c.message || ""; Layout.fillWidth: true; wrapMode: Text.Wrap
                color: card.st === "failed" ? Tokens.negative : Tokens.text
                font.family: Tokens.fontUi; font.pointSize: 9.5
            }
            VButton {
                visible: !!card.c.can_undo && ["done", "failed"].indexOf(card.st) >= 0
                text: bridge.tr.undo
                onClicked: bridge.undo(card.c.id)
            }
        }

        Repeater {
            model: card.st === "done" && card.c.tool === "diagnose_audio" ? (card.d.sinks || []) : []
            delegate: RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true; wrapMode: Text.Wrap
                    text: modelData.description + (modelData.name === card.d.default ? " · " + bridge.tr.audio_default : "") +
                          (modelData.mute ? " · " + bridge.tr.audio_muted : "")
                    color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9
                }
                VButton {
                    visible: modelData.name !== card.d.default
                    text: bridge.tr.audio_choose
                    onClicked: bridge.proposeAudioOutput(modelData.name)
                }
            }
        }

        // find_files results
        Repeater {
            model: card.st === "done" && card.d.files ? card.d.files.slice(0, 8) : []
            delegate: Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                radius: Tokens.radiusControl
                color: fileMouse.containsMouse ? Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06) : "transparent"
                MouseArea { id: fileMouse; anchors.fill: parent; hoverEnabled: true; onDoubleClicked: bridge.openPath(modelData.path) }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 4; spacing: 8
                    Icon { name: modelData.is_dir ? "folder" : "text-x-generic|unknown"; size: 22 }
                    ColumnLayout {
                        spacing: 0; Layout.fillWidth: true
                        Text { text: modelData.name; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9.5
                               elide: Text.ElideMiddle; Layout.fillWidth: true }
                        Text { text: modelData.short + " · " + modelData.modified; color: Tokens.textSecondary
                               font.family: Tokens.fontUi; font.pointSize: 8.5; elide: Text.ElideMiddle; Layout.fillWidth: true }
                    }
                    VButton { text: bridge.tr.open_file; implicitHeight: 26; onClicked: bridge.openPath(modelData.path) }
                    IconButton { iconName: "folder-open|document-open-folder"; tip: bridge.tr.show_in_folder
                                 onClicked: bridge.reveal(modelData.path) }
                }
            }
        }

        // system_status
        GridLayout {
            visible: card.st === "done" && card.c.tool === "system_status"
            columns: 2; columnSpacing: 16; rowSpacing: 4
            Repeater {
                model: card.c.tool === "system_status" ? Object.keys(card.d) : []
                delegate: Text {
                    text: (bridge.tr["stat_" + modelData] || modelData) + ":  " + card.d[modelData]
                    color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9
                }
            }
        }

        // command output
        Rectangle {
            visible: card.c.tool === "run_command" && (card.d.output || "") !== ""
            Layout.fillWidth: true
            implicitHeight: Math.min(outText.implicitHeight + 16, 220)
            radius: Tokens.radiusControl
            color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.06)
            clip: true
            Flickable {
                anchors.fill: parent; anchors.margins: 8
                contentHeight: outText.implicitHeight; clip: true
                ScrollBar.vertical: ScrollBar {}
                TextEdit { id: outText; width: parent.width; text: card.d.output || ""; readOnly: true; selectByMouse: true
                           wrapMode: Text.WrapAnywhere; color: Tokens.text; font.family: Tokens.fontMono; font.pointSize: 9 }
            }
            IconButton { anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 4
                         iconName: "edit-copy"; tip: bridge.tr.copy; onClicked: bridge.copy(card.d.output) }
        }
    }
}
