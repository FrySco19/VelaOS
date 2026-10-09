import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "shield"
    title: backend.s["priv.title"]
    body: backend.s["priv.body"] + " " + backend.s["priv.local"]

    SectionLabel { text: backend.s["priv.see"] }
    Panel {
        VSwitch { width: parent.width; iconName: "window"; title: backend.s["priv.window"]; hint: backend.s["priv.windowHint"]
                  checked: backend.privGet("active_window"); onToggled: backend.privSet("active_window", checked) }
        VSwitch { width: parent.width; iconName: "text"; title: backend.s["priv.selection"]; hint: backend.s["priv.selectionHint"]
                  checked: backend.privGet("selected_text"); onToggled: backend.privSet("selected_text", checked) }
        VSwitch { width: parent.width; iconName: "clipboard"; title: backend.s["priv.clipboard"]; hint: backend.s["priv.clipboardHint"]
                  checked: backend.privGet("clipboard"); onToggled: backend.privSet("clipboard", checked) }
        VSwitch { width: parent.width; iconName: "monitor"; title: backend.s["priv.screen"]; hint: backend.s["priv.screenHint"]
                  checked: backend.privGet("screen"); onToggled: backend.privSet("screen", checked) }
        VSwitch { width: parent.width; iconName: "mic"; title: backend.s["priv.mic"]; hint: backend.s["priv.micHint"]
                  checked: backend.privGet("microphone"); onToggled: backend.privSet("microphone", checked) }
    }
    SectionLabel { text: backend.s["priv.remember"] }
    Panel {
        VSwitch { width: parent.width; iconName: "memory"; title: backend.s["priv.memory"]; hint: backend.s["priv.memoryHint"]
                  checked: backend.privGet("memory"); onToggled: backend.privSet("memory", checked) }
        VSwitch { width: parent.width; iconName: "list"; title: backend.s["priv.log"]; hint: backend.s["priv.logHint"]
                  checked: backend.privGet("log"); onToggled: backend.privSet("log", checked) }
    }
    Text { text: backend.s["priv.footer"]; color: theme.c.muted; width: parent.width; wrapMode: Text.WordWrap
           font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
}
