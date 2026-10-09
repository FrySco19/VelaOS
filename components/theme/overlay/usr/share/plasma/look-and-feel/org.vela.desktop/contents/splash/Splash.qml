/*
    Vela splash screen. SPDX-License-Identifier: GPL-2.0-or-later
*/
import QtQuick
import org.kde.kirigami 2 as Kirigami

Rectangle {
    id: root
    property int stage
    readonly property bool motion: Kirigami.Units.longDuration > 1
    gradient: Gradient {
        GradientStop { position: 0.0; color: "#123B4D" }
        GradientStop { position: 1.0; color: "#06161F" }
    }

    onStageChanged: {
        if (stage == 2) {
            intro.running = true
        }
    }

    Item {
        id: content
        anchors.fill: parent
        opacity: root.motion ? 0 : 1

        Image {
            id: logo
            readonly property real size: Kirigami.Units.gridUnit * 6
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -Kirigami.Units.gridUnit
            source: "images/vela.svg"
            sourceSize.width: size
            sourceSize.height: size
            asynchronous: true
            scale: root.motion ? 0.94 : 1
            NumberAnimation on scale { id: grow; running: false; to: 1; duration: 600; easing.type: Easing.OutCubic }
        }
        Text {
            id: word
            anchors.top: logo.bottom
            anchors.topMargin: Kirigami.Units.largeSpacing
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Vela"
            color: "#EEF6F6"
            font.family: "Inter"
            font.pointSize: 20
            font.weight: Font.Light
            font.letterSpacing: 1
        }
        Rectangle {
            id: track
            width: Kirigami.Units.gridUnit * 8
            height: 3
            radius: 1.5
            color: "#33EEF6F6"
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: word.bottom
            anchors.topMargin: Kirigami.Units.gridUnit * 2
            Rectangle {
                height: parent.height
                radius: parent.radius
                color: "#E68B75"
                width: parent.width * Math.min(1, Math.max(0.08, root.stage / 6))
                Behavior on width { enabled: root.motion; NumberAnimation { duration: 400; easing.type: Easing.OutCubic } }
            }
        }
    }

    ParallelAnimation {
        id: intro
        running: false
        OpacityAnimator { target: content; from: content.opacity; to: 1; duration: root.motion ? 500 : 0; easing.type: Easing.OutCubic }
        ScriptAction { script: if (root.motion) grow.running = true }
    }
}
