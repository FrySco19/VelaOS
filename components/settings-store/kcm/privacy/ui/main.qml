import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Dialogs as QtDialogs
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: root

    function v(path) { kcm.revision; return kcm.value(path) }
    function setv(path, val) { kcm.setValue(path, val) }
    function td(s) { return i18nd("kcm_vela_privacy", s) }

    readonly property bool online: v("provider") === "openai"
    readonly property string onlineHost: {
        var u = String(v("openai.base_url") || "")
        var m = u.match(/^[a-z]+:\/\/([^\/:]+)/i)
        return m ? m[1] : u
    }
    readonly property bool cloud: online && kcm.isCloudUrl(String(v("openai.base_url") || ""))

    Component.onCompleted: kcm.refresh()

    header: Kirigami.InlineMessage {
        visible: kcm.lastError.length > 0
        type: Kirigami.MessageType.Error
        text: kcm.lastError
    }

    Kirigami.FormLayout {
        // ----- data leaving the computer -----
        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Data that leaves this computer") }

        RowLayout {
            Kirigami.FormData.label: root.td("Now:")
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth
                source: v("enabled") !== true || !root.cloud ? "security-high" : "security-medium"
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                textFormat: Text.PlainText
                text: v("enabled") !== true
                      ? root.td("The assistant is off. Nothing is sent.")
                      : !root.cloud
                        ? root.td("The assistant uses a model on this computer. Your requests are not sent anywhere.")
                        : v("privacy.allow_cloud") === true
                          ? i18nd("kcm_vela_privacy", "The assistant uses an online service. Your requests and the context you allow are sent to %1.", root.onlineHost)
                          : i18nd("kcm_vela_privacy", "An online service (%1) is selected, but sending is not allowed, so nothing is sent.", root.onlineHost)
            }
        }
        QQC2.CheckBox {
            Kirigami.FormData.label: root.td("Online services:")
            text: root.td("Allow sending requests to online services")
            checked: v("privacy.allow_cloud") === true
            onToggled: root.setv("privacy.allow_cloud", checked)
        }
        QQC2.CheckBox {
            text: root.td("Remove passwords and keys from text before sending it")
            checked: v("privacy.redact_secrets") === true
            onToggled: root.setv("privacy.redact_secrets", checked)
        }
        QQC2.Label {
            visible: kcm.actionCount > 0
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: i18ndp("kcm_vela_privacy", "In the history, %1 action sent data off this computer.",
                         "In the history, %1 actions sent data off this computer.", kcm.offMachineCount)
        }

        // ----- actions -----
        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("What the assistant can do") }

        component AskCombo: QQC2.ComboBox {
            property string key
            property bool allowAlways: false
            Layout.preferredWidth: Kirigami.Units.gridUnit * 14
            textRole: "label"; valueRole: "value"
            model: allowAlways
                   ? [ { value: "never", label: root.td("Never") },
                       { value: "ask", label: root.td("Ask every time") },
                       { value: "allow", label: root.td("Allow without asking") } ]
                   : [ { value: "never", label: root.td("Never") },
                       { value: "ask", label: root.td("Ask every time") } ]
            currentIndex: Math.max(0, indexOfValue(String(root.v(key) || "ask")))
            onActivated: root.setv(key, currentValue)
        }
        AskCombo { Kirigami.FormData.label: root.td("Run commands:"); key: "permissions.run_commands" }
        AskCombo { Kirigami.FormData.label: root.td("Browse the web for you:"); key: "permissions.web"; allowAlways: true }
        AskCombo { Kirigami.FormData.label: root.td("Change network settings:"); key: "permissions.network_settings" }
        AskCombo { Kirigami.FormData.label: root.td("Install apps or change the system:"); key: "permissions.system_changes" }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: root.td("With «Ask every time» you see a preview and confirm before anything runs.")
        }

        // ----- history -----
        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Action history") }

        QQC2.CheckBox {
            Kirigami.FormData.label: root.td("History:")
            text: root.td("Keep a history of what the assistant does")
            checked: v("privacy.log_actions") === true
            onToggled: root.setv("privacy.log_actions", checked)
        }
        QQC2.ComboBox {
            Kirigami.FormData.label: root.td("Keep for:")
            Layout.preferredWidth: Kirigami.Units.gridUnit * 14
            enabled: v("privacy.log_actions") === true
            textRole: "label"; valueRole: "value"
            model: [
                { value: 7, label: root.td("1 week") },
                { value: 30, label: root.td("1 month") },
                { value: 90, label: root.td("3 months") },
                { value: 365, label: root.td("1 year") },
                { value: 0, label: root.td("Always") }
            ]
            currentIndex: Math.max(0, indexOfValue(Number(v("privacy.log_retention_days"))))
            onActivated: root.setv("privacy.log_retention_days", currentValue)
        }

        QQC2.Frame {
            Kirigami.FormData.label: root.td("Recent actions:")
            Kirigami.FormData.labelAlignment: Qt.AlignTop
            Layout.preferredWidth: Kirigami.Units.gridUnit * 26
            Layout.preferredHeight: Kirigami.Units.gridUnit * 12
            implicitHeight: Kirigami.Units.gridUnit * 12
            implicitWidth: Kirigami.Units.gridUnit * 26
            padding: 1
            QQC2.ScrollView {
            anchors.fill: parent
            ListView {
                id: actionsList
                clip: true
                model: kcm.actions
                delegate: QQC2.ItemDelegate {
                    width: ListView.view.width
                    required property var modelData
                    hoverEnabled: false
                    down: false
                    contentItem: RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        Kirigami.Icon {
                            implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth
                            source: modelData.sent_off_machine ? "cloud-upload" : "computer"
                            QQC2.ToolTip.visible: hov.hovered
                            QQC2.ToolTip.text: modelData.sent_off_machine ? root.td("Sent to an online service") : root.td("Stayed on this computer")
                            HoverHandler { id: hov }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            QQC2.Label {
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                textFormat: Text.PlainText
                                text: String(modelData.summary || modelData.action || "")
                            }
                            QQC2.Label {
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                font: Kirigami.Theme.smallFont
                                opacity: 0.7
                                textFormat: Text.PlainText
                                text: [modelData.when, modelData.app,
                                       modelData.confirmed === true ? root.td("confirmed") : "",
                                       modelData.result && modelData.result !== "ok" ? String(modelData.result) : ""]
                                      .filter(function(x) { return x && String(x).length }).join(" · ")
                            }
                        }
                    }
                }
                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 2
                    visible: actionsList.count === 0
                    icon.name: "view-history"
                    text: root.td("No actions yet")
                    explanation: root.td("What the assistant does will appear here.")
                }
            }
            }
        }
        RowLayout {
            QQC2.Button {
                icon.name: "view-refresh"
                text: root.td("Refresh")
                onClicked: kcm.refresh()
            }
            QQC2.Button {
                icon.name: "document-open"
                text: root.td("Open file")
                enabled: kcm.actionCount > 0
                onClicked: kcm.openActionLog()
            }
            QQC2.Button {
                icon.name: "edit-clear-history"
                text: root.td("Clear history…")
                enabled: kcm.actionCount > 0
                onClicked: clearDialog.open()
            }
        }

        // ----- memory -----
        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Assistant memory") }

        QQC2.CheckBox {
            Kirigami.FormData.label: root.td("Memory:")
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            text: root.td("Remember files, pages and commands so you can find them by asking")
            checked: v("privacy.memory_enabled") === true
            onToggled: root.setv("privacy.memory_enabled", checked)
        }
        QQC2.CheckBox {
            enabled: v("privacy.memory_enabled") === true
            text: root.td("Pause memory")
            checked: v("privacy.memory_paused") === true
            onToggled: root.setv("privacy.memory_paused", checked)
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: root.td("Memory stays on this computer, in your home folder.")
        }

        ColumnLayout {
            Kirigami.FormData.label: root.td("Never remember:")
            Kirigami.FormData.labelAlignment: Qt.AlignTop
            Layout.preferredWidth: Kirigami.Units.gridUnit * 20
            enabled: v("privacy.memory_enabled") === true
            Repeater {
                model: { kcm.revision; return v("privacy.memory_exclude_paths") || [] }
                delegate: RowLayout {
                    required property var modelData
                    required property int index
                    Kirigami.Icon { source: "folder"; implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth }
                    QQC2.Label { Layout.fillWidth: true; text: modelData; elide: Text.ElideMiddle; textFormat: Text.PlainText }
                    QQC2.ToolButton {
                        icon.name: "list-remove"
                        display: QQC2.AbstractButton.IconOnly
                        text: root.td("Remove")
                        QQC2.ToolTip.visible: hovered; QQC2.ToolTip.text: text
                        onClicked: {
                            var l = (v("privacy.memory_exclude_paths") || []).slice()
                            l.splice(index, 1)
                            root.setv("privacy.memory_exclude_paths", l)
                        }
                    }
                }
            }
            Repeater {
                model: { kcm.revision; return v("privacy.memory_exclude_sites") || [] }
                delegate: RowLayout {
                    required property var modelData
                    required property int index
                    Kirigami.Icon { source: "globe"; implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth }
                    QQC2.Label { Layout.fillWidth: true; text: modelData; elide: Text.ElideMiddle; textFormat: Text.PlainText }
                    QQC2.ToolButton {
                        icon.name: "list-remove"
                        display: QQC2.AbstractButton.IconOnly
                        text: root.td("Remove")
                        QQC2.ToolTip.visible: hovered; QQC2.ToolTip.text: text
                        onClicked: {
                            var l = (v("privacy.memory_exclude_sites") || []).slice()
                            l.splice(index, 1)
                            root.setv("privacy.memory_exclude_sites", l)
                        }
                    }
                }
            }
            QQC2.Button {
                icon.name: "folder-new"
                text: root.td("Add folder…")
                onClicked: folderDialog.open()
            }
            RowLayout {
                QQC2.TextField {
                    id: siteField
                    Layout.fillWidth: true
                    placeholderText: root.td("Site, e.g. bank.example.com")
                    onAccepted: addSite.clicked()
                }
                QQC2.Button {
                    id: addSite
                    icon.name: "list-add"
                    text: root.td("Add site")
                    enabled: siteField.text.trim().length > 0
                    onClicked: {
                        var s = siteField.text.trim().replace(/^[a-z]+:\/\//i, "").replace(/\/.*$/, "").toLowerCase()
                        var l = (v("privacy.memory_exclude_sites") || []).slice()
                        if (s.length && l.indexOf(s) < 0) { l.push(s); root.setv("privacy.memory_exclude_sites", l) }
                        siteField.text = ""
                    }
                }
            }
        }
        RowLayout {
            Kirigami.FormData.label: root.td("Space used:")
            QQC2.Label { text: kcm.memorySize }
            QQC2.Button {
                icon.name: "edit-delete"
                text: root.td("Delete memory…")
                enabled: kcm.hasMemory
                onClicked: memoryDialog.open()
            }
        }
    }

    QtDialogs.FolderDialog {
        id: folderDialog
        title: root.td("Choose a folder the assistant must not remember")
        onAccepted: {
            var p = kcm.homeRelative(selectedFolder.toString())
            var l = (v("privacy.memory_exclude_paths") || []).slice()
            if (l.indexOf(p) < 0) { l.push(p); root.setv("privacy.memory_exclude_paths", l) }
        }
    }

    Kirigami.PromptDialog {
        id: clearDialog
        title: root.td("Clear the action history?")
        subtitle: root.td("The list of what the assistant has done will be deleted. This cannot be undone.")
        standardButtons: Kirigami.Dialog.NoButton
        customFooterActions: [
            Kirigami.Action { text: root.td("Clear"); icon.name: "edit-clear-history"; onTriggered: { kcm.clearActions(); clearDialog.close() } },
            Kirigami.Action { text: root.td("Cancel"); icon.name: "dialog-cancel"; onTriggered: clearDialog.close() }
        ]
    }
    Kirigami.PromptDialog {
        id: memoryDialog
        title: root.td("Delete the assistant memory?")
        subtitle: root.td("Everything the assistant remembers will be deleted. Your files are not touched. This cannot be undone.")
        standardButtons: Kirigami.Dialog.NoButton
        customFooterActions: [
            Kirigami.Action { text: root.td("Delete"); icon.name: "edit-delete"; onTriggered: { kcm.deleteMemory(); memoryDialog.close() } },
            Kirigami.Action { text: root.td("Cancel"); icon.name: "dialog-cancel"; onTriggered: memoryDialog.close() }
        ]
    }
}
