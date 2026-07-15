import QtQuick
import LoreForge

Rectangle {
    id: button

    property string text: ""
    property bool busy: false

    signal clicked()

    implicitHeight: 40
    implicitWidth: label.implicitWidth + Theme.spacingUnit * 4
    radius: Theme.radiusStandard
    color: !button.enabled ? Theme.colorSurfaceInteractive
        : mouseArea.pressed ? Theme.colorAccentBorder : Theme.colorAccent
    opacity: button.enabled ? 1.0 : 0.6

    Text {
        id: label
        anchors.centerIn: parent
        text: button.busy ? "..." : button.text
        color: Theme.colorBackgroundBase
        font.pixelSize: Theme.fontSizeButton
        font.bold: true
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        enabled: button.enabled && !button.busy
        onClicked: button.clicked()
    }
}
