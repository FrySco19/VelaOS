import QtQuick
import QtQuick.Controls.Basic

PageBase {
    id: p
    iconName: "sparkle"
    title: backend.s["ai.title"]
    body: backend.s["ai.body"].replace("%1", backend.assistantShortcut.replace("Space", backend.keyName("Space")))
    property string stKind: ""
    property string stText: ""
    property string error: ""

    Connections {
        target: backend
        function onAiStatus(kind, msg) { p.stKind = kind; p.stText = msg }
    }
    Component.onCompleted: if (backend.aiChoice === "local") backend.checkLocal()

    // Called by the wizard before leaving the page. Returns "" or an error message.
    function commit() {
        const e = backend.saveAi(url.text, model.text, key.text, consent.checked)
        p.error = e ? backend.s[e] : ""
        return p.error
    }

    Row {
        spacing: 12
        Repeater {
            model: [{v: "local", ic: "cpu"}, {v: "online", ic: "cloud"}, {v: "off", ic: "off"}]
            ChoiceCard {
                required property var modelData
                width: 196
                height: 150
                title: backend.s["ai." + modelData.v]
                hint: backend.s["ai." + modelData.v + "Hint"]
                iconName: modelData.ic
                selected: backend.aiChoice === modelData.v
                onClicked: {
                    backend.aiChoice = modelData.v; p.stText = ""; p.error = ""
                    if (modelData.v === "local") backend.checkLocal()
                }
            }
        }
    }

    StatusLine { width: parent.width; visible: backend.aiChoice === "local" && text !== ""; kind: p.stKind; text: p.stText }

    Column {
        visible: backend.aiChoice === "local" && p.stKind === "warn"
        width: parent.width
        spacing: 8
        VButton {
            visible: backend.canSetupLocal
            text: backend.s["ai.setupLocal"]
            onClicked: backend.setupLocal()
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap
            text: backend.s["ai.setupLater"]
            color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale)
        }
    }

    Column {
        visible: backend.aiChoice === "online"
        width: parent.width
        spacing: 10
        Grid {
            columns: 2; columnSpacing: 14; rowSpacing: 10
            verticalItemAlignment: Grid.AlignVCenter
            Text { text: backend.s["ai.url"]; color: theme.c.text; font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale) }
            VField { id: url; width: 340; label: backend.s["ai.url"]; text: backend.aiGet("openai.base_url") }
            Text { text: backend.s["ai.model"]; color: theme.c.text; font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale) }
            VField { id: model; width: 340; label: backend.s["ai.model"]; text: backend.aiGet("openai.model") }
            Text { text: backend.s["ai.key"]; color: theme.c.text; font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale) }
            VField { id: key; width: 340; label: backend.s["ai.key"]; echoMode: TextInput.Password
                     placeholderText: backend.hasApiKey() ? "••••••••" : backend.s["ai.keyHint"] }
        }
        Text { text: backend.s["ai.keySaved"]; color: theme.c.muted; width: parent.width; wrapMode: Text.WordWrap
               font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
        Panel {
            VSwitch { id: consent; width: parent.width; iconName: "cloud"; title: backend.s["ai.consent"]
                      checked: backend.aiGet("privacy.allow_cloud") === "true" }
        }
        Row {
            spacing: 12
            VButton { text: backend.s["ai.test"]; enabled: p.stKind !== "busy"
                      onClicked: backend.testOnline(url.text, model.text, key.text) }
            StatusLine { kind: p.stKind; text: p.stText; anchors.verticalCenter: parent.verticalCenter }
        }
    }
    StatusLine { width: parent.width; kind: "error"; text: p.error }
}
