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
    property var nodeTypes: ({})
    property string pendingFrom: ""
    property var connections: []

    property var directoryModel: []
    property var roleModel: []

    property string saveFeedback: ""
    property color saveFeedbackColor: Theme.colorTextSecondary

    PermissionConfigController {
        id: permissionConfig
    }

    function registerNode(id, item, kind) {
        nodeRegistry[id] = item
        nodeTypes[id] = kind
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

    function defaultDirectoryModel() {
        return [
            { id: "dirAssets", path: "Assets", x: 32, y: 24 },
            { id: "dirCharacters", path: "Assets/Characters", x: 32, y: 100 },
            { id: "dirEnvironments", path: "Assets/Environments", x: 32, y: 176 },
            { id: "dirAudio", path: "Assets/Audio", x: 32, y: 252 },
            { id: "dirSource", path: "Source", x: 32, y: 328 }
        ]
    }

    // x intentionally omitted here: the delegate below falls back to a
    // canvasArea.width-relative binding when x is undefined, so freshly
    // spawned role nodes stay right-aligned even if the canvas hasn't been
    // laid out yet at model-build time (loaded configs always carry a
    // concrete x and skip this fallback entirely).
    function defaultRoleModel() {
        return [
            { id: "roleCharacterArtists", principal: "Character Artists", permissionLabel: "Read / Write", y: 40 },
            { id: "roleEnvironmentArtists", principal: "Environment Artists", permissionLabel: "Read / Write / Lock", y: 140 },
            { id: "roleQaContractors", principal: "QA Contractors", permissionLabel: "Read", y: 240 }
        ]
    }

    function loadInitialState() {
        if (permissionConfig.hasSavedConfig && permissionConfig.directoryNodes.length > 0) {
            directoryModel = permissionConfig.directoryNodes
            roleModel = permissionConfig.roleNodes
            connections = permissionConfig.connections
            saveFeedback = "Loaded saved access control config."
            saveFeedbackColor = Theme.colorTextSecondary
        } else {
            resetToDefaults()
        }
    }

    function resetToDefaults() {
        directoryModel = defaultDirectoryModel()
        roleModel = defaultRoleModel()
        connections = []
        saveFeedback = ""
        lineCanvas.requestPaint()
    }

    function saveToConfig() {
        const nodes = []
        for (var key in nodeRegistry) {
            const item = nodeRegistry[key]
            const kind = nodeTypes[key]
            if (kind === "directory")
                nodes.push({ id: key, type: kind, path: item.label, x: item.x, y: item.y })
            else
                nodes.push({ id: key, type: kind, principal: item.label, permissionLabel: item.permissionLabel, x: item.x, y: item.y })
        }

        const ok = permissionConfig.saveConfig(nodes, panel.connections)
        if (ok) {
            saveFeedback = "Saved to " + permissionConfig.configPath
            saveFeedbackColor = Theme.colorAccent
        } else {
            saveFeedback = permissionConfig.lastError.length > 0
                ? permissionConfig.lastError
                : "Failed to save access control config."
            saveFeedbackColor = Theme.colorNegative
        }
    }

    Component.onCompleted: Qt.callLater(panel.loadInitialState)

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
                text: "Reset to defaults"
                onClicked: panel.resetToDefaults()

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

            Button {
                text: "Save"
                onClicked: panel.saveToConfig()

                background: Rectangle {
                    radius: Theme.radiusStandard
                    color: parent.hovered ? Theme.colorAccentBorder : Theme.colorAccent
                }
                contentItem: Text {
                    text: parent.text
                    color: "#121212"
                    font.bold: true
                    font.pixelSize: Theme.fontSizeButton
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: "Drag a directory's right-side handle onto a role's left-side handle to grant that role access. "
                  + "Save writes the graph to a Lore server access-control JSON file, which is reloaded automatically next launch (ARCHITECTURE.md §3.3)."
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeSmall
            font.italic: true
            wrapMode: Text.WordWrap
        }

        Text {
            Layout.fillWidth: true
            text: panel.saveFeedback
            visible: panel.saveFeedback.length > 0
            color: panel.saveFeedbackColor
            font.pixelSize: Theme.fontSizeSmall
            wrapMode: Text.WordWrap
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: Theme.spacingUnit
            height: applyRow.implicitHeight + Theme.spacingUnit * 2
            radius: Theme.radiusStandard
            color: Theme.colorSurfaceInteractive
            border.color: Theme.colorBorderLight
            border.width: 1

            RowLayout {
                id: applyRow
                anchors.fill: parent
                anchors.margins: Theme.spacingUnit
                spacing: Theme.spacingUnit

                Text {
                    text: permissionConfig.connected
                          ? "Connected as " + permissionConfig.connectedAs
                          : "Not connected to the LoreHub server."
                    color: permissionConfig.connected ? Theme.colorAccent : Theme.colorTextSecondary
                    font.pixelSize: Theme.fontSizeSmall
                    Layout.preferredWidth: 200
                    wrapMode: Text.WordWrap
                }

                Rectangle {
                    visible: !permissionConfig.connected
                    Layout.preferredWidth: 160
                    height: 32
                    radius: Theme.radiusStandard
                    color: Theme.colorSurface
                    border.width: emailField.activeFocus ? 1 : 0
                    border.color: Theme.colorAccent

                    TextInput {
                        id: emailField
                        anchors.fill: parent
                        anchors.margins: Theme.spacingUnit * 0.5
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.colorTextPrimary
                        font.pixelSize: Theme.fontSizeSmall
                        selectByMouse: true
                        text: "aiko.tanaka@nebula.studio"
                        KeyNavigation.tab: passwordField
                    }
                }

                Rectangle {
                    visible: !permissionConfig.connected
                    Layout.preferredWidth: 120
                    height: 32
                    radius: Theme.radiusStandard
                    color: Theme.colorSurface
                    border.width: passwordField.activeFocus ? 1 : 0
                    border.color: Theme.colorAccent

                    TextInput {
                        id: passwordField
                        anchors.fill: parent
                        anchors.margins: Theme.spacingUnit * 0.5
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.colorTextPrimary
                        font.pixelSize: Theme.fontSizeSmall
                        echoMode: TextInput.Password
                        selectByMouse: true
                        text: "lorehub"
                        Keys.onReturnPressed: permissionConfig.login(emailField.text, passwordField.text)
                    }
                }

                Button {
                    visible: !permissionConfig.connected
                    text: "Connect"
                    onClicked: permissionConfig.login(emailField.text, passwordField.text)

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

                Item { Layout.fillWidth: true }

                Button {
                    text: permissionConfig.applying ? "Applying…" : "Apply to LoreHub Server"
                    enabled: permissionConfig.connected && panel.directoryModel.length > 0 && !permissionConfig.applying
                    onClicked: permissionConfig.applyToServer()

                    background: Rectangle {
                        radius: Theme.radiusStandard
                        opacity: parent.enabled ? 1.0 : 0.5
                        color: parent.hovered ? Theme.colorAccentBorder : Theme.colorAccent
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "#121212"
                        font.bold: true
                        font.pixelSize: Theme.fontSizeButton
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: permissionConfig.applyError.length > 0 ? permissionConfig.applyError : permissionConfig.applySuccess
            visible: permissionConfig.applyError.length > 0 || permissionConfig.applySuccess.length > 0
            color: permissionConfig.applyError.length > 0 ? Theme.colorNegative : Theme.colorAccent
            font.pixelSize: Theme.fontSizeSmall
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

            Repeater {
                model: panel.directoryModel

                DirectoryNode {
                    nodeId: modelData.id
                    label: modelData.path
                    x: modelData.x
                    y: modelData.y
                    highlighted: panel.pendingFrom === nodeId
                    onDragged: lineCanvas.requestPaint()
                    onConnectRequested: (id) => panel.pendingFrom = id
                    onConnectReleased: (id, pos) => panel.tryConnect(id, pos)
                    Component.onCompleted: {
                        panel.registerNode(nodeId, this, "directory")
                        lineCanvas.requestPaint()
                    }
                }
            }

            Repeater {
                model: panel.roleModel

                RoleNode {
                    nodeId: modelData.id
                    label: modelData.principal
                    permissionLabel: modelData.permissionLabel
                    x: modelData.x !== undefined ? modelData.x : Math.max(232, canvasArea.width - 232)
                    y: modelData.y
                    onDragged: lineCanvas.requestPaint()
                    Component.onCompleted: {
                        panel.registerNode(nodeId, this, "role")
                        lineCanvas.requestPaint()
                    }
                }
            }
        }
    }

    onWidthChanged: lineCanvas.requestPaint()
    onHeightChanged: lineCanvas.requestPaint()
}
