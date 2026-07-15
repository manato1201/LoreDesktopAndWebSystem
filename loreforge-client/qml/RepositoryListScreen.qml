import QtQuick
import QtQuick.Layouts
import LoreForge

Item {
    id: screen

    property var repositoryModel
    property var authController

    signal repositorySelected(string slug, string name)

    Component.onCompleted: if (repositoryModel) repositoryModel.refresh()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 3
        spacing: Theme.spacingUnit * 3

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit * 3

            Text {
                text: "LoreForge"
                color: Theme.colorAccent
                font.bold: true
                font.pixelSize: Theme.fontSizeSectionTitle
            }

            Item { Layout.fillWidth: true }

            Text {
                text: screen.authController ? screen.authController.currentUserName : ""
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }

            Text {
                text: screen.repositoryModel ? screen.repositoryModel.count + " repositories" : ""
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }

            Text {
                text: "Log out"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: if (screen.authController) screen.authController.logout()
                }
            }
        }

        Text {
            visible: screen.repositoryModel && screen.repositoryModel.errorMessage.length > 0
            text: screen.repositoryModel ? screen.repositoryModel.errorMessage : ""
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeCaption
        }

        Text {
            visible: screen.repositoryModel && screen.repositoryModel.busy
            text: "Loading repositories…"
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeCaption
        }

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: 320
            cellHeight: 168
            model: screen.repositoryModel

            delegate: Item {
                width: grid.cellWidth
                height: grid.cellHeight

                RepositoryCard {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit
                    repoName: model.name
                    repoDescription: model.description
                    updatedAt: model.updatedAt
                    sizeLabel: model.sizeLabel
                    lockedFileCount: model.lockedFileCount
                    visibilityLabel: model.visibility

                    onClicked: screen.repositorySelected(model.slug, model.name)
                }
            }
        }
    }
}
