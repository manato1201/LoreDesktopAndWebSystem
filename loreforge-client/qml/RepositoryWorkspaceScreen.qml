import QtQuick
import QtQuick.Layouts
import LoreForge

Item {
    id: workspace

    property var treeModel
    property var commitModel
    property var authController
    property string slug: ""
    property string repoName: ""
    property string selectedPath: ""
    property string activeTab: "files" // "files" | "history"
    property bool branchMenuOpen: false

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

    Component.onCompleted: {
        if (treeModel && slug.length > 0)
            treeModel.loadRepository(slug)
        if (commitModel && slug.length > 0)
            commitModel.refreshBranches(slug)
    }

    Connections {
        target: workspace.commitModel
        function onBranchCreated(name) {
            newBranchField.text = ""
            workspace.commitModel.checkout(name)
            workspace.branchMenuOpen = false
        }
        function onCommitSucceeded() {
            commitMessageField.text = ""
            commitDescriptionField.text = ""
            if (workspace.treeModel && workspace.slug.length > 0)
                workspace.treeModel.loadRepository(workspace.slug)
        }
    }

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

    // Mirrors CommitHistoryView.changeTypeColor's mapping so staged-file
    // indicators use the same added/modified/deleted color language as
    // committed-file badges.
    function stageColor(changeType) {
        switch (changeType) {
        case "added": return Theme.colorAccent
        case "deleted": return Theme.colorNegative
        default: return Theme.colorWarning
        }
    }

    function confirmNewBranch() {
        if (!workspace.commitModel)
            return
        const name = newBranchField.text.trim()
        if (name.length === 0)
            return
        workspace.commitModel.createBranch(name)
    }

    // Explicit sync action for UX parity with a real Git client — re-fetches
    // tree, commits, and branch state from the server rather than trusting
    // whatever this client last had in memory.
    function pull() {
        if (workspace.treeModel && workspace.slug.length > 0)
            workspace.treeModel.loadRepository(workspace.slug)
        if (workspace.commitModel && workspace.slug.length > 0) {
            workspace.commitModel.refresh(workspace.slug)
            workspace.commitModel.refreshBranches(workspace.slug)
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

            Rectangle {
                id: branchChip
                implicitWidth: branchChipRow.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: height / 2
                color: Theme.colorSurfaceElevated
                border.width: 1
                border.color: Theme.colorBorder

                RowLayout {
                    id: branchChipRow
                    anchors.centerIn: parent
                    spacing: Theme.spacingUnit / 2

                    Text {
                        text: "⌥"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        text: (workspace.commitModel && workspace.commitModel.currentBranch.length > 0)
                            ? workspace.commitModel.currentBranch : "main"
                        color: Theme.colorTextPrimary
                        font.bold: true
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Text {
                        text: workspace.branchMenuOpen ? "▴" : "▾"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeSmall
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: workspace.branchMenuOpen = !workspace.branchMenuOpen
                }
            }

            Rectangle {
                implicitWidth: pullLabel.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: Theme.radiusStandard
                color: Theme.colorSurfaceInteractive

                Text {
                    id: pullLabel
                    anchors.centerIn: parent
                    text: "⭯ Pull"
                    color: Theme.colorTextPrimary
                    font.pixelSize: Theme.fontSizeCaption
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: workspace.pull()
                }
            }

            Item { Layout.fillWidth: true }

            Text {
                visible: workspace.treeModel && workspace.treeModel.busy
                text: "Working…"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeCaption
            }
        }

        Rectangle {
            Layout.fillWidth: true
            visible: workspace.branchMenuOpen
            implicitHeight: branchPanelColumn.implicitHeight + Theme.spacingUnit * 2
            color: Theme.colorSurface
            radius: Theme.radiusComfortable

            ColumnLayout {
                id: branchPanelColumn
                anchors.fill: parent
                anchors.margins: Theme.spacingUnit
                spacing: Theme.spacingUnit / 2

                Repeater {
                    model: workspace.commitModel ? workspace.commitModel.branches : []

                    delegate: Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 28
                        radius: Theme.radiusSubtle
                        color: modelData.name === (workspace.commitModel ? workspace.commitModel.currentBranch : "")
                            ? Theme.colorSurfaceElevated : "transparent"

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacingUnit
                            anchors.rightMargin: Theme.spacingUnit
                            spacing: Theme.spacingUnit

                            Text {
                                text: modelData.name
                                color: Theme.colorTextPrimary
                                font.pixelSize: Theme.fontSizeCaption
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Text {
                                visible: modelData.isDefault === true
                                text: "default"
                                color: Theme.colorTextSecondary
                                font.pixelSize: Theme.fontSizeSmall
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                workspace.commitModel.checkout(modelData.name)
                                workspace.branchMenuOpen = false
                            }
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: Theme.spacingUnit
                    spacing: Theme.spacingUnit

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 32
                        radius: Theme.radiusStandard
                        color: Theme.colorSurfaceInteractive
                        border.width: newBranchField.activeFocus ? 1 : 0
                        border.color: Theme.colorAccent

                        TextInput {
                            id: newBranchField
                            anchors.fill: parent
                            anchors.margins: Theme.spacingUnit
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.colorTextPrimary
                            font.pixelSize: Theme.fontSizeCaption
                            selectByMouse: true
                            Keys.onReturnPressed: workspace.confirmNewBranch()
                        }

                        Text {
                            visible: newBranchField.text.length === 0 && !newBranchField.activeFocus
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.spacingUnit
                            anchors.verticalCenter: parent.verticalCenter
                            text: "New branch name…"
                            color: Theme.colorTextSecondary
                            font.pixelSize: Theme.fontSizeCaption
                        }
                    }

                    PrimaryButton {
                        text: "Create"
                        enabled: newBranchField.text.trim().length > 0
                        onClicked: workspace.confirmNewBranch()
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingUnit

            Rectangle {
                implicitWidth: filesTabLabel.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: height / 2
                color: workspace.activeTab === "files" ? Theme.colorSurfaceElevated : "transparent"
                border.width: workspace.activeTab === "files" ? 0 : 1
                border.color: Theme.colorBorder

                Text {
                    id: filesTabLabel
                    anchors.centerIn: parent
                    text: "Files"
                    color: workspace.activeTab === "files" ? Theme.colorTextPrimary : Theme.colorTextSecondary
                    font.bold: workspace.activeTab === "files"
                    font.pixelSize: Theme.fontSizeCaption
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: workspace.activeTab = "files"
                }
            }

            Rectangle {
                implicitWidth: historyTabLabel.implicitWidth + Theme.spacingUnit * 3
                implicitHeight: 32
                radius: height / 2
                color: workspace.activeTab === "history" ? Theme.colorSurfaceElevated : "transparent"
                border.width: workspace.activeTab === "history" ? 0 : 1
                border.color: Theme.colorBorder

                Text {
                    id: historyTabLabel
                    anchors.centerIn: parent
                    text: "History"
                    color: workspace.activeTab === "history" ? Theme.colorTextPrimary : Theme.colorTextSecondary
                    font.bold: workspace.activeTab === "history"
                    font.pixelSize: Theme.fontSizeCaption
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: workspace.activeTab = "history"
                }
            }

            Item { Layout.fillWidth: true }
        }

        Text {
            visible: workspace.activeTab === "files" && workspace.treeModel && workspace.treeModel.errorMessage.length > 0
            text: workspace.treeModel ? workspace.treeModel.errorMessage : ""
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeCaption
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacingUnit * 2
            visible: workspace.activeTab === "files"

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
                        id: treeRow
                        width: treeList.width
                        height: 36
                        radius: Theme.radiusSubtle
                        color: model.path === workspace.selectedPath ? Theme.colorSurfaceElevated
                            : rowMouse.containsMouse ? Theme.colorSurfaceInteractive : "transparent"

                        RowLayout {
                            // Above the full-row rowMouse (declared below, default
                            // z 0) so nested controls — staged-change squares, and
                            // the Sparse Workspace Manager's +/x affordances — take
                            // priority for clicks over the row-wide expand handler.
                            z: 1
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacingUnit + model.depth * (Theme.spacingUnit * 2)
                            anchors.rightMargin: Theme.spacingUnit
                            spacing: Theme.spacingUnit

                            // Expand/collapse arrow — only for directories already
                            // included (checked out) into the sparse workspace.
                            Text {
                                visible: !model.isDirectory || treeRow.rowIncluded
                                text: model.isDirectory ? (model.expanded ? "▾" : "▸") : ""
                                color: Theme.colorTextSecondary
                                font.pixelSize: Theme.fontSizeSmall
                                Layout.preferredWidth: 14
                            }

                            // Sparse Workspace Manager: a directory not yet checked
                            // out shows a "+" affordance in the arrow's slot instead.
                            Text {
                                visible: model.isDirectory && !treeRow.rowIncluded
                                text: "+"
                                color: Theme.colorAccent
                                font.pixelSize: Theme.fontSizeSmall
                                font.bold: true
                                Layout.preferredWidth: 14

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (workspace.treeModel)
                                            workspace.treeModel.toggleWorkspaceInclusion(treeRow.rowPath)
                                    }
                                }
                            }

                            // Small always-visible affordance to remove an included
                            // directory from the workspace again (no popup/menu
                            // component exists in this codebase, so a second icon
                            // next to the arrow is preferred over a context menu).
                            Text {
                                visible: model.isDirectory && treeRow.rowIncluded
                                text: "✕"
                                color: Theme.colorTextSecondary
                                font.pixelSize: 10
                                Layout.preferredWidth: 12

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (workspace.treeModel)
                                            workspace.treeModel.toggleWorkspaceInclusion(treeRow.rowPath)
                                    }
                                }
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
                                text: model.name + (model.isDirectory && !treeRow.rowIncluded ? "  ·  not in workspace" : "")
                                color: (model.isDirectory && !treeRow.rowIncluded) ? Theme.colorTextSecondary : Theme.colorTextPrimary
                                font.pixelSize: Theme.fontSizeCaption
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }

                            Rectangle {
                                visible: !model.isDirectory && model.stagedChangeType !== ""
                                width: 8
                                height: 8
                                radius: 4
                                color: workspace.stageColor(model.stagedChangeType)
                            }

                            RowLayout {
                                visible: !treeRow.rowIsDirectory
                                spacing: 2

                                Repeater {
                                    model: ["added", "modified", "deleted"]

                                    delegate: Rectangle {
                                        width: 14
                                        height: 14
                                        radius: 3
                                        border.width: 1
                                        border.color: workspace.stageColor(modelData)
                                        color: treeRow.stagedType === modelData ? workspace.stageColor(modelData) : "transparent"

                                        Text {
                                            anchors.centerIn: parent
                                            text: modelData.charAt(0).toUpperCase()
                                            color: treeRow.stagedType === modelData ? Theme.colorBackgroundBase : workspace.stageColor(modelData)
                                            font.pixelSize: 8
                                            font.bold: true
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (!workspace.treeModel)
                                                    return
                                                if (treeRow.stagedType === modelData)
                                                    workspace.treeModel.unstageChange(treeRow.rowPath)
                                                else
                                                    workspace.treeModel.stageChange(treeRow.rowPath, modelData)
                                            }
                                        }
                                    }
                                }
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

                        // Captured once per row (outside the nested Repeater
                        // above) so the Repeater's own `model`/`modelData`
                        // context inside its delegate doesn't shadow the
                        // ListView row's `model` role accessors.
                        property string rowPath: model.path
                        property string stagedType: model.stagedChangeType
                        property bool rowIsDirectory: model.isDirectory
                        // Sparse Workspace Manager inclusion flag; files have no
                        // meaningful "included" role, so default them to true so
                        // none of the directory-only visuals above apply to them.
                        property bool rowIncluded: model.isDirectory ? model.included : true

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

                    // --- Binary Diff Viewer (ARCHITECTURE.md §2.3) ---
                    // Image kind: draggable before/after slider, ported from
                    // lorehub-web's ImageDiffSlider.tsx. Both "current" and
                    // "before" bytes come from lorehub-api's authenticated
                    // image endpoints via the image://lore/... provider
                    // (LoreImageProvider) rather than a plain Image element,
                    // since a plain Image would bypass ApiClient's shared
                    // cookie jar and 401.
                    Item {
                        id: imagePreview
                        visible: workspace.selectedFile.kind === "image"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 180

                        property real sliderPosition: 50

                        Connections {
                            target: workspace
                            function onSelectedPathChanged() { imagePreview.sliderPosition = 50 }
                        }

                        Keys.onLeftPressed: imagePreview.sliderPosition = Math.max(0, imagePreview.sliderPosition - 5)
                        Keys.onRightPressed: imagePreview.sliderPosition = Math.min(100, imagePreview.sliderPosition + 5)

                        Rectangle {
                            id: previewFrame
                            anchors.fill: parent
                            radius: Theme.radiusComfortable
                            color: Theme.colorSurfaceElevated
                            clip: true

                            Image {
                                id: afterImage
                                anchors.fill: parent
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                                cache: false
                                source: (imagePreview.visible && workspace.selectedFile.path)
                                    ? "image://lore/" + workspace.slug + "/current/" + workspace.selectedFile.path
                                    : ""
                            }

                            Item {
                                id: beforeClip
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: previewFrame.width * (imagePreview.sliderPosition / 100)
                                clip: true

                                Image {
                                    id: beforeImage
                                    width: previewFrame.width
                                    height: previewFrame.height
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    cache: false
                                    source: (imagePreview.visible && workspace.selectedFile.path)
                                        ? "image://lore/" + workspace.slug + "/before/" + workspace.selectedFile.path
                                        : ""
                                }
                            }

                            Rectangle {
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.margins: Theme.spacingUnit
                                implicitWidth: beforeLabel.implicitWidth + Theme.spacingUnit * 2
                                implicitHeight: 20
                                radius: height / 2
                                color: Qt.rgba(0, 0, 0, 0.55)

                                Text {
                                    id: beforeLabel
                                    anchors.centerIn: parent
                                    text: "Before"
                                    color: Theme.colorTextSecondary
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeSmall
                                }
                            }

                            Rectangle {
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: Theme.spacingUnit
                                implicitWidth: afterLabel.implicitWidth + Theme.spacingUnit * 2
                                implicitHeight: 20
                                radius: height / 2
                                color: Qt.rgba(0, 0, 0, 0.55)

                                Text {
                                    id: afterLabel
                                    anchors.centerIn: parent
                                    text: "After"
                                    color: Theme.colorTextSecondary
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeSmall
                                }
                            }

                            Rectangle {
                                width: 2
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                x: previewFrame.width * (imagePreview.sliderPosition / 100) - width / 2
                                color: Theme.colorTextPrimary
                            }

                            Rectangle {
                                width: 26
                                height: 26
                                radius: 13
                                anchors.verticalCenter: parent.verticalCenter
                                x: previewFrame.width * (imagePreview.sliderPosition / 100) - width / 2
                                color: Theme.colorTextPrimary

                                Text {
                                    anchors.centerIn: parent
                                    text: "⇔"
                                    color: Theme.colorBackgroundBase
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeCaption
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.SizeHorCursor
                                onPressed: (mouse) => {
                                    imagePreview.forceActiveFocus()
                                    imagePreview.sliderPosition = Math.min(100, Math.max(0, (mouse.x / previewFrame.width) * 100))
                                }
                                onPositionChanged: (mouse) => {
                                    if (pressed)
                                        imagePreview.sliderPosition = Math.min(100, Math.max(0, (mouse.x / previewFrame.width) * 100))
                                }
                            }
                        }
                    }

                    Text {
                        visible: imagePreview.visible
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        text: "Drag or use arrow keys to compare"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeSmall
                    }

                    // 3D kind: Before/After pill toggle around a stylized
                    // wireframe stand-in, ported from lorehub-web's
                    // Model3DDiffToggle.tsx (whose own "3D viewer" is
                    // likewise a procedural Three.js placeholder, not a real
                    // FBX/OBJ loader — no QtQuick3D/Qt3D dependency here).
                    Item {
                        id: modelPreview
                        visible: workspace.selectedFile.kind === "model3d"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 210

                        property string variant: "after" // "before" | "after"

                        Connections {
                            target: workspace
                            function onSelectedPathChanged() { modelPreview.variant = "after" }
                        }

                        ColumnLayout {
                            anchors.fill: parent
                            spacing: Theme.spacingUnit

                            Rectangle {
                                id: togglePill
                                Layout.alignment: Qt.AlignHCenter
                                implicitWidth: toggleRow.implicitWidth + Theme.spacingUnit
                                implicitHeight: 30
                                radius: height / 2
                                color: Theme.colorSurfaceInteractive

                                RowLayout {
                                    id: toggleRow
                                    anchors.centerIn: parent
                                    spacing: 2

                                    Repeater {
                                        model: ["before", "after"]

                                        delegate: Rectangle {
                                            implicitWidth: variantLabel.implicitWidth + Theme.spacingUnit * 2
                                            implicitHeight: 24
                                            radius: height / 2
                                            color: modelPreview.variant === modelData ? Theme.colorAccent : "transparent"

                                            Text {
                                                id: variantLabel
                                                anchors.centerIn: parent
                                                text: modelData === "before" ? "Before" : "After"
                                                color: modelPreview.variant === modelData ? Theme.colorBackgroundBase : Theme.colorTextSecondary
                                                font.bold: true
                                                font.pixelSize: Theme.fontSizeSmall
                                            }

                                            MouseArea {
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: modelPreview.variant = modelData
                                            }
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 140
                                radius: Theme.radiusComfortable
                                color: Theme.colorSurfaceElevated

                                // Purely illustrative wireframe silhouette —
                                // re-angled and re-colored between variants
                                // so the toggle visibly does something.
                                Item {
                                    anchors.centerIn: parent
                                    width: 96
                                    height: 96
                                    rotation: modelPreview.variant === "before" ? -10 : 10

                                    Rectangle {
                                        anchors.fill: parent
                                        radius: Theme.radiusStandard
                                        color: "transparent"
                                        border.width: 2
                                        border.color: modelPreview.variant === "before" ? Theme.colorBorderLight : Theme.colorAccent
                                    }

                                    Rectangle {
                                        width: parent.width * 0.58
                                        height: parent.height * 0.58
                                        anchors.centerIn: parent
                                        rotation: 45
                                        radius: Theme.radiusSubtle
                                        color: "transparent"
                                        border.width: 2
                                        border.color: modelPreview.variant === "before" ? Theme.colorBorderLight : Theme.colorAccent
                                    }

                                    Rectangle {
                                        width: 2
                                        height: parent.height
                                        anchors.centerIn: parent
                                        color: modelPreview.variant === "before" ? Theme.colorBorderLight : Theme.colorAccent
                                    }

                                    Rectangle {
                                        width: parent.width
                                        height: 2
                                        anchors.centerIn: parent
                                        color: modelPreview.variant === "before" ? Theme.colorBorderLight : Theme.colorAccent
                                    }
                                }
                            }

                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: (modelPreview.variant === "before" ? "Before" : "After") + " — " + (workspace.selectedFile.name || "")
                                color: Theme.colorTextSecondary
                                font.pixelSize: Theme.fontSizeSmall
                            }
                        }
                    }

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

        Rectangle {
            Layout.fillWidth: true
            visible: workspace.activeTab === "files"
            implicitHeight: commitPanelColumn.implicitHeight + Theme.spacingUnit * 2
            color: Theme.colorSurface
            radius: Theme.radiusComfortable

            ColumnLayout {
                id: commitPanelColumn
                anchors.fill: parent
                anchors.margins: Theme.spacingUnit * 2
                spacing: Theme.spacingUnit

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingUnit

                    Text {
                        text: (workspace.treeModel ? workspace.treeModel.pendingCount : 0) + " staged change(s)"
                        color: Theme.colorTextPrimary
                        font.bold: true
                        font.pixelSize: Theme.fontSizeCaption
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        visible: workspace.commitModel && workspace.commitModel.errorMessage.length > 0
                        text: workspace.commitModel ? workspace.commitModel.errorMessage : ""
                        color: Theme.colorNegative
                        font.pixelSize: Theme.fontSizeSmall
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 36
                    radius: Theme.radiusStandard
                    color: Theme.colorSurfaceInteractive
                    border.width: commitMessageField.activeFocus ? 1 : 0
                    border.color: Theme.colorAccent

                    TextInput {
                        id: commitMessageField
                        anchors.fill: parent
                        anchors.margins: Theme.spacingUnit
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.colorTextPrimary
                        font.pixelSize: Theme.fontSizeCaption
                        selectByMouse: true
                    }

                    Text {
                        visible: commitMessageField.text.length === 0 && !commitMessageField.activeFocus
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spacingUnit
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Commit message (required)"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 36
                    radius: Theme.radiusStandard
                    color: Theme.colorSurfaceInteractive
                    border.width: commitDescriptionField.activeFocus ? 1 : 0
                    border.color: Theme.colorAccent

                    TextInput {
                        id: commitDescriptionField
                        anchors.fill: parent
                        anchors.margins: Theme.spacingUnit
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.colorTextPrimary
                        font.pixelSize: Theme.fontSizeCaption
                        selectByMouse: true
                    }

                    Text {
                        visible: commitDescriptionField.text.length === 0 && !commitDescriptionField.activeFocus
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spacingUnit
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Description (optional)"
                        color: Theme.colorTextSecondary
                        font.pixelSize: Theme.fontSizeCaption
                    }
                }

                PrimaryButton {
                    Layout.alignment: Qt.AlignRight
                    text: "Commit & Push " + (workspace.treeModel ? workspace.treeModel.pendingCount : 0) + " change(s)"
                    busy: workspace.commitModel && workspace.commitModel.busy
                    enabled: (workspace.treeModel ? workspace.treeModel.pendingCount : 0) > 0
                        && commitMessageField.text.trim().length > 0
                    onClicked: {
                        if (!workspace.commitModel)
                            return
                        workspace.commitModel.commit(commitMessageField.text, commitDescriptionField.text)
                    }
                }
            }
        }

        CommitHistoryView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: workspace.activeTab === "history"
            commitModel: workspace.commitModel
            slug: workspace.slug
        }
    }
}
