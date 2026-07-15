pragma Singleton
import QtQuick

// Mirrors ../../DESIGN.md §10 Cross-Platform Token Mapping. Keep values in
// sync with that table and with lorehub-web's globals.css — it is the
// shared source of truth across the ecosystem.
QtObject {
    readonly property color colorBackgroundBase: "#121212"
    readonly property color colorSurface: "#181818"
    readonly property color colorSurfaceInteractive: "#1f1f1f"
    readonly property color colorSurfaceElevated: "#252525"

    readonly property color colorAccent: "#1ed760"
    readonly property color colorAccentBorder: "#1db954"

    readonly property color colorTextPrimary: "#ffffff"
    readonly property color colorTextSecondary: "#b3b3b3"

    readonly property color colorBorder: "#4d4d4d"
    readonly property color colorBorderLight: "#7c7c7c"

    readonly property color colorNegative: "#f3727f"
    readonly property color colorWarning: "#ffa42b"
    readonly property color colorAnnouncement: "#539df5"

    readonly property int radiusSubtle: 4
    readonly property int radiusStandard: 6
    readonly property int radiusComfortable: 8

    readonly property int spacingUnit: 8

    readonly property int fontSizeSectionTitle: 24
    readonly property int fontSizeHeading: 18
    readonly property int fontSizeBody: 16
    readonly property int fontSizeButton: 14
    readonly property int fontSizeCaption: 14
    readonly property int fontSizeSmall: 12
}
