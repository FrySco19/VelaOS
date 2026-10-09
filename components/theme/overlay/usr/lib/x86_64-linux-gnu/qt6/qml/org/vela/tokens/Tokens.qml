// Vela design tokens for QML apps. Colors come from the active KDE color
// scheme (VelaChiaro / VelaScuro) through the system palette, so light/dark and
// the user's accent are always respected. Sizes mirror /usr/share/vela/theme/tokens.json.
pragma Singleton
import QtQuick

QtObject {
    readonly property SystemPalette palette: SystemPalette { colorGroup: SystemPalette.Active }
    readonly property bool dark: palette.window.hslLightness < 0.5

    // radii
    readonly property int radiusWindow: 12
    readonly property int radiusCard: 12
    readonly property int radiusPopup: 12
    readonly property int radiusPanel: 16
    readonly property int radiusControl: 8
    readonly property int radiusSmall: 6
    // spacing (4 px grid)
    readonly property int xs: 4
    readonly property int s: 8
    readonly property int m: 12
    readonly property int l: 16
    readonly property int xl: 24
    readonly property int xxl: 32
    // type
    readonly property string fontUi: "Inter"
    readonly property string fontMono: "JetBrains Mono"
    // motion
    readonly property int fastMs: 120
    readonly property int normalMs: 200
    readonly property int slowMs: 280

    // semantic colors (palette-driven where possible)
    readonly property color window: palette.window
    readonly property color view: palette.base
    readonly property color text: palette.windowText
    readonly property color textSecondary: palette.placeholderText
    readonly property color accent: palette.highlight
    readonly property color accentText: palette.highlightedText
    readonly property color separator: dark ? "#2A3E48" : "#D5E0E3"
    readonly property color glass: dark ? Qt.rgba(20/255, 36/255, 44/255, 0.70) : Qt.rgba(246/255, 250/255, 251/255, 0.72)
    readonly property color positive: dark ? "#5CBF8A" : "#2E8B57"
    readonly property color neutral: dark ? "#E0B25C" : "#B7791F"
    readonly property color negative: dark ? "#E66A6A" : "#C93A3A"
    // brand (fixed)
    readonly property color coral: "#E68B75"
    readonly property color marine: "#1C5468"
    readonly property color marineDeep: "#0B2A3A"
}
