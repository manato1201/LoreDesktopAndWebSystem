import QtQuick
import LoreForgeServerAdmin

Rectangle {
    id: node

    property string nodeId: ""
    property string label: ""
    property Item canvasRoot: parent
    property bool highlighted: false

    signal dragged()
    signal connectRequested(string sourceId)
    signal connectReleased(string sourceId, point globalPos)
    signal removeRequested(string sourceId)

    width: 168
    height: 56
    z: dragArea.pressed ? 100 : 1
    radius: Theme.radiusComfortable
    color: dragArea.pressed ? Theme.colorSurfaceElevated : Theme.colorSurface
    border.color: node.highlighted ? Theme.colorAccent : Theme.colorBorder
    border.width: node.highlighted ? 2 : 1

    Rectangle {
        width: 6
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        radius: Theme.radiusSubtle
        color: Theme.colorAnnouncement
    }

    Text {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacingUnit * 2
        anchors.rightMargin: Theme.spacingUnit
        verticalAlignment: Text.AlignVCenter
        text: node.label
        color: Theme.colorTextPrimary
        font.pixelSize: Theme.fontSizeCaption
        elide: Text.ElideRight
    }

    // Small always-visible delete affordance, matching the "✕" pattern used
    // elsewhere in this codebase (e.g. loreforge-client's sparse-workspace
    // tree) instead of a context menu, which this codebase has no component
    // for.
    Text {
        text: "×"
        color: Theme.colorTextSecondary
        font.pixelSize: 14
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 2
        z: 10

        MouseArea {
            anchors.fill: parent
            anchors.margins: -5
            cursorShape: Qt.PointingHandCursor
            onClicked: node.removeRequested(node.nodeId)
        }
    }

    Rectangle {
        width: 14
        height: 14
        radius: 7
        color: connectArea.containsMouse || connectArea.pressed ? Theme.colorAccent : Theme.colorSurfaceInteractive
        border.color: Theme.colorAccentBorder
        border.width: 1
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.rightMargin: -7

        MouseArea {
            id: connectArea
            anchors.fill: parent
            anchors.margins: -6
            hoverEnabled: true
            onPressed: node.connectRequested(node.nodeId)
            onReleased: (mouse) => {
                const globalPos = connectArea.mapToItem(node.canvasRoot, mouse.x, mouse.y)
                node.connectReleased(node.nodeId, globalPos)
            }
        }
    }

    MouseArea {
        id: dragArea
        anchors.fill: parent
        anchors.rightMargin: 10
        drag.target: node
        drag.axis: Drag.XAndYAxis
        onPositionChanged: node.dragged()
    }
}
