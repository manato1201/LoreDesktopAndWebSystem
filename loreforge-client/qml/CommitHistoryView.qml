import QtQuick
import QtQuick.Layouts
import LoreForge

Item {
    id: history

    property var commitModel
    property string slug: ""
    property string selectedHash: ""

    // Re-evaluated whenever commitModel.count changes since rowForHash()
    // reads C++ state that isn't itself a bindable property.
    property var selectedCommit: {
        if (history.commitModel)
            history.commitModel.count
        return history.commitModel && history.selectedHash.length > 0
            ? history.commitModel.rowForHash(history.selectedHash)
            : ({})
    }

    function branchColor(lane) {
        var palette = [Theme.colorAccent, Theme.colorAnnouncement, Theme.colorWarning,
            Theme.colorNegative, Theme.colorAccentBorder]
        return palette[lane % palette.length]
    }

    function changeTypeColor(changeType) {
        switch (changeType) {
        case "added": return Theme.colorAccent
        case "deleted": return Theme.colorNegative
        default: return Theme.colorWarning
        }
    }

    Component.onCompleted: if (commitModel && slug.length > 0) commitModel.refresh(slug)
    onSlugChanged: if (commitModel && slug.length > 0) commitModel.refresh(slug)

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingUnit

        Text {
            visible: history.commitModel && history.commitModel.errorMessage.length > 0
            text: history.commitModel ? history.commitModel.errorMessage : ""
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeCaption
        }

        Text {
            visible: history.commitModel && history.commitModel.busy
            text: "Loading commits…"
            color: Theme.colorTextSecondary
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
                    id: commitList
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit
                    clip: true
                    spacing: 2
                    model: history.commitModel

                    delegate: Rectangle {
                        width: commitList.width
                        height: 52
                        radius: Theme.radiusSubtle
                        color: model.hash === history.selectedHash ? Theme.colorSurfaceElevated
                            : commitMouse.containsMouse ? Theme.colorSurfaceInteractive : "transparent"

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: Theme.spacingUnit
                            spacing: Theme.spacingUnit

                            Rectangle {
                                width: 8
                                height: 8
                                radius: 4
                                color: history.branchColor(model.branchLane)
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: model.message
                                    color: Theme.colorTextPrimary
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeCaption
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }

                                Text {
                                    text: model.author + " · " + model.timestamp + " · " + model.shortHash
                                    color: Theme.colorTextSecondary
                                    font.pixelSize: Theme.fontSizeSmall
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }

                            Rectangle {
                                radius: Theme.radiusSubtle
                                color: Theme.colorSurfaceInteractive
                                implicitWidth: branchLabel.implicitWidth + Theme.spacingUnit * 2
                                implicitHeight: 20

                                Text {
                                    id: branchLabel
                                    anchors.centerIn: parent
                                    text: model.branch
                                    color: history.branchColor(model.branchLane)
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }
                        }

                        MouseArea {
                            id: commitMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: history.selectedHash = model.hash
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 320
                Layout.fillHeight: true
                color: Theme.colorSurface
                radius: Theme.radiusComfortable
                visible: history.selectedHash.length > 0

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit * 2
                    spacing: Theme.spacingUnit

                    Text {
                        text: history.selectedCommit.message || ""
                        color: Theme.colorTextPrimary
                        font.bold: true
                        font.pixelSize: Theme.fontSizeHeading
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Text {
                        visible: (history.selectedCommit.description || "").length > 0
                        text: history.selectedCommit.description || ""
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Item { Layout.preferredHeight: Theme.spacingUnit }

                    Text {
                        text: (history.selectedCommit.author || "") + " (" + (history.selectedCommit.authorInitials || "") + ")"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        text: history.selectedCommit.shortHash || ""
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeSmall
                    }

                    Item { Layout.preferredHeight: Theme.spacingUnit }

                    Text {
                        text: "Changed files"
                        color: Theme.colorTextPrimary
                        font.bold: true
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        visible: (history.selectedCommit.changedFiles || []).length === 0
                        text: "No file changes (merge commit)"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeSmall
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        spacing: Theme.spacingUnit / 2
                        model: history.selectedCommit.changedFiles || []

                        delegate: RowLayout {
                            width: ListView.view.width
                            spacing: Theme.spacingUnit

                            Rectangle {
                                radius: Theme.radiusSubtle
                                color: Theme.colorSurfaceInteractive
                                implicitWidth: changeLabel.implicitWidth + Theme.spacingUnit * 1.5
                                implicitHeight: 18

                                Text {
                                    id: changeLabel
                                    anchors.centerIn: parent
                                    text: modelData.changeType.toUpperCase()
                                    color: history.changeTypeColor(modelData.changeType)
                                    font.pixelSize: 9
                                    font.bold: true
                                }
                            }

                            Text {
                                text: modelData.path
                                color: Theme.colorTextPrimary
                                font.pixelSize: Theme.fontSizeSmall
                                Layout.fillWidth: true
                                elide: Text.ElideMiddle
                            }

                            Text {
                                text: modelData.sizeDeltaLabel
                                color: Theme.colorTextSecondary
                                font.pixelSize: Theme.fontSizeSmall
                            }
                        }
                    }
                }
            }
        }
    }
}
