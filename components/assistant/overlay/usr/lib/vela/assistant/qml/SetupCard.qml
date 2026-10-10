import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.vela.tokens

// Shown instead of an answer when no model can be used. Never fakes an answer.
Rectangle {
    id: card
    property string message: ""
    property var info: ({})
    readonly property string state_: info.state || ""
    readonly property bool localCase: ["ollama_missing", "ollama_down", "no_model"].indexOf(state_) >= 0
    property int tab: (state_ === "no_key" || state_ === "cloud_blocked") ? 1 : 0
    property string chosen: bridge.localModels.length ? bridge.localModels[0].tag : ""
    readonly property var inst: bridge.install
    implicitHeight: col.implicitHeight + 32
    radius: Tokens.radiusCard
    color: Tokens.view
    border.width: 1
    border.color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.10)

    ColumnLayout {
        id: col
        anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 16
        spacing: 12

        RowLayout {
            spacing: 10
            Icon { name: "vela-assistant|help-browser"; size: 28 }
            ColumnLayout {
                spacing: 2; Layout.fillWidth: true
                Text { text: bridge.tr.setup_title; color: Tokens.text; font.family: Tokens.fontUi
                       font.pointSize: 12; font.weight: Font.DemiBold }
                Text { text: card.message; visible: text.length > 0; color: Tokens.textSecondary; wrapMode: Text.Wrap
                       font.family: Tokens.fontUi; font.pointSize: 9.5; Layout.fillWidth: true }
            }
        }

        // ---------- disabled
        RowLayout {
            visible: card.state_ === "local_loading"
            BusyIndicator { running: visible }
            VButton { text: "Riprova / Retry"; onClicked: bridge.retry() }
        }
        RowLayout {
            visible: card.state_ === "disabled"
            spacing: 8
            VButton { primary: true; text: bridge.tr.setup_enable; onClicked: bridge.enableAssistant() }
            VButton { text: bridge.tr.setup_open_settings; onClicked: bridge.openSettings() }
        }

        // ---------- segmented control
        Rectangle {
            visible: card.state_ !== "disabled" && card.state_ !== "local_loading"
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: seg.implicitWidth + 4; implicitHeight: 30
            radius: Tokens.radiusControl
            color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.07)
            Row {
                id: seg
                anchors.centerIn: parent
                Repeater {
                    model: [bridge.tr.setup_local_title, bridge.tr.setup_online_title]
                    AbstractButton {
                        id: sb
                        width: Math.max(150, segText.implicitWidth + 28); height: 26
                        onClicked: card.tab = index
                        hoverEnabled: true
                        Accessible.name: modelData
                        background: Rectangle {
                            radius: Tokens.radiusControl - 2
                            color: card.tab === index ? Tokens.view : "transparent"
                            border.width: card.tab === index ? 1 : 0
                            border.color: Qt.rgba(Tokens.text.r, Tokens.text.g, Tokens.text.b, 0.10)
                        }
                        contentItem: Text { id: segText; text: modelData; horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter; color: Tokens.text; font.family: Tokens.fontUi
                            font.pointSize: 9.5; font.weight: card.tab === index ? Font.DemiBold : Font.Normal }
                    }
                }
            }
        }

        // ---------- local model
        ColumnLayout {
            visible: card.state_ !== "disabled" && card.state_ !== "local_loading" && card.tab === 0
            spacing: 8
            Layout.fillWidth: true
            Text { text: bridge.tr.setup_local_desc; color: Tokens.textSecondary; wrapMode: Text.Wrap
                   font.family: Tokens.fontUi; font.pointSize: 9.5; Layout.fillWidth: true }
            Repeater {
                model: card.state_ === "ollama_down" ? [] : bridge.localModels
                delegate: VRadio {
                    text: modelData.tag
                    subtitle: modelData.label
                    checked: card.chosen === modelData.tag
                    enabled: !card.inst.active
                    onToggled: if (checked) card.chosen = modelData.tag
                }
            }
            Text {
                visible: card.state_ !== "ollama_down" && !card.inst.active
                text: bridge.downloadSummary(card.chosen)
                color: Tokens.textSecondary; font.family: Tokens.fontUi; font.pointSize: 9
            }
            // progress
            ColumnLayout {
                visible: card.inst.active || (card.inst.message || "") !== ""
                Layout.fillWidth: true
                spacing: 6
                ProgressBar {
                    visible: card.inst.active
                    Layout.fillWidth: true
                    from: 0; to: Math.max(1, card.inst.total || 0); value: card.inst.done || 0
                    indeterminate: !card.inst.total
                }
                Text { text: card.inst.message || ""; wrapMode: Text.Wrap; Layout.fillWidth: true
                       color: card.inst.phase === "error" ? Tokens.negative : card.inst.phase === "done" ? Tokens.positive : Tokens.textSecondary
                       font.family: Tokens.fontUi; font.pointSize: 9 }
            }
            RowLayout {
                spacing: 8
                Item { Layout.fillWidth: true }
                VButton { visible: card.inst.active; text: bridge.tr.install_cancel; onClicked: bridge.cancelSetup() }
                VButton {
                    visible: !card.inst.active && card.inst.phase !== "done"
                    primary: true
                    text: card.state_ === "ollama_down" ? bridge.tr.setup_local_start
                        : card.state_ === "no_model" ? bridge.tr.setup_local_pull : bridge.tr.setup_local_install
                    onClicked: card.state_ === "ollama_down" ? bridge.startOllama() : bridge.startLocalSetup(card.chosen)
                }
                VButton { visible: card.inst.phase === "done"; primary: true; text: bridge.tr.retry_question
                          onClicked: bridge.retry() }
            }
        }

        // ---------- online service
        ColumnLayout {
            visible: card.state_ !== "disabled" && card.state_ !== "local_loading" && card.tab === 1
            spacing: 8
            Layout.fillWidth: true
            Text { text: bridge.tr.setup_online_desc; color: Tokens.textSecondary; wrapMode: Text.Wrap
                   font.family: Tokens.fontUi; font.pointSize: 9.5; Layout.fillWidth: true }
            GridLayout {
                columns: 2; columnSpacing: 10; rowSpacing: 8; Layout.fillWidth: true
                Text { text: bridge.tr.setup_online_url; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9.5 }
                VField { id: url; text: "https://api.openai.com/v1"; Layout.fillWidth: true }
                Text { text: bridge.tr.setup_online_model; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9.5 }
                VField { id: mdl; text: "gpt-4o-mini"; Layout.fillWidth: true }
                Text { text: bridge.tr.setup_online_key; color: Tokens.text; font.family: Tokens.fontUi; font.pointSize: 9.5 }
                VField { id: key; echoMode: TextInput.Password; placeholderText: "sk-…"; Layout.fillWidth: true }
            }
            VCheck { id: consent; text: bridge.tr.setup_online_consent; Layout.fillWidth: true }
            Text { id: onlineErr; visible: text.length > 0; color: Tokens.negative; font.family: Tokens.fontUi; font.pointSize: 9 }
            RowLayout {
                Item { Layout.fillWidth: true }
                VButton { text: bridge.tr.setup_open_settings; onClicked: bridge.openSettings() }
                VButton { primary: true; text: bridge.tr.setup_online_save; enabled: consent.checked && mdl.text.length > 0
                          onClicked: { onlineErr.text = bridge.saveOnline(url.text, mdl.text, key.text, consent.checked)
                                       if (onlineErr.text === "") bridge.retry() } }
            }
        }

        Text {
            visible: card.state_ !== "disabled" && card.state_ !== "local_loading"
            text: bridge.tr.offline_hint; color: Tokens.textSecondary; wrapMode: Text.Wrap; Layout.fillWidth: true
            font.family: Tokens.fontUi; font.pointSize: 8.5
        }
    }
}
