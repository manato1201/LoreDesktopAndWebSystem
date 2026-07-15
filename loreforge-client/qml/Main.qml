import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import LoreForge

ApplicationWindow {
    id: window
    width: 1200
    height: 800
    visible: true
    title: "LoreForge Client"
    color: Theme.colorBackgroundBase

    RepositoryListModel {
        id: repositoryModel
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 3
        spacing: Theme.spacingUnit * 3

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "LoreForge"
                color: Theme.colorAccent
                font.bold: true
                font.pixelSize: Theme.fontSizeSectionTitle
            }

            Item { Layout.fillWidth: true }

            Text {
                text: repositoryModel.count + " repositories"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }
        }

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: 320
            cellHeight: 168
            model: repositoryModel

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
                }
            }
        }
    }
}
