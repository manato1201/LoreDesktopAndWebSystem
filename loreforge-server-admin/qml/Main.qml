import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import LoreForgeServerAdmin

ApplicationWindow {
    id: window
    width: 1200
    height: 800
    visible: true
    title: "LoreForge Server Admin"
    color: Theme.colorBackgroundBase

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 3
        spacing: Theme.spacingUnit * 3

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "LoreForge Server Admin"
                color: Theme.colorAccent
                font.bold: true
                font.pixelSize: Theme.fontSizeSectionTitle
            }

            Item { Layout.fillWidth: true }

            Text {
                text: "Local environment control"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }
        }

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            background: Rectangle { color: "transparent" }

            TabButton {
                text: "Environment"
                font.pixelSize: Theme.fontSizeButton
            }
            TabButton {
                text: "Access Control"
                font.pixelSize: Theme.fontSizeButton
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            EnvironmentPanel {}
            AccessControlPanel {}
        }
    }
}
