import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "sun"
    title: backend.s["look.title"]
    body: backend.s["look.body"]

    Row {
        spacing: 14
        Repeater {
            model: [{v: "light", ic: "sun"}, {v: "dark", ic: "moon"}, {v: "auto", ic: "auto"}]
            ChoiceCard {
                required property var modelData
                width: 186
                height: 214
                title: backend.s["look." + modelData.v]
                hint: modelData.v === "auto" ? backend.autoHint : ""
                iconName: modelData.ic
                selected: backend.mode === modelData.v
                onClicked: backend.mode = modelData.v
                preview: Component { MiniDesktop { width: 158; height: 96; dark: modelData.v === "dark"; split: modelData.v === "auto" } }
            }
        }
    }

    SectionLabel { text: backend.s["look.accent"] }
    Row {
        spacing: 12
        Repeater {
            model: backend.accents
            AbstractButton {
                id: sw
                required property var modelData
                property bool on: backend.accent.toLowerCase() === modelData.color.toLowerCase()
                width: 34; height: 34
                focusPolicy: Qt.StrongFocus; hoverEnabled: true
                Accessible.role: Accessible.RadioButton
                Accessible.name: backend.s["accent." + modelData.id]
                Accessible.checked: on
                ToolTip.visible: hovered || visualFocus
                ToolTip.text: backend.s["accent." + modelData.id]
                ToolTip.delay: 300
                onClicked: backend.accent = modelData.color
                Keys.onReturnPressed: clicked()
                Keys.onLeftPressed: nextItemInFocusChain(false).forceActiveFocus()
                Keys.onRightPressed: nextItemInFocusChain(true).forceActiveFocus()
                background: Rectangle {
                    radius: width / 2
                    color: sw.modelData.color
                    border.width: sw.on || sw.visualFocus ? 3 : 0
                    border.color: theme.c.bg
                    Rectangle { anchors.fill: parent; anchors.margins: -3; radius: width / 2; color: "transparent"
                                border.width: 2; border.color: sw.on || sw.visualFocus ? sw.modelData.color : "transparent" }
                    scale: sw.hovered ? 1.08 : 1
                    Behavior on scale { enabled: theme.animations; NumberAnimation { duration: 120 } }
                }
                contentItem: Item { Icon { anchors.centerIn: parent; visible: sw.on; name: "check"; size: 18; color: "#FFFFFF"; stroke: 2.4 } }
            }
        }
    }
    Text {
        text: { for (const a of backend.accents) if (a.color.toLowerCase() === backend.accent.toLowerCase()) return backend.s["accent." + a.id]; return "" }
        color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale)
    }
}
