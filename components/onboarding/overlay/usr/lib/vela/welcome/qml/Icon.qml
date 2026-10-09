import QtQuick
import QtQuick.Shapes

// Line icons drawn as vector paths (24x24 grid) so they follow theme colours.
Item {
    id: root
    property string name
    property color color: theme.c.text
    property real size: 20
    property real stroke: 1.8
    width: size; height: size
    Accessible.ignored: true

    readonly property var paths: ({
        "check": "M5 12.5l4.5 4.5L19 7.5",
        "chevronRight": "M9 6l6 6-6 6",
        "chevronLeft": "M15 6l-6 6 6 6",
        "globe": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M3 12h18 M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9s1.3-6.4 3.8-9z",
        "wifi": "M2.5 9a14 14 0 0 1 19 0 M5.5 12.5a9.5 9.5 0 0 1 13 0 M8.8 16a4.8 4.8 0 0 1 6.4 0 M12 19.5h.01",
        "wifi2": "M5.5 12.5a9.5 9.5 0 0 1 13 0 M8.8 16a4.8 4.8 0 0 1 6.4 0 M12 19.5h.01",
        "wifi1": "M8.8 16a4.8 4.8 0 0 1 6.4 0 M12 19.5h.01",
        "lock": "M6 11h12v9H6z M8.5 11V8a3.5 3.5 0 0 1 7 0v3",
        "cable": "M4 8h16v9H4z M8 17v2.5 M12 17v2.5 M16 17v2.5 M8.5 8V5h7v3",
        "sun": "M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8z M12 2.5v2 M12 19.5v2 M5.3 5.3l1.4 1.4 M17.3 17.3l1.4 1.4 M2.5 12h2 M19.5 12h2 M5.3 18.7l1.4-1.4 M17.3 6.7l1.4-1.4",
        "moon": "M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z",
        "auto": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M12 3v18 M12 7.5h4 M12 12h5.5 M12 16.5h4",
        "a11y": "M12 3.2a1.8 1.8 0 1 0 0 3.6a1.8 1.8 0 1 0 0-3.6z M5 9l7 1.5L19 9 M12 10.5V15 M12 15l-3 6 M12 15l3 6",
        "keyboard": "M3 6h18v12H3z M7 10h.01 M10.3 10h.01 M13.6 10h.01 M17 10h.01 M7.5 14h9",
        "dock": "M3 5h18v14H3z M7 15.5h10",
        "sparkle": "M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z M18.5 15.5l.8 2.2 2.2.8-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.8z",
        "shield": "M12 3l8 3v6c0 4.5-3.4 8.2-8 9-4.6-.8-8-4.5-8-9V6z M8.8 12.2l2.2 2.2 4.2-4.4",
        "done": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M8 12.3l2.8 2.8L16.5 9.5",
        "sail": "M12 3v15 M12 4.5c3.8 3 6 7 6.5 11.5H12 M10.5 7.5c-2.4 2.5-3.8 5.3-4.2 8.5h4.2 M4 19.5c2.5 1.2 5 1.5 8 1.5s5.5-.3 8-1.5",
        "eye": "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6z",
        "mic": "M9 5a3 3 0 0 1 6 0v6a3 3 0 0 1-6 0z M5.5 11a6.5 6.5 0 0 0 13 0 M12 17.5V21",
        "monitor": "M3 4h18v12H3z M8 20h8 M12 16v4",
        "clipboard": "M8.5 3.5h7v3h-7z M6.5 5H5v15.5h14V5h-1.5 M8.5 11h7 M8.5 15h5",
        "text": "M5 6h14 M12 6v12 M9 18h6",
        "window": "M3 5h18v14H3z M3 9h18",
        "memory": "M12 3c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3z M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6 M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6",
        "list": "M9 6h11 M9 12h11 M9 18h11 M4.5 6h.01 M4.5 12h.01 M4.5 18h.01",
        "search": "M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14z M16 16l4.5 4.5",
        "play": "M8 5.5v13l10-6.5z",
        "pause": "M9 5.5v13 M15 5.5v13",
        "download": "M12 4v11 M7.5 10.5L12 15l4.5-4.5 M5 20h14",
        "cloud": "M7 18.5h10a4 4 0 0 0 .5-8 6 6 0 0 0-11.5 1.5 3.3 3.3 0 0 0 1 6.5z",
        "cpu": "M7 7h10v10H7z M10 3v4 M14 3v4 M10 17v4 M14 17v4 M3 10h4 M3 14h4 M17 10h4 M17 14h4",
        "off": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M5.6 5.6l12.8 12.8",
        "help": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 1-1 1.7v.5 M12 17h.01",
        "refresh": "M20 11a8 8 0 1 0-2.3 5.7 M20 4.5V11h-6.5",
        "close": "M6 6l12 12 M18 6L6 18",
        "info": "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z M12 11v5.5 M12 7.8h.01",
        "warn": "M12 3.5l9.5 16.5h-19z M12 10v4.5 M12 17.5h.01",
        "grid": "M4 4h6v6H4z M14 4h6v6h-6z M4 14h6v6H4z M14 14h6v6h-6z",
        "spaces": "M2.5 6h8.5v12H2.5z M13 6h8.5v12H13z",
        "tile": "M3 5h18v14H3z M12 5v14",
        "camera": "M4 7.5h3.5L9 5h6l1.5 2.5H20V19H4z M12 10a3.2 3.2 0 1 0 0 6.4a3.2 3.2 0 1 0 0-6.4z"
    })

    Shape {
        anchors.centerIn: parent
        width: 24; height: 24
        scale: root.size / 24
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: root.color
            strokeWidth: root.stroke
            fillColor: root.name === "play" ? root.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
