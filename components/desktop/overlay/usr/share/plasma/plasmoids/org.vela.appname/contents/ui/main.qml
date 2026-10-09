/*
    Vela: bold name of the active app in the top bar (like macOS); "Scrivania" when the
    desktop is focused. Click = menu with all open windows.
    Based on org.kde.plasma.windowlist (SPDX: 2016 Eike Hein, 2022 Carson Black; GPL-2.0-or-later).
*/
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.kde.ksvg as KSvg
import org.kde.taskmanager as TaskManager

PlasmoidItem {
    id: root
    Plasmoid.constraintHints: Plasmoid.CanFillArea
    preferredRepresentation: fullRepresentation
    fullRepresentation: button
    compactRepresentation: button

    readonly property bool italian: Qt.locale().name.startsWith("it")
    readonly property string desktopName: italian ? "Scrivania" : "Desktop"

    TaskManager.TasksModel {
        id: tasksModel
        sortMode: TaskManager.TasksModel.SortVirtualDesktop
        groupMode: TaskManager.TasksModel.GroupDisabled
    }

    function activeName() {
        const t = tasksModel.activeTask;
        if (!t.valid) {
            return desktopName;
        }
        const appId = tasksModel.data(t, TaskManager.AbstractTasksModel.AppId) || "";
        if (appId === "org.kde.plasmashell" || appId === "plasmashell") {
            return desktopName;
        }
        return tasksModel.data(t, TaskManager.AbstractTasksModel.AppName)
            || tasksModel.data(t, 0 /* window title */) || desktopName;
    }

    Component {
        id: button

        QQC2.AbstractButton {
            id: control
            hoverEnabled: true
            Layout.minimumWidth: implicitWidth
            Layout.maximumWidth: Kirigami.Units.gridUnit * 14
            Layout.fillHeight: true
            leftPadding: Kirigami.Units.smallSpacing * 2
            rightPadding: Kirigami.Units.smallSpacing * 2
            text: root.activeName()
            Accessible.name: text
            Accessible.description: root.italian ? "Mostra le finestre aperte" : "Show open windows"
            down: pressed || tasksMenu.status === PlasmaExtras.Menu.Open
            onClicked: tasksMenu.open()

            Connections {
                target: tasksModel
                function onActiveTaskChanged() { control.text = Qt.binding(() => root.activeName()) }
                function onDataChanged() { control.text = Qt.binding(() => root.activeName()) }
            }

            background: KSvg.FrameSvgItem {
                imagePath: "widgets/menubaritem"
                prefix: control.down ? "pressed" : "hover"
                visible: control.down || control.hovered
            }
            contentItem: PlasmaComponents.Label {
                text: control.text
                font.weight: Font.Bold
                elide: Text.ElideRight
                textFormat: Text.PlainText
                verticalAlignment: Text.AlignVCenter
                color: control.down || control.hovered ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
            }

            PlasmaExtras.ModelContextMenu {
                id: tasksMenu
                visualParent: control
                placement: PlasmaExtras.Menu.BottomPosedLeftAlignedPopup
                model: tasksModel
                onClicked: (model) => tasksModel.requestActivate(tasksModel.makeModelIndex(model.index))
            }
        }
    }
}
