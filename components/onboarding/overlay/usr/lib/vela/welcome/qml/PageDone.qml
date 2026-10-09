import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "done"
    title: backend.s["done.title"]
    body: backend.s["done.body"]

    SectionLabel { text: backend.s["done.summary"] }
    Panel {
        Repeater {
            model: [
                {ic: "globe", t: backend.s["lang.language"] + ": " + (backend.lang === "it" ? "Italiano" : "English") + " · " + backend.s["done.kb"] + ": " + backend.s["kb." + backend.keyboard]},
                {ic: backend.mode === "dark" ? "moon" : (backend.mode === "auto" ? "auto" : "sun"), t: backend.s["look." + backend.mode]},
                {ic: "dock", t: backend.s["dock.position"] + ": " + backend.s["dock." + backend.dockPosition].toLowerCase()},
                {ic: "sparkle", t: backend.s[backend.aiChoice === "local" ? "done.aiLocal" : (backend.aiChoice === "online" ? "done.aiOnline" : "done.aiOff")]}
            ]
            Item {
                required property var modelData
                width: parent.width; height: Math.round(40 * theme.scale)
                Icon { x: 12; anchors.verticalCenter: parent.verticalCenter; name: modelData.ic; size: 20; color: theme.c.accent }
                Text { x: 46; anchors.verticalCenter: parent.verticalCenter; text: modelData.t; color: theme.c.text
                       font.family: theme.font; font.pixelSize: Math.round(14 * theme.scale) }
            }
        }
    }
    VButton { text: backend.s["done.help"]; iconName: "help"; onClicked: backend.openGuide() }
}
