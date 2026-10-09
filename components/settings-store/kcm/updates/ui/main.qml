import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: root
    function td(s) { return i18nd("kcm_vela_updates", s) }

    header: ColumnLayout {
        spacing: 0
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: kcm.error.length > 0
            type: Kirigami.MessageType.Error
            text: kcm.error
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: kcm.liveSession
            type: Kirigami.MessageType.Information
            text: root.td("You are trying Vela without installing it: nothing is updated automatically in this session.")
        }
    }

    Kirigami.FormLayout {
        enabled: !kcm.busy

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Security") }

        QQC2.Switch {
            Kirigami.FormData.label: root.td("Security updates:")
            text: root.td("Install automatically")
            checked: kcm.securityAuto
            onToggled: { kcm.setSecurityAuto(checked); checked = Qt.binding(function() { return kcm.securityAuto }) }
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            text: kcm.lastCheck.length
                  ? i18nd("kcm_vela_updates", "Only security fixes from Debian, in the background. The computer never restarts on its own. Last check: %1.", kcm.lastCheck)
                  : root.td("Only security fixes from Debian, in the background. The computer never restarts on its own.")
        }

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Snapshots") }

        QQC2.Switch {
            Kirigami.FormData.label: root.td("Before each update:")
            visible: kcm.snapperAvailable
            text: root.td("Take a system snapshot")
            checked: kcm.snapshotsBeforeUpdate
            onToggled: { kcm.setSnapshotsBeforeUpdate(checked); checked = Qt.binding(function() { return kcm.snapshotsBeforeUpdate }) }
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            font: kcm.snapperAvailable ? Kirigami.Theme.smallFont : Kirigami.Theme.defaultFont
            opacity: kcm.snapperAvailable ? 0.7 : 1
            text: kcm.snapperAvailable
                  ? root.td("If an update causes problems, you can go back to the snapshot. Your personal files are not affected.")
                  : root.td("Snapshots are available when Vela is installed on a disk with the Btrfs file system.")
        }
        ColumnLayout {
            Kirigami.FormData.label: root.td("Latest snapshots:")
            Kirigami.FormData.labelAlignment: Qt.AlignTop
            visible: kcm.snapperAvailable && kcm.snapshots.length > 0
            spacing: Kirigami.Units.smallSpacing
            Repeater {
                model: kcm.snapshots
                delegate: RowLayout {
                    required property var modelData
                    Kirigami.Icon { source: "vela-snapshots"; fallback: "camera-photo"; implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth }
                    QQC2.Label { textFormat: Text.PlainText; text: modelData.when }
                    QQC2.Label { textFormat: Text.PlainText; opacity: 0.7; text: modelData.description }
                }
            }
        }
        QQC2.Button {
            visible: kcm.canOpenSnapshots()
            icon.name: "vela-snapshots"
            text: root.td("System snapshots…")
            onClicked: kcm.openSnapshots()
        }

        Kirigami.Separator { Kirigami.FormData.isSection: true; Kirigami.FormData.label: root.td("Apps and other updates") }

        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            wrapMode: Text.Wrap
            text: root.td("The Store tells you when other updates are ready, at most once a week. They are installed when you restart.")
        }
        RowLayout {
            QQC2.Button {
                icon.name: "system-software-update"
                text: root.td("Check for updates")
                onClicked: kcm.openStoreUpdates()
            }
            QQC2.Button {
                icon.name: "configure"
                text: root.td("More options…")
                onClicked: KCM.KCMLauncher.openSystemSettings("kcm_updates")
            }
        }
    }
}
