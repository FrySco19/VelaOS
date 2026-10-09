import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "globe"
    title: backend.s["lang.title"]
    body: backend.s["lang.body"]

    SectionLabel { text: backend.s["lang.language"] }
    Segmented {
        accessibleName: backend.s["lang.language"]
        model: [{value: "it", label: "Italiano"}, {value: "en", label: "English"}]
        current: backend.lang
        onPicked: v => backend.lang = v
    }
    Text { text: backend.s["lang.relogin"]; color: theme.c.muted; width: parent.width; wrapMode: Text.WordWrap
           font.family: theme.font; font.pixelSize: Math.round(12.5 * theme.scale) }

    SectionLabel { text: backend.s["lang.keyboard"] }
    Panel {
        Repeater {
            model: backend.keyboards
            AbstractButton {
                id: kb
                required property string modelData
                property bool on: backend.keyboard === modelData
                width: parent.width; height: Math.round(40 * theme.scale)
                focusPolicy: Qt.StrongFocus; hoverEnabled: true
                Accessible.role: Accessible.RadioButton
                Accessible.name: backend.s["kb." + modelData]
                Accessible.checked: on
                onClicked: backend.keyboard = modelData
                Keys.onReturnPressed: clicked()
                background: Rectangle { radius: theme.radius - 4; color: kb.hovered ? theme.c.surface2 : "transparent"
                                        border.width: kb.visualFocus ? 2 : 0; border.color: theme.c.accent }
                contentItem: Item {
                    Text { text: backend.s["kb." + kb.modelData]; x: 14; anchors.verticalCenter: parent.verticalCenter
                           color: theme.c.text; font.family: theme.font; font.pixelSize: Math.round(14 * theme.scale)
                           font.weight: kb.on ? Font.DemiBold : Font.Normal }
                    Icon { visible: kb.on; name: "check"; size: 20; color: theme.c.accent
                           anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter }
                }
            }
        }
    }
    SectionLabel { text: backend.s["lang.test"] }
    VField { width: Math.min(parent.width, 460); label: backend.s["lang.test"]; placeholderText: backend.s["lang.testHint"] }
}
