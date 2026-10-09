import QtQuick
import org.vela.tokens

Row {
    spacing: 5
    Repeater {
        model: 3
        Rectangle {
            width: 7; height: 7; radius: 3.5; color: Tokens.textSecondary
            opacity: 0.3
            SequentialAnimation on opacity {
                running: bridge.animations; loops: Animation.Infinite
                PauseAnimation { duration: index * 160 }
                NumberAnimation { to: 1; duration: 320; easing.type: Easing.OutCubic }
                NumberAnimation { to: 0.3; duration: 320; easing.type: Easing.InCubic }
                PauseAnimation { duration: (2 - index) * 160 }
            }
        }
    }
}
