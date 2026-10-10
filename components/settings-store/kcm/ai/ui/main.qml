import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: root

    // re-evaluate bindings whenever the backend changes
    function v(path) { kcm.revision; return kcm.value(path) }
    function setv(path, val) { kcm.setValue(path, val) }
    function td(s) { return i18nd("kcm_vela_ai", s) }

    readonly property bool aiOn: v("enabled") === true
    readonly property bool online: v("provider") === "openai"
    readonly property bool packaged: String(v("openai.base_url")) === "http://127.0.0.1:11435/v1" && root.online
    readonly property string onlineHost: {
        kcm.revision
        var u = String(v("openai.base_url") || "")
        var m = u.match(/^[a-z]+:\/\/([^\/:]+)/i)
        return m ? m[1] : u
    }
    readonly property bool onlineIsCloud: kcm.isCloudUrl(String(v("openai.base_url") || ""))

    header: ColumnLayout {
        spacing: 0
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: kcm.lastError.length > 0
            type: Kirigami.MessageType.Error
            text: i18nd("kcm_vela_ai", "Cannot save the settings: %1", kcm.lastError)
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: root.aiOn && root.online && root.onlineIsCloud && v("privacy.allow_cloud") !== true
            type: Kirigami.MessageType.Warning
            text: root.td("The assistant will not work until you allow sending data to the online service.")
        }
    }

    Kirigami.FormLayout {
        id: form

        QQC2.Switch {
            Kirigami.FormData.label: root.td("Assistant:")
            text: root.td("Use the assistant")
            checked: root.aiOn
            onToggled: root.setv("enabled", checked)
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            wrapMode: Text.Wrap
            textFormat: Text.PlainText
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: root.aiOn
                  ? i18nd("kcm_vela_ai", "Open it with %1 from any app.", String(v("assistant.shortcut") || "Meta+A"))
                  : root.td("When it is off, no app can use a model or send anything.")
        }

        QQC2.CheckBox {
            enabled: root.aiOn
            text: root.td("Show the assistant in the top bar")
            checked: v("assistant.tray") === true
            onToggled: root.setv("assistant.tray", checked)
        }
        QQC2.CheckBox {
            enabled: root.aiOn
            text: root.td("Remember conversations")
            checked: v("assistant.history") === true
            onToggled: root.setv("assistant.history", checked)
        }

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Model") }

        QQC2.RadioButton {
            id: localRadio
            Kirigami.FormData.label: root.td("Runs on:")
            enabled: root.aiOn
            text: root.td("This computer (local model)")
            checked: root.packaged || !root.online
            onToggled: if (checked) {
                root.setv("provider", "openai")
                root.setv("openai.base_url", "http://127.0.0.1:11435/v1")
                root.setv("openai.model", "vela-local")
                root.setv("privacy.allow_cloud", false)
            }
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            leftPadding: localRadio.indicator.width + localRadio.spacing
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: root.td("Gemma 4 E2B is included and runs locally. No account or first-run download.")
        }
        QQC2.RadioButton {
            id: onlineRadio
            visible: !root.packaged
            enabled: root.aiOn
            text: root.td("An online service (OpenAI-compatible API)")
            checked: root.online && !root.packaged
            onToggled: if (checked) root.setv("provider", "openai")
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            leftPadding: onlineRadio.indicator.width + onlineRadio.spacing
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            visible: !root.packaged
            text: root.td("What you ask is sent to the service you choose.")
        }

        // ---------- local ----------
        QQC2.TextField {
            Kirigami.FormData.label: root.td("Ollama address:")
            visible: !root.online
            enabled: root.aiOn
            Layout.preferredWidth: Kirigami.Units.gridUnit * 16
            text: String(v("ollama.base_url") || "")
            placeholderText: "http://127.0.0.1:11434"
            onEditingFinished: root.setv("ollama.base_url", text.trim())
        }
        // ---------- online ----------
        QQC2.ComboBox {
            id: presetCombo
            Kirigami.FormData.label: root.td("Service:")
            visible: root.online && !root.packaged
            enabled: root.aiOn
            Layout.preferredWidth: Kirigami.Units.gridUnit * 16
            textRole: "label"
            readonly property var presets: [
                { label: "OpenAI", url: "https://api.openai.com/v1", model: "gpt-4o-mini" },
                { label: "OpenRouter", url: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini" },
                { label: "Groq", url: "https://api.groq.com/openai/v1", model: "llama-3.1-8b-instant" },
                { label: "Mistral AI", url: "https://api.mistral.ai/v1", model: "mistral-small-latest" },
                { label: "Google Gemini", url: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-2.0-flash" },
                { label: root.td("Other (enter the address)"), url: "", model: "" }
            ]
            model: presets
            currentIndex: {
                var u = String(v("openai.base_url") || "")
                for (var i = 0; i < presets.length - 1; i++) if (presets[i].url === u) return i
                return presets.length - 1
            }
            onActivated: function(i) {
                if (presets[i].url.length) {
                    root.setv("openai.base_url", presets[i].url)
                    root.setv("openai.model", presets[i].model)
                }
            }
        }
        QQC2.TextField {
            Kirigami.FormData.label: root.td("Address:")
            visible: root.online && !root.packaged
            enabled: root.aiOn
            Layout.preferredWidth: Kirigami.Units.gridUnit * 16
            text: String(v("openai.base_url") || "")
            placeholderText: "https://api.example.com/v1"
            onEditingFinished: root.setv("openai.base_url", text.trim())
        }
        Kirigami.PasswordField {
            Kirigami.FormData.label: root.td("API key:")
            visible: root.online && !root.packaged
            enabled: root.aiOn
            Layout.preferredWidth: Kirigami.Units.gridUnit * 16
            text: kcm.apiKey
            placeholderText: root.td("Paste the key from your account")
            onTextChanged: if (activeFocus) kcm.apiKey = text
        }
        QQC2.Label {
            visible: root.online && !root.packaged
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            color: Kirigami.Theme.textColor; opacity: 0.7
            text: root.td("The key is stored only on this computer, readable by your account only.")
        }
        RowLayout {
            Kirigami.FormData.label: root.td("Model:")
            enabled: root.aiOn
            QQC2.ComboBox {
                id: modelCombo
                Layout.preferredWidth: Kirigami.Units.gridUnit * 16
                editable: !root.packaged
                readonly property string key: root.online ? "openai.model" : "ollama.model"
                readonly property string current: String(v(key) || "")
                model: {
                    var l = kcm.models.slice()
                    if (current.length && l.indexOf(current) < 0) l.unshift(current)
                    return l
                }
                Component.onCompleted: editText = current
                onCurrentTextChanged: if (activeFocus || popup.visible) root.setv(key, currentText)
                onAccepted: root.setv(key, editText.trim())
                onActiveFocusChanged: if (!activeFocus && editText.trim() !== current) root.setv(key, editText.trim())
                Connections {
                    target: kcm
                    function onRevisionChanged() { if (!modelCombo.activeFocus) modelCombo.editText = modelCombo.current }
                }
            }
            QQC2.Button {
                icon.name: "view-refresh"
                text: root.td("Check connection")
                enabled: kcm.testState !== 1
                onClicked: kcm.testConnection()
            }
        }
        RowLayout {
            visible: kcm.testState !== 0
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth
                source: kcm.testState === 2 ? "dialog-ok-apply" : kcm.testState === 3 ? "dialog-error" : "content-loading-symbolic"
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                textFormat: Text.PlainText
                text: kcm.testMessage
            }
        }

        QQC2.Button {
            visible: !root.online && kcm.testState === 3 && kcm.canSetupLocal()
            enabled: root.aiOn
            icon.name: "run-install"
            text: root.td("Install Ollama…")
            onClicked: kcm.setupLocal()
        }
        RowLayout {
            Kirigami.FormData.label: root.td("Download a model:")
            visible: !root.online
            enabled: root.aiOn && !kcm.pulling
            QQC2.ComboBox {
                id: pullCombo
                Layout.preferredWidth: Kirigami.Units.gridUnit * 16
                editable: !root.packaged
                textRole: "label"
                valueRole: "tag"
                model: [
                    { tag: "qwen2.5:1.5b", label: "qwen2.5:1.5b — " + root.td("small, 1 GB") },
                    { tag: "gemma3:1b", label: "gemma3:1b — " + root.td("small, 0.8 GB") },
                    { tag: "llama3.2:3b", label: "llama3.2:3b — " + root.td("medium, 2 GB") },
                    { tag: "qwen2.5:7b", label: "qwen2.5:7b — " + root.td("large, 4.7 GB, needs 8 GB RAM") }
                ]
            }
            QQC2.Button {
                icon.name: "download"
                text: root.td("Download")
                onClicked: {
                    var t = pullCombo.currentIndex >= 0 && pullCombo.editText === pullCombo.currentText
                            ? pullCombo.currentValue : pullCombo.editText.split(" ")[0]
                    kcm.pullModel(t)
                }
            }
        }
        RowLayout {
            visible: !root.online && (kcm.pulling || kcm.pullStatus.length > 0)
            Layout.preferredWidth: Kirigami.Units.gridUnit * 22
            ColumnLayout {
                Layout.fillWidth: true
                QQC2.ProgressBar { Layout.fillWidth: true; visible: kcm.pulling; from: 0; to: 1; value: kcm.pullProgress; indeterminate: kcm.pullProgress <= 0 }
                QQC2.Label { Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText; text: kcm.pullStatus }
            }
            QQC2.Button { visible: kcm.pulling; text: root.td("Cancel"); onClicked: kcm.cancelPull() }
        }

        QQC2.CheckBox {
            Kirigami.FormData.label: root.td("Consent:")
            visible: root.online && root.onlineIsCloud
            enabled: root.aiOn
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            text: i18nd("kcm_vela_ai", "Allow sending my requests to %1", root.onlineHost)
            checked: v("privacy.allow_cloud") === true
            onToggled: root.setv("privacy.allow_cloud", checked)
        }

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Answers") }

        QQC2.ComboBox {
            Kirigami.FormData.label: root.td("Language:")
            Layout.preferredWidth: Kirigami.Units.gridUnit * 12
            enabled: root.aiOn
            textRole: "label"; valueRole: "value"
            model: [
                { value: "auto", label: root.td("Same as the system") },
                { value: "it", label: "Italiano" },
                { value: "en", label: "English" }
            ]
            currentIndex: Math.max(0, indexOfValue(String(v("language") || "auto")))
            onActivated: root.setv("language", currentValue)
        }
        RowLayout {
            Kirigami.FormData.label: root.td("Style:")
            enabled: root.aiOn
            QQC2.Label { text: root.td("Precise") }
            QQC2.Slider {
                Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                from: 0; to: 1; stepSize: 0.1; snapMode: QQC2.Slider.SnapAlways
                value: Number(v("generation.temperature") || 0)
                onMoved: root.setv("generation.temperature", Math.round(value * 10) / 10)
            }
            QQC2.Label { text: root.td("Creative") }
        }

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("What the assistant can use") }

        QQC2.CheckBox {
            Kirigami.FormData.label: root.td("Context:")
            enabled: root.aiOn
            text: root.td("Name of the active window and app")
            checked: v("permissions.active_window") === true
            onToggled: root.setv("permissions.active_window", checked)
        }
        QQC2.CheckBox {
            enabled: root.aiOn
            text: root.td("Text selected when you open it")
            checked: v("permissions.selected_text") === true
            onToggled: root.setv("permissions.selected_text", checked)
        }
        QQC2.CheckBox {
            enabled: root.aiOn
            text: root.td("Clipboard contents, without asking")
            checked: v("permissions.clipboard") === true
            onToggled: root.setv("permissions.clipboard", checked)
        }

        component AskCombo: QQC2.ComboBox {
            property string key
            property bool allowHome: false
            Layout.preferredWidth: Kirigami.Units.gridUnit * 14
            enabled: root.aiOn
            textRole: "label"; valueRole: "value"
            model: allowHome
                   ? [ { value: "never", label: root.td("Never") },
                       { value: "ask", label: root.td("Ask every time") },
                       { value: "home", label: root.td("Home folder, without asking") } ]
                   : [ { value: "never", label: root.td("Never") },
                       { value: "ask", label: root.td("Ask every time") } ]
            currentIndex: Math.max(0, indexOfValue(String(root.v(key) || "ask")))
            onActivated: root.setv(key, currentValue)
        }
        AskCombo { Kirigami.FormData.label: root.td("Files:"); key: "permissions.files"; allowHome: true }
        AskCombo { Kirigami.FormData.label: root.td("Screen:"); key: "permissions.screen" }
        AskCombo { Kirigami.FormData.label: root.td("Microphone:"); key: "permissions.microphone" }

        QQC2.Button {
            Kirigami.FormData.label: " "
            icon.name: "preferences-system-privacy"
            text: root.td("Privacy and memory…")
            onClicked: KCM.KCMLauncher.openSystemSettings("kcm_vela_privacy")
        }
    }
}
