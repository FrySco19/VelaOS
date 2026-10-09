import QtQuick
import QtQuick.Controls.Basic

PageBase {
    id: p
    iconName: "keyboard"
    title: backend.s["tour.title"]
    body: backend.s["tour.body"]
    readonly property var cards: backend.shortcuts.filter(s => s.tour)
    property int idx: 0
    property bool playing: true
    // whole tour = 60 s
    Timer { interval: Math.round(60000 / Math.max(1, p.cards.length)); repeat: true; running: p.playing && p.visible; onTriggered: p.idx = (p.idx + 1) % p.cards.length }
    Keys.onLeftPressed: p.idx = (p.idx + p.cards.length - 1) % p.cards.length
    Keys.onRightPressed: p.idx = (p.idx + 1) % p.cards.length
    readonly property var cur: cards[idx] || ({})

    Rectangle {
        width: Math.min(parent.width, 640); height: inner.implicitHeight + 36
        radius: theme.radius + 2; color: theme.c.surface; border.width: 1; border.color: theme.c.border
        Accessible.role: Accessible.Grouping
        Accessible.name: (backend.lang === "it" ? p.cur.tourIt : p.cur.tourEn) + ". " + (backend.lang === "it" ? p.cur.descIt : p.cur.descEn)
        Row {
            id: inner
            x: 18; y: 18; spacing: 22
            TourScene { id: scene; kind: p.cur.tour || "search"; width: 300; height: 188 }
            Column {
                width: 260; spacing: 10
                anchors.verticalCenter: parent.verticalCenter
                Text { text: backend.s["tour.counter"].replace("%1", p.idx + 1).replace("%2", p.cards.length)
                       color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12 * theme.scale) }
                Text { text: backend.lang === "it" ? p.cur.tourIt : p.cur.tourEn; width: parent.width; wrapMode: Text.WordWrap
                       color: theme.c.text; font.family: theme.font; font.weight: Font.Bold; font.pixelSize: Math.round(19 * theme.scale) }
                Text { text: backend.lang === "it" ? p.cur.descIt : p.cur.descEn; width: parent.width; wrapMode: Text.WordWrap
                       color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(13.5 * theme.scale); lineHeight: 1.1 }
                Repeater {
                    model: p.cur.keys || []
                    Column {
                        required property var modelData
                        required property int index
                        spacing: 4
                        Text { visible: index > 0; text: backend.s["tour.or"]; color: theme.c.muted; font.family: theme.font; font.pixelSize: 12 }
                        KeyCap { keys: modelData; big: 1.15 }
                    }
                }
            }
        }
    }
    Row {
        spacing: 10
        VButton { iconName: "chevronLeft"; text: ""; Accessible.name: backend.s["nav.back"]; width: implicitHeight
                  leftPadding: 8; rightPadding: 8; onClicked: p.idx = (p.idx + p.cards.length - 1) % p.cards.length }
        VButton { iconName: p.playing ? "pause" : "play"; text: p.playing ? backend.s["tour.pause"] : backend.s["tour.play"]
                  onClicked: p.playing = !p.playing }
        VButton { iconName: "chevronRight"; text: ""; Accessible.name: backend.s["nav.next"]; width: implicitHeight
                  leftPadding: 8; rightPadding: 8; onClicked: p.idx = (p.idx + 1) % p.cards.length }
        Row {
            spacing: 6; anchors.verticalCenter: parent.verticalCenter; leftPadding: 8
            Repeater { model: p.cards.length
                Rectangle { width: index === p.idx ? 18 : 7; height: 7; radius: 4
                            color: index === p.idx ? theme.c.accent : theme.c.border
                            Behavior on width { enabled: theme.animations; NumberAnimation { duration: 180; easing.type: Easing.OutCubic } } } }
        }
    }
    Text { text: backend.s["key.metaNote"]; color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
}
