import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "a11y"
    title: backend.s["a11y.title"]
    body: backend.s["a11y.body"]

    Panel {
        VSwitch { width: parent.width; iconName: "text"; title: backend.s["a11y.large"]; hint: backend.s["a11y.largeHint"]
                  checked: backend.a11y("large_text"); onToggled: backend.setA11y("large_text", checked) }
        VSwitch { width: parent.width; iconName: "eye"; title: backend.s["a11y.contrast"]; hint: backend.s["a11y.contrastHint"]
                  checked: backend.a11y("high_contrast"); onToggled: backend.setA11y("high_contrast", checked) }
        VSwitch { width: parent.width; iconName: "window"; title: backend.s["a11y.motion"]; hint: backend.s["a11y.motionHint"]
                  checked: backend.a11y("reduce_motion"); onToggled: backend.setA11y("reduce_motion", checked) }
        VSwitch { width: parent.width; iconName: "mic"; title: backend.s["a11y.reader"]
                  hint: backend.orcaAvailable ? backend.s["a11y.readerHint"] : backend.s["a11y.readerMissing"]
                  enabled: backend.orcaAvailable
                  checked: backend.a11y("screen_reader"); onToggled: backend.setA11y("screen_reader", checked) }
    }
}
