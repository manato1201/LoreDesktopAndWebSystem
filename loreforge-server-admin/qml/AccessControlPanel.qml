import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import LoreForgeServerAdmin

Rectangle {
    id: panel

    color: Theme.colorSurface
    radius: Theme.radiusComfortable
    border.color: Theme.colorBorder
    border.width: 1

    property var nodeRegistry: ({})
    property string pendingFrom: ""
    property var connections: []

    function registerNode(id, item) {
        nodeRegistry[id] = item
    }

    function edgeAnchor(item, side) {
        if (side === "right")
            return Qt.point(item.x + item.width, item.y + item.height / 2)
        return Qt.point(item.x, item.y + item.height / 2)
    }

    function nodeAt(px, py) {
        for (var key in nodeRegistry) {
            var it = nodeRegistry[key]
            if (px >= it.x && px <= it.x + it.width && py >= it.y && py <= it.y + it.height)
                return key
        }
        return ""
    }

    function tryConnect(fromId, globalPoint) {
        const targetId = nodeAt(globalPoint.x, globalPoint.y)
        if (targetId !== "" && targetId !== fromId) {
            const exists = connections.some(c => c.from === fromId && c.to === targetId)
            if (!exists) {
                connections = connections.concat([{ from: fromId, to: targetId }])
                lineCanvas.requestPaint()
            }
        }
        pendingFrom = ""
    }

    function clearConnections() {
        connections = []
        lineCanvas.requestPaint()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacingUnit * 2
        spacing: Theme.spacingUnit

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit

            Text {
                text: "Access Control"
                color: Theme.colorTextPrimary
                font.bold: true
                font.pixelSize: Theme.fontSizeHeading
            }

            Item { Layout.fillWidth: true }

            Button {
                text: "Clear connections"
                onClicked: panel.clearConnections()

                background: Rectangle {
                    radius: Theme.radiusStandard
                    color: "transparent"
                    border.color: Theme.colorBorderLight
                    border.width: 1
                }
                contentItem: Text {
                    text: parent.text
                    color: Theme.colorTextPrimary
                    font.pixelSize: Theme.fontSizeButton
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: "Drag a directory's right-side handle onto a role's left-side handle to grant that role access. "
                  + "UI-only first pass — connections are not persisted or applied to a server config yet (ARCHITECTURE.md §3.3)."
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeSmall
            font.italic: true
            wrapMode: Text.WordWrap
        }

        Item {
            id: canvasArea
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 420
            clip: true

            Rectangle {
                anchors.fill: parent
                color: Theme.colorBackgroundBase
                radius: Theme.radiusStandard
            }

            Canvas {
                id: lineCanvas
                anchors.fill: parent
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.reset()
                    ctx.strokeStyle = Theme.colorAccent
                    ctx.lineWidth = 2
                    for (var i = 0; i < panel.connections.length; i++) {
                        const c = panel.connections[i]
                        const fromItem = panel.nodeRegistry[c.from]
                        const toItem = panel.nodeRegistry[c.to]
                        if (!fromItem || !toItem)
                            continue
                        const p1 = panel.edgeAnchor(fromItem, "right")
                        const p2 = panel.edgeAnchor(toItem, "left")
                        const midX = (p1.x + p2.x) / 2
                        ctx.beginPath()
                        ctx.moveTo(p1.x, p1.y)
                        ctx.bezierCurveTo(midX, p1.y, midX, p2.y, p2.x, p2.y)
                        ctx.stroke()
                    }
                }
            }

            DirectoryNode {
                id: dirAssets
                nodeId: "dirAssets"
                label: "Assets"
                x: 32; y: 24
                highlighted: panel.pendingFrom === nodeId
                onDragged: lineCanvas.requestPaint()
                onConnectRequested: (id) => panel.pendingFrom = id
                onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                Component.onCompleted: panel.registerNode(nodeId, dirAssets)
            }

            DirectoryNode {
                id: dirCharacters
                nodeId: "dirCharacters"
                label: "Assets/Characters"
                x: 32; y: 100
                highlighted: panel.pendingFrom === nodeId
                onDragged: lineCanvas.requestPaint()
                onConnectRequested: (id) => panel.pendingFrom = id
                onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                Component.onCompleted: panel.registerNode(nodeId, dirCharacters)
            }

            DirectoryNode {
                id: dirEnvironments
                nodeId: "dirEnvironments"
                label: "Assets/Environments"
                x: 32; y: 176
                highlighted: panel.pendingFrom === nodeId
                onDragged: lineCanvas.requestPaint()
                onConnectRequested: (id) => panel.pendingFrom = id
                onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                Component.onCompleted: panel.registerNode(nodeId, dirEnvironments)
            }

            DirectoryNode {
                id: dirAudio
                nodeId: "dirAudio"
                label: "Assets/Audio"
                x: 32; y: 252
                highlighted: panel.pendingFrom === nodeId
                onDragged: lineCanvas.requestPaint()
                onConnectRequested: (id) => panel.pendingFrom = id
                onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                Component.onCompleted: panel.registerNode(nodeId, dirAudio)
            }

            DirectoryNode {
                id: dirSource
                nodeId: "dirSource"
                label: "Source"
                x: 32; y: 328
                highlighted: panel.pendingFrom === nodeId
                onDragged: lineCanvas.requestPaint()
                onConnectRequested: (id) => panel.pendingFrom = id
                onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                Component.onCompleted: panel.registerNode(nodeId, dirSource)
            }

            RoleNode {
                id: roleCharacterArtists
                nodeId: "roleCharacterArtists"
                label: "Character Artists"
                permissionLabel: "Read / Write"
                x: canvasArea.width - 232; y: 40
                onDragged: lineCanvas.requestPaint()
                Component.onCompleted: panel.registerNode(nodeId, roleCharacterArtists)
            }

            RoleNode {
                id: roleEnvironmentArtists
                nodeId: "roleEnvironmentArtists"
                label: "Environment Artists"
                permissionLabel: "Read / Write / Lock"
                x: canvasArea.width - 232; y: 140
                onDragged: lineCanvas.requestPaint()
                Component.onCompleted: panel.registerNode(nodeId, roleEnvironmentArtists)
            }

            RoleNode {
                id: roleQaContractors
                nodeId: "roleQaContractors"
                label: "QA Contractors"
                permissionLabel: "Read"
                x: canvasArea.width - 232; y: 240
                onDragged: lineCanvas.requestPaint()
                Component.onCompleted: panel.registerNode(nodeId, roleQaContractors)
            }
        }
    }

    onWidthChanged: lineCanvas.requestPaint()
    onHeightChanged: lineCanvas.requestPaint()
}
