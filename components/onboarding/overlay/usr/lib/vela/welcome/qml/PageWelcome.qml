import QtQuick
import QtQuick.Controls.Basic

PageBase {
    id: p
    iconName: "sail"
    title: backend.s["welcome.title"]
    body: backend.s["welcome.body"]

    SectionLabel { text: backend.lang === "it" ? "Lingua · Language" : "Language · Lingua" }
    Segmented {
        accessibleName: backend.s["lang.language"]
        model: [{value: "it", label: "Italiano"}, {value: "en", label: "English"}]
        current: backend.lang
        onPicked: v => backend.lang = v
    }

    Item { width: 1; height: 10 }

    Panel {
        visible: backend.live
        Item {
            width: parent.width
            height: Math.max(liveCol.implicitHeight, installBtn.height) + 32
            Icon { id: liveIc; name: "info"; size: 26; color: theme.c.accent; x: 16; y: 18 }
            Column {
                id: liveCol
                anchors.left: liveIc.right; anchors.leftMargin: 14
                anchors.right: installBtn.left; anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3
                Text { text: backend.s["welcome.liveTitle"]; width: parent.width; wrapMode: Text.WordWrap
                       color: theme.c.text; font.family: theme.font; font.weight: Font.Medium; font.pixelSize: Math.round(14 * theme.scale) }
                Text { text: backend.s["welcome.live"]; width: parent.width; wrapMode: Text.WordWrap
                       color: theme.c.muted; font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }
            }
            VButton {
                id: installBtn
                objectName: "installButton"
                iconName: "download"; text: backend.s["welcome.install"]
                anchors.right: parent.right; anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                onClicked: backend.launchInstaller()
            }
        }
    }
}
