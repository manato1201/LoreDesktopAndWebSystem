import QtQuick
import QtQuick.Layouts
import LoreForge

Rectangle {
    id: card

    property string repoName
    property string repoDescription
    property string updatedAt
    property string sizeLabel
    property int lockedFileCount
    property string visibilityLabel

    property bool hovered: false

    signal clicked()

    color: hovered ? Theme.colorSurfaceElevated : Theme.colorSurface
    radius: Theme.radiusComfortable

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: card.hovered = true
        onExited: card.hovered = false
        onClicked: card.clicked()
    }

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 2
        spacing: Theme.spacingUnit

        RowLayout {
            width: parent.width
            spacing: Theme.spacingUnit

            Text {
                text: card.repoName
                color: Theme.colorTextPrimary
                font.bold: true
                font.pixelSize: Theme.fontSizeBody
            }

            Item { Layout.fillWidth: true }

            Text {
                text: card.visibilityLabel.toUpperCase()
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeSmall
                font.letterSpacing: 1
            }
        }

        Text {
            text: card.repoDescription
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeCaption
            wrapMode: Text.WordWrap
            width: parent.width
            maximumLineCount: 2
            elide: Text.ElideRight
        }

        Row {
            width: parent.width
            spacing: Theme.spacingUnit * 2

            Text {
                text: "Updated " + card.updatedAt
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeSmall
            }

            Text {
                text: card.sizeLabel
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeSmall
            }

            Row {
                visible: card.lockedFileCount > 0
                spacing: 4

                Rectangle {
                    width: 8
                    height: 8
                    radius: 4
                    color: Theme.colorWarning
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    text: card.lockedFileCount + " locked"
                    color: Theme.colorWarning
                    font.pixelSize: Theme.fontSizeSmall
                }
            }
        }
    }
}
