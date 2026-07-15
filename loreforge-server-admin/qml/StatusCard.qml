import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import LoreForgeServerAdmin

Rectangle {
    id: card

    property string title: ""
    property string statusText: ""
    property color statusColor: Theme.colorTextSecondary
    property string caption: ""
    property bool captionItalic: false
    property bool busy: false

    property string primaryActionLabel: ""
    property bool primaryActionEnabled: true
    signal primaryActionTriggered()

    property string secondaryActionLabel: ""
    property bool secondaryActionEnabled: true
    signal secondaryActionTriggered()

    color: Theme.colorSurface
    radius: Theme.radiusComfortable
    border.color: Theme.colorBorder
    border.width: 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 2
        spacing: Theme.spacingUnit

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit

            Text {
                text: card.title
                color: Theme.colorTextPrimary
                font.bold: true
                font.pixelSize: Theme.fontSizeHeading
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                width: 10
                height: 10
                radius: 5
                color: card.statusColor
                visible: card.statusText.length > 0
            }

            Text {
                text: card.busy ? "Working…" : card.statusText
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }
        }

        Item { Layout.fillHeight: true }

        Text {
            Layout.fillWidth: true
            text: card.caption
            visible: card.caption.length > 0
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeSmall
            font.italic: card.captionItalic
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit

            Button {
                text: card.primaryActionLabel
                visible: card.primaryActionLabel.length > 0
                enabled: card.primaryActionEnabled && !card.busy
                onClicked: card.primaryActionTriggered()

                background: Rectangle {
                    radius: Theme.radiusStandard
                    color: parent.enabled
                           ? (parent.hovered ? Theme.colorAccentBorder : Theme.colorAccent)
                           : Theme.colorSurfaceInteractive
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#121212" : Theme.colorTextSecondary
                    font.bold: true
                    font.pixelSize: Theme.fontSizeButton
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Button {
                text: card.secondaryActionLabel
                visible: card.secondaryActionLabel.length > 0
                enabled: card.secondaryActionEnabled && !card.busy
                onClicked: card.secondaryActionTriggered()

                background: Rectangle {
                    radius: Theme.radiusStandard
                    color: "transparent"
                    border.color: parent.enabled ? Theme.colorBorderLight : Theme.colorBorder
                    border.width: 1
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? Theme.colorTextPrimary : Theme.colorTextSecondary
                    font.bold: true
                    font.pixelSize: Theme.fontSizeButton
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Item { Layout.fillWidth: true }
        }
    }
}
