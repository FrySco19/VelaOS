import QtQuick
import QtQuick.Controls.Basic

PageBase {
    iconName: "dock"
    title: backend.s["dock.title"]
    body: backend.s["dock.body"]

    Row {
        spacing: 24
        width: parent.width
        Column {
            spacing: 10
            SectionLabel { text: backend.s["dock.position"] }
            Segmented {
                accessibleName: backend.s["dock.position"]
                model: [{value: "left", label: backend.s["dock.left"]}, {value: "bottom", label: backend.s["dock.bottom"]},
                        {value: "right", label: backend.s["dock.right"]}]
                current: backend.dockPosition
                onPicked: v => backend.dockPosition = v
            }
        }
        MiniDesktop { width: 170; height: 106; dark: theme.dark; dock: backend.dockPosition; autohide: backend.dockAutohide }
    }
    Panel {
        VSwitch { width: parent.width; iconName: "dock"; title: backend.s["dock.autohide"]; hint: backend.s["dock.autohideHint"]
                  checked: backend.dockAutohide; onToggled: backend.dockAutohide = checked }
        VSwitch { width: parent.width; iconName: "grid"; title: backend.s["desk.icons"]; hint: backend.s["desk.iconsHint"]
                  checked: backend.desktopIcons; onToggled: backend.desktopIcons = checked }
        VSwitch { width: parent.width; iconName: "download"; title: backend.s["dock.newApps"]; hint: backend.s["dock.newAppsHint"]
                  checked: backend.dockNewApps; onToggled: backend.dockNewApps = checked }
    }
    SectionLabel { text: backend.s["desk.click"] }
    Segmented {
        accessibleName: backend.s["desk.click"]
        model: [{value: "double", label: backend.s["desk.double"]}, {value: "single", label: backend.s["desk.single"]}]
        current: backend.singleClick ? "single" : "double"
        onPicked: v => backend.singleClick = (v === "single")
    }
}
