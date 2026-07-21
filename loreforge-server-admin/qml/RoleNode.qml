import QtQuick
import LoreForgeServerAdmin

Rectangle {
    id: node

    property string nodeId: ""
    property string label: ""
    property string permissionLabel: ""
    property bool highlighted: false

    signal dragged()
    signal removeRequested(string sourceId)

    width: 200
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
        color: Theme.colorAccent
    }

    Column {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacingUnit * 2
        anchors.rightMargin: Theme.spacingUnit
        anchors.topMargin: Theme.spacingUnit
        anchors.bottomMargin: Theme.spacingUnit
        spacing: 2

        Text {
            text: node.label
            color: Theme.colorTextPrimary
            font.pixelSize: Theme.fontSizeSmall
            font.bold: true
            elide: Text.ElideRight
            width: parent.width
        }

        Text {
            text: node.permissionLabel
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeSmall
            elide: Text.ElideRight
            width: parent.width
        }
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
        color: hoverArea.containsMouse ? Theme.colorAccent : Theme.colorSurfaceInteractive
        border.color: Theme.colorAccentBorder
        border.width: 1
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: -7

        MouseArea {
            id: hoverArea
            anchors.fill: parent
            anchors.margins: -6
            hoverEnabled: true
        }
    }

    MouseArea {
        id: dragArea
        anchors.fill: parent
        anchors.leftMargin: 10
        drag.target: node
        drag.axis: Drag.XAndYAxis
        onPositionChanged: node.dragged()
    }
}
