import QtQuick
import QtQuick.Controls.Basic

// "Guida e scorciatoie": searchable shortcut cheatsheet + tips.
Item {
    id: g
    objectName: "guide"
    property string tab: "shortcuts"
    readonly property var cats: ["general", "windows", "spaces", "text", "screenshot", "browser", "terminal", "a11y"]
    function label(s) { return backend.lang === "it" ? s.it : s.en }
    function matches(s) {
        const q = search.text.trim().toLowerCase()
        if (!q) return true
        const keys = s.keys.map(c => c.map(k => backend.keyName(k)).join("+")).join(" ")
        return (label(s) + " " + keys).toLowerCase().indexOf(q) >= 0
    }
    readonly property var filtered: { search.text; backend.lang; return backend.shortcuts.filter(s => g.matches(s)) }

    Rectangle {
        id: head
        width: parent.width; height: 120
        color: theme.c.bg
        Column {
            x: 40; y: 26; width: parent.width - 80; spacing: 14
            Row {
                spacing: 12
                Rectangle { width: 36; height: 36; radius: 10; color: Qt.alpha(theme.c.accent, theme.dark ? 0.18 : 0.1)
                            Icon { anchors.centerIn: parent; name: "help"; size: 22; color: theme.c.accent } }
                Text { text: backend.s["guide.title"]; color: theme.c.text; font.family: theme.font; font.weight: Font.Bold
                       font.pixelSize: Math.round(24 * theme.scale); anchors.verticalCenter: parent.verticalCenter
                       Accessible.role: Accessible.Heading; Accessible.name: text }
            }
            Row {
                spacing: 14
                Segmented {
                    accessibleName: backend.s["guide.title"]
                    model: [{value: "shortcuts", label: backend.s["guide.shortcuts"]}, {value: "tips", label: backend.s["guide.tips"]}]
                    current: g.tab
                    onPicked: v => g.tab = v
                }
                VField { id: search; visible: g.tab === "shortcuts"; width: 280; label: backend.s["guide.search"]
                         placeholderText: backend.s["guide.search"]; focus: true }
            }
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: theme.c.border }
    }

    Flickable {
        id: fl
        anchors.top: head.bottom; anchors.bottom: foot.top; width: parent.width
        contentWidth: width; contentHeight: body.implicitHeight + 40
        clip: true; boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        Column {
            id: body
            x: 40; y: 20; width: fl.width - 80; spacing: 12
            // shortcuts
            Repeater {
                model: g.tab === "shortcuts" ? g.cats : []
                Column {
                    id: catCol
                    required property string modelData
                    readonly property var items: g.filtered.filter(s => s.cat === modelData)
                    visible: items.length > 0
                    width: body.width; spacing: 8
                    SectionLabel { text: backend.s["cat." + catCol.modelData] }
                    Panel {
                        Repeater {
                            model: catCol.items
                            Item {
                                required property var modelData
                                required property int index
                                width: parent.width; height: Math.max(Math.round(42 * theme.scale), lbl.implicitHeight + 16)
                                Accessible.role: Accessible.StaticText
                                Accessible.name: lbl.text + ": " + modelData.keys.map(c => c.map(k => backend.keyName(k)).join(" + ")).join(", ")
                                focus: false
                                Rectangle { visible: index > 0; width: parent.width - 24; x: 12; height: 1; color: Qt.alpha(theme.c.border, 0.7) }
                                Text { id: lbl; x: 14; width: parent.width * 0.55; anchors.verticalCenter: parent.verticalCenter
                                       wrapMode: Text.WordWrap; text: g.label(modelData); color: theme.c.text
                                       font.family: theme.font; font.pixelSize: Math.round(14 * theme.scale) }
                                Row {
                                    anchors.right: parent.right; anchors.rightMargin: 12; anchors.verticalCenter: parent.verticalCenter
                                    spacing: 10
                                    Repeater { model: modelData.keys
                                        Row { required property var modelData; required property int index; spacing: 10
                                              Text { visible: index > 0; text: backend.s["tour.or"]; color: theme.c.muted
                                                     font.family: theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
                                              KeyCap { keys: modelData } } }
                                }
                            }
                        }
                    }
                }
            }
            Text { visible: g.tab === "shortcuts" && g.filtered.length === 0; text: backend.s["guide.noResults"]
                   color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(14 * theme.scale) }
            Text { visible: g.tab === "shortcuts"; text: backend.s["key.metaNote"] + " " + backend.s["guide.footer"]
                   width: parent.width; wrapMode: Text.WordWrap
                   color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
            // tips
            Repeater {
                model: g.tab === "tips" ? backend.tips : []
                Rectangle {
                    required property var modelData
                    width: body.width; height: tc.implicitHeight + 28; radius: theme.radius
                    color: theme.c.surface; border.width: 1; border.color: theme.c.border
                    Accessible.role: Accessible.StaticText
                    Accessible.name: (backend.lang === "it" ? modelData.it : modelData.en).join(". ")
                    Column { id: tc; x: 18; y: 14; width: parent.width - 36; spacing: 4
                        Text { text: (backend.lang === "it" ? modelData.it : modelData.en)[0]; color: theme.c.text; font.family: theme.font
                               font.weight: Font.DemiBold; font.pixelSize: Math.round(14.5 * theme.scale) }
                        Text { text: (backend.lang === "it" ? modelData.it : modelData.en)[1]; width: parent.width; wrapMode: Text.WordWrap
                               color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(13 * theme.scale) } }
                }
            }
            Panel {
                visible: g.tab === "tips"
                VSwitch { width: parent.width; title: backend.s["guide.tipsToggle"]; checked: backend.tipsEnabled
                          onToggled: backend.tipsEnabled = checked }
            }
        }
    }
    Rectangle {
        id: foot
        anchors.bottom: parent.bottom; width: parent.width; height: 64; color: theme.c.bg
        Rectangle { width: parent.width; height: 1; color: theme.c.border }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 32; anchors.verticalCenter: parent.verticalCenter; spacing: 10
            VButton { text: backend.s["guide.reopen"]; iconName: "sail"; onClicked: backend.openWelcome() }
            VButton { kind: "primary"; text: backend.s["nav.close"]; onClicked: Qt.quit() }
        }
    }
    Shortcut { sequences: [StandardKey.Find]; onActivated: { g.tab = "shortcuts"; search.forceActiveFocus() } }
}
