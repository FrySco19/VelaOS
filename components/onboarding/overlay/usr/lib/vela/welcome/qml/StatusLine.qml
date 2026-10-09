import QtQuick
import QtQuick.Controls.Basic
// kind: ok | warn | error | busy | info
Row {
    id: st
    property string kind: "info"
    property string text
    visible: text !== ""
    spacing: 8
    Accessible.role: Accessible.StaticText
    Accessible.name: text
    BusyIndicator { visible: st.kind === "busy"; running: visible; width: 20; height: 20 }
    Icon {
        visible: st.kind !== "busy"; size: 20
        name: st.kind === "ok" ? "done" : (st.kind === "info" ? "info" : "warn")
        color: st.kind === "ok" ? theme.c.success : (st.kind === "error" ? theme.c.danger : (st.kind === "warn" ? "#C98A00" : theme.c.muted))
    }
    Text {
        text: st.text; width: Math.min(implicitWidth, (st.parent ? st.parent.width : 500) - 32); wrapMode: Text.WordWrap
        color: st.kind === "error" ? theme.c.danger : theme.c.text
        font.family: theme.font; font.pixelSize: Math.round(13 * theme.scale)
        anchors.verticalCenter: parent.verticalCenter
    }
}
