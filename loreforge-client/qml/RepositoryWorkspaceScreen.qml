import QtQuick
import QtQuick.Layouts
import LoreForge

Item {
    id: workspace

    property var treeModel
    property var authController
    property string slug: ""
    property string repoName: ""
    property string selectedPath: ""

    // Re-evaluated whenever treeModel.count changes (load, expand/collapse,
    // lock round-trip) since rowForPath() reads C++ state that isn't itself
    // a bindable property.
    property var selectedFile: {
        if (workspace.treeModel)
            workspace.treeModel.count
        return workspace.treeModel && workspace.selectedPath.length > 0
            ? workspace.treeModel.rowForPath(workspace.selectedPath)
            : ({})
    }

    signal backRequested()

    Component.onCompleted: if (treeModel && slug.length > 0) treeModel.loadRepository(slug)

    function kindLabel(kind) {
        switch (kind) {
        case "directory": return "DIR"
        case "text": return "TXT"
        case "image": return "IMG"
        case "model3d": return "3D"
        case "audio": return "AUD"
        case "binary": return "BIN"
        default: return kind.toUpperCase()
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.colorBackgroundBase
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 3
        spacing: Theme.spacingUnit * 2

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit * 2

            Text {
                text: "‹ Repositories"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: workspace.backRequested()
                }
            }

            Text {
                text: workspace.repoName
                color: Theme.colorTextPrimary
                font.bold: true
                font.pixelSize: Theme.fontSizeHeading
            }

            Item { Layout.fillWidth: true }

            Text {
                visible: workspace.treeModel && workspace.treeModel.busy
                text: "Working…"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }
        }

        Text {
            visible: workspace.treeModel && workspace.treeModel.errorMessage.length > 0
            text: workspace.treeModel ? workspace.treeModel.errorMessage : ""
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeCaption
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacingUnit * 2

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: Theme.colorSurface
                radius: Theme.radiusComfortable

                ListView {
                    id: treeList
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit
                    clip: true
                    model: workspace.treeModel

                    delegate: Rectangle {
                        width: treeList.width
                        height: 36
                        radius: Theme.radiusSubtle
                        color: model.path === workspace.selectedPath ? Theme.colorSurfaceElevated
                            : rowMouse.containsMouse ? Theme.colorSurfaceInteractive : "transparent"

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacingUnit + model.depth * (Theme.spacingUnit * 2)
                            anchors.rightMargin: Theme.spacingUnit
                            spacing: Theme.spacingUnit

                            Text {
                                text: model.isDirectory ? (model.expanded ? "▾" : "▸") : ""
                                color: Theme.colorTextSecondary
                                font.pixelSize: Theme.fontSizeSmall
                                Layout.preferredWidth: 14
                            }

                            Rectangle {
                                width: 32
                                height: 16
                                radius: Theme.radiusSubtle
                                color: Theme.colorSurfaceInteractive

                                Text {
                                    anchors.centerIn: parent
                                    text: workspace.kindLabel(model.kind)
                                    color: Theme.colorTextSecondary
                                    font.pixelSize: 9
                                    font.bold: true
                                }
                            }

                            Text {
                                text: model.name
                                color: Theme.colorTextPrimary
                                font.pixelSize: Theme.fontSizeCaption
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Rectangle {
                                visible: model.lockedBy !== ""
                                width: 8
                                height: 8
                                radius: 4
                                color: Theme.colorWarning
                            }

                            Text {
                                visible: model.lockedBy !== ""
                                text: "locked by " + model.lockedBy
                                color: Theme.colorWarning
                                font.pixelSize: Theme.fontSizeSmall
                            }
                        }

                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (model.isDirectory)
                                    workspace.treeModel.toggleExpanded(model.path)
                                else
                                    workspace.selectedPath = model.path
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 320
                Layout.fillHeight: true
                color: Theme.colorSurface
                radius: Theme.radiusComfortable
                visible: workspace.selectedPath.length > 0

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit * 2
                    spacing: Theme.spacingUnit

                    Text {
                        text: workspace.selectedFile.name || ""
                        color: Theme.colorTextPrimary
                        font.bold: true
                        font.pixelSize: Theme.fontSizeHeading
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    Text {
                        text: workspace.selectedFile.path || ""
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeSmall
                        Layout.fillWidth: true
                        wrapMode: Text.WrapAnywhere
                    }

                    Item { Layout.preferredHeight: Theme.spacingUnit }

                    Text {
                        text: "Size: " + (workspace.selectedFile.sizeLabel || "—")
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        text: "Updated: " + (workspace.selectedFile.updatedAt || "—")
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        text: workspace.selectedFile.lockedBy
                            ? "Locked by " + workspace.selectedFile.lockedBy
                            : "Not locked"
                        color: workspace.selectedFile.lockedBy ? Theme.colorWarning : Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Item { Layout.fillHeight: true }

                    PrimaryButton {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.fillWidth: true
                        busy: workspace.treeModel && workspace.treeModel.busy
                        enabled: {
                            const lockedBy = workspace.selectedFile.lockedBy
                            const currentUser = workspace.authController ? workspace.authController.currentUserName : ""
                            return !lockedBy || lockedBy === currentUser
                        }
                        text: {
                            const lockedBy = workspace.selectedFile.lockedBy
                            const currentUser = workspace.authController ? workspace.authController.currentUserName : ""
                            if (!lockedBy)
                                return "Lock"
                            return lockedBy === currentUser ? "Unlock" : "Locked by " + lockedBy
                        }
                        onClicked: {
                            if (!workspace.treeModel)
                                return
                            const willLock = !workspace.selectedFile.lockedBy
                            workspace.treeModel.toggleLock(workspace.selectedFile.path, willLock)
                        }
                    }
                }
            }
        }
    }
}
