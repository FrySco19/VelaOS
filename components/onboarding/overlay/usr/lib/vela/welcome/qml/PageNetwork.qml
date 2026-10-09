import QtQuick
import QtQuick.Controls.Basic

PageBase {
    id: p
    iconName: "wifi"
    title: backend.s["net.title"]
    body: backend.s["net.body"]
    property string pending: ""
    Component.onCompleted: backend.scanWifi()

    StatusLine {
        width: parent.width
        kind: backend.netSummary === "offline" ? "info" : "ok"
        text: backend.netSummary === "wired" ? backend.s["net.wired"]
            : backend.netSummary === "online" ? backend.s["net.online"]
            : backend.netSummary === "offline" ? backend.s["net.offline"] : ""
    }
    StatusLine {
        width: parent.width
        kind: backend.wifiState === "scanning" || backend.wifiState === "connecting" ? "busy" : (backend.wifiState === "failed" ? "error" : "info")
        text: backend.wifiState === "scanning" ? backend.s["net.scanning"]
            : backend.wifiState === "connecting" ? backend.s["net.connecting"]
            : backend.wifiState === "failed" ? backend.s["net.failed"].replace("%1", backend.wifiMessage)
            : backend.wifiState === "nowifi" ? backend.s["net.noWifi"]
            : (backend.wifiState === "list" && backend.wifi.length === 0) ? backend.s["net.noneFound"] : ""
    }

    Panel {
        visible: backend.wifi.length > 0
        Repeater {
            model: backend.wifi
            Column {
                id: net
                required property var modelData
                width: parent.width
                AbstractButton {
                    id: nb
                    width: parent.width; height: Math.round(46 * theme.scale)
                    focusPolicy: Qt.StrongFocus; hoverEnabled: true
                    Accessible.role: Accessible.Button
                    Accessible.name: net.modelData.ssid + ", " + (net.modelData.secure ? backend.s["net.secured"] : backend.s["net.openNet"])
                    Keys.onReturnPressed: clicked()
                    onClicked: {
                        if (net.modelData.active) return
                        if (net.modelData.secure) { p.pending = net.modelData.ssid; pw.text = ""; pw.forceActiveFocus() }
                        else backend.connectWifi(net.modelData.ssid, "")
                    }
                    background: Rectangle { radius: theme.radius - 4; color: nb.hovered ? theme.c.surface2 : "transparent"
                                            border.width: nb.visualFocus ? 2 : 0; border.color: theme.c.accent }
                    contentItem: Item {
                        Icon { id: wi; x: 12; anchors.verticalCenter: parent.verticalCenter; size: 20
                               name: net.modelData.signal > 66 ? "wifi" : (net.modelData.signal > 33 ? "wifi2" : "wifi1")
                               color: theme.c.text }
                        Text { anchors.left: wi.right; anchors.leftMargin: 12; anchors.verticalCenter: parent.verticalCenter
                               text: net.modelData.ssid; color: theme.c.text; font.family: theme.font
                               font.pixelSize: Math.round(14 * theme.scale); font.weight: net.modelData.active ? Font.DemiBold : Font.Normal }
                        Row {
                            anchors.right: parent.right; anchors.rightMargin: 12; anchors.verticalCenter: parent.verticalCenter
                            spacing: 10
                            Icon { visible: net.modelData.secure; name: "lock"; size: 16; color: theme.c.muted }
                            Icon { visible: net.modelData.active; name: "check"; size: 20; color: theme.c.accent }
                        }
                    }
                }
                Row {
                    visible: p.pending === net.modelData.ssid
                    spacing: 8; leftPadding: 44; bottomPadding: 10
                    VField { id: pw; width: 240; echoMode: TextInput.Password
                             label: backend.s["net.passwordFor"].replace("%1", net.modelData.ssid)
                             placeholderText: backend.s["net.password"]
                             onAccepted: cbtn.clicked() }
                    VButton { id: cbtn; kind: "primary"; text: backend.s["net.connect"]; enabled: pw.text.length > 0
                              onClicked: { backend.connectWifi(p.pending, pw.text); p.pending = "" } }
                    VButton { text: backend.s["net.cancel"]; onClicked: p.pending = "" }
                }
            }
        }
    }
    Row {
        spacing: 8
        VButton { text: backend.s["net.refresh"]; iconName: "refresh"; visible: backend.wifiState !== "nowifi"
                  enabled: backend.wifiState !== "scanning"; onClicked: backend.scanWifi() }
        VButton { kind: "plain"; text: backend.s["net.more"]; onClicked: backend.openNetworkSettings() }
    }
}
