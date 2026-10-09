import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Dialogs as QtDialogs
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: root
    function td(s) { return i18nd("kcm_vela_aspetto", s) }
    readonly property var st: kcm.state
    readonly property string curAccent: String(st.accent || "default").toUpperCase()

    header: ColumnLayout {
        spacing: 0
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: !kcm.available
            type: Kirigami.MessageType.Warning
            text: root.td("The Vela appearance tool is not installed. Use Global theme and Colors below.")
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: kcm.error.length > 0
            type: Kirigami.MessageType.Error
            text: kcm.error
        }
    }

    // Mini preview of a window in light or dark
    component ModeTile: QQC2.AbstractButton {
        id: tile
        property string mode
        property string label
        property bool dark: false
        property bool split: false
        checkable: true
        checked: String(root.st.mode) === mode
        enabled: kcm.available
        implicitWidth: Kirigami.Units.gridUnit * 7
        implicitHeight: Kirigami.Units.gridUnit * 6.5
        onClicked: kcm.setMode(mode)
        Accessible.name: label
        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Kirigami.Units.gridUnit * 4.5
                radius: 12
                clip: true
                border.width: tile.checked ? 3 : 1
                border.color: tile.checked ? Kirigami.Theme.highlightColor : Kirigami.Theme.disabledTextColor
                color: tile.dark ? "#15232B" : "#F3F6F7"
                Rectangle {
                    visible: tile.split
                    anchors { top: parent.top; bottom: parent.bottom; right: parent.right; margins: parent.border.width }
                    width: (parent.width - 2 * parent.border.width) / 2
                    color: "#15232B"
                    radius: 10
                    Rectangle { anchors { top: parent.top; bottom: parent.bottom; left: parent.left } width: 10; color: parent.color }
                }
                Rectangle { // window
                    x: parent.width * 0.16; y: parent.height * 0.22
                    width: parent.width * 0.68; height: parent.height * 0.6
                    radius: 6
                    color: tile.dark ? "#1A2A33" : "#FFFFFF"
                    border.width: 1
                    border.color: tile.dark ? "#2A3E48" : "#D5E0E3"
                    Row {
                        x: 6; y: 6; spacing: 3
                        Repeater { model: ["#E5806C", "#E3C27E", "#7FB49E"]; Rectangle { width: 6; height: 6; radius: 3; color: modelData } }
                    }
                    Rectangle { x: 6; y: 18; width: parent.width * 0.5; height: 5; radius: 2; color: tile.dark ? "#93AAB1" : "#587279"; opacity: 0.6 }
                    Rectangle { x: 6; y: 28; width: parent.width * 0.35; height: 8; radius: 4; color: Kirigami.Theme.highlightColor }
                }
            }
            QQC2.Label {
                Layout.alignment: Qt.AlignHCenter
                text: tile.label
                font.weight: tile.checked ? Font.DemiBold : Font.Normal
            }
        }
    }

    Kirigami.FormLayout {
        RowLayout {
            Kirigami.FormData.label: root.td("Mode:")
            spacing: Kirigami.Units.largeSpacing
            ModeTile { mode: "light"; label: root.td("Light") }
            ModeTile { mode: "dark"; label: root.td("Dark"); dark: true }
            ModeTile { mode: "auto"; label: root.td("Automatic"); split: true }
        }
        RowLayout {
            visible: String(root.st.mode) === "auto"
            enabled: kcm.available
            QQC2.Label { text: root.td("Light from") }
            QQC2.TextField {
                id: dayField
                implicitWidth: Kirigami.Units.gridUnit * 4
                inputMask: "99:99"
                text: String(root.st.day || "07:00")
                onEditingFinished: if (/^([01]\d|2[0-3]):[0-5]\d$/.test(text)) kcm.setSchedule(text, nightField.text)
            }
            QQC2.Label { text: root.td("dark from") }
            QQC2.TextField {
                id: nightField
                implicitWidth: Kirigami.Units.gridUnit * 4
                inputMask: "99:99"
                text: String(root.st.night || "19:30")
                onEditingFinished: if (/^([01]\d|2[0-3]):[0-5]\d$/.test(text)) kcm.setSchedule(dayField.text, text)
            }
        }

        Item { Kirigami.FormData.isSection: true }

        RowLayout {
            Kirigami.FormData.label: root.td("Accent color:")
            spacing: Kirigami.Units.smallSpacing
            enabled: kcm.available
            Repeater {
                model: [
                    { c: "default", hex: "#D4624A", name: root.td("Coral (default)") },
                    { c: "#1C6F86", hex: "#1C6F86", name: root.td("Sea blue") },
                    { c: "#2F80ED", hex: "#2F80ED", name: root.td("Blue") },
                    { c: "#2E8B57", hex: "#2E8B57", name: root.td("Green") },
                    { c: "#8E5BD0", hex: "#8E5BD0", name: root.td("Purple") },
                    { c: "#D6457A", hex: "#D6457A", name: root.td("Pink") },
                    { c: "#D9822B", hex: "#D9822B", name: root.td("Orange") },
                    { c: "#5F6B73", hex: "#5F6B73", name: root.td("Graphite") }
                ]
                delegate: QQC2.AbstractButton {
                    required property var modelData
                    readonly property bool sel: root.curAccent === String(modelData.c).toUpperCase()
                    implicitWidth: Kirigami.Units.gridUnit * 1.6
                    implicitHeight: implicitWidth
                    onClicked: kcm.setAccent(modelData.c)
                    Accessible.name: modelData.name
                    QQC2.ToolTip.visible: hovered
                    QQC2.ToolTip.text: modelData.name
                    contentItem: Rectangle {
                        radius: width / 2
                        color: modelData.hex
                        border.width: parent.sel ? 3 : 0
                        border.color: Kirigami.Theme.textColor
                        Kirigami.Icon {
                            anchors.centerIn: parent
                            visible: parent.parent.sel
                            width: parent.width * 0.6; height: width
                            source: "checkmark"
                            color: "white"
                            isMask: true
                        }
                    }
                }
            }
            QQC2.Button {
                icon.name: "color-picker"
                text: root.td("Other…")
                onClicked: colorDialog.open()
            }
        }

        Item { Kirigami.FormData.isSection: true }

        QQC2.Button {
            Kirigami.FormData.label: root.td("Wallpaper:")
            icon.name: "preferences-desktop-wallpaper"
            text: root.td("Choose wallpaper…")
            onClicked: KCM.KCMLauncher.openSystemSettings("kcm_wallpaper")
        }

        Item { Kirigami.FormData.isSection: true }

        QQC2.ComboBox {
            Kirigami.FormData.label: root.td("Transparency and effects:")
            enabled: kcm.available
            Layout.preferredWidth: Kirigami.Units.gridUnit * 14
            textRole: "label"; valueRole: "value"
            model: [
                { value: "full", label: root.td("Full") },
                { value: "light", label: root.td("Light (faster on older computers)") }
            ]
            currentIndex: Math.max(0, indexOfValue(String(root.st.effects || "full")))
            onActivated: kcm.setEffects(currentValue)
        }
        QQC2.Switch {
            Kirigami.FormData.label: root.td("Accessibility:")
            enabled: kcm.available
            text: root.td("Reduce motion")
            checked: String(root.st.motion) === "reduced"
            onToggled: kcm.setMotion(checked ? "reduced" : "normal")
        }
        QQC2.Switch {
            enabled: kcm.available
            text: root.td("Larger text")
            checked: String(root.st.text) === "large"
            onToggled: kcm.setText(checked ? "large" : "normal")
        }
        QQC2.Switch {
            enabled: kcm.available
            text: root.td("High contrast")
            checked: root.st.high_contrast === true
            onToggled: kcm.setHighContrast(checked)
        }

        Item { Kirigami.FormData.isSection: true }

        Flow {
            Kirigami.FormData.label: root.td("More options:")
            Layout.maximumWidth: Kirigami.Units.gridUnit * 26
            spacing: Kirigami.Units.smallSpacing
            QQC2.Button { flat: true; visible: kcm.hasDesktopSettings(); icon.name: "preferences-desktop"; text: root.td("Desktop and Dock…"); onClicked: kcm.openDesktopSettings() }
            QQC2.Button { flat: true; icon.name: "preferences-desktop-theme-global"; text: root.td("Global theme"); onClicked: KCM.KCMLauncher.openSystemSettings("kcm_lookandfeel") }
            QQC2.Button { flat: true; icon.name: "preferences-desktop-color"; text: root.td("Colors"); onClicked: KCM.KCMLauncher.openSystemSettings("kcm_colors") }
            QQC2.Button { flat: true; icon.name: "preferences-desktop-font"; text: root.td("Text and fonts"); onClicked: KCM.KCMLauncher.openSystemSettings("kcm_fonts") }
            QQC2.Button { flat: true; icon.name: "preferences-desktop-icons"; text: root.td("Icons"); onClicked: KCM.KCMLauncher.openSystemSettings("kcm_icons") }
            QQC2.Button { flat: true; icon.name: "preferences-desktop-cursors"; text: root.td("Cursor"); onClicked: KCM.KCMLauncher.openSystemSettings("kcm_cursortheme") }
        }
    }

    QtDialogs.ColorDialog {
        id: colorDialog
        title: root.td("Choose an accent color")
        onAccepted: kcm.setAccent(String(selectedColor).substring(0, 7).toUpperCase())
    }
}
