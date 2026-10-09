import QtQuick
Text {
    color: theme.c.muted
    font.family: theme.font; font.weight: Font.DemiBold
    font.pixelSize: Math.round(12 * theme.scale)
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 0.6
    topPadding: 6
    Accessible.role: Accessible.Heading
    Accessible.name: text
}
