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

    property string activeTab: "environment" // "environment" | "access-control"

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

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit

            Rectangle {
                implicitWidth: environmentTabLabel.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: height / 2
                color: window.activeTab === "environment" ? Theme.colorSurfaceElevated : "transparent"
                border.width: window.activeTab === "environment" ? 0 : 1
                border.color: Theme.colorBorder

                Text {
                    id: environmentTabLabel
                    anchors.centerIn: parent
                    text: "Environment"
                    color: window.activeTab === "environment" ? Theme.colorTextPrimary : Theme.colorTextSecondary
                    font.bold: window.activeTab === "environment"
                    font.pixelSize: Theme.fontSizeCaption
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: window.activeTab = "environment"
                }
            }

            Rectangle {
                implicitWidth: accessControlTabLabel.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: height / 2
                color: window.activeTab === "access-control" ? Theme.colorSurfaceElevated : "transparent"
                border.width: window.activeTab === "access-control" ? 0 : 1
                border.color: Theme.colorBorder

                Text {
                    id: accessControlTabLabel
                    anchors.centerIn: parent
                    text: "Access Control"
                    color: window.activeTab === "access-control" ? Theme.colorTextPrimary : Theme.colorTextSecondary
                    font.bold: window.activeTab === "access-control"
                    font.pixelSize: Theme.fontSizeCaption
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: window.activeTab = "access-control"
                }
            }

            Item { Layout.fillWidth: true }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: window.activeTab === "environment" ? 0 : 1

            EnvironmentPanel {}
            AccessControlPanel {}
        }
    }
}
