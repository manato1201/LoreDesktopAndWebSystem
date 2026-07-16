import QtQuick
import QtQuick.Layouts
import LoreForgeServerAdmin

Item {
    id: root

    DockerController {
        id: docker
        Component.onCompleted: checkDockerAvailable()
    }

    LoreServerController {
        id: loreServer
    }

    function minioColor() {
        switch (docker.minioStatus) {
        case DockerController.Running:
            return Theme.colorAccent
        case DockerController.Stopped:
            return Theme.colorWarning
        default:
            return Theme.colorTextSecondary
        }
    }

    function loreServerColor() {
        switch (loreServer.status) {
        case LoreServerController.Running:
            return Theme.colorAccent
        case LoreServerController.Starting:
            return Theme.colorWarning
        case LoreServerController.Error:
            return Theme.colorNegative
        default:
            return Theme.colorTextSecondary
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingUnit * 2

        Text {
            text: docker.dockerAvailable
                  ? "Docker detected on this machine."
                  : (docker.busy ? "Checking for Docker…" : "Docker was not detected. Install Docker Desktop to manage local containers.")
            color: docker.dockerAvailable ? Theme.colorTextSecondary : Theme.colorWarning
            font.pixelSize: Theme.fontSizeCaption
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Text {
            text: docker.lastError
            visible: docker.lastError.length > 0
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeSmall
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Text {
            text: loreServer.lastError
            visible: loreServer.lastError.length > 0
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeSmall
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacingUnit * 2

            StatusCard {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1

                title: "MinIO (S3-compatible storage)"
                busy: docker.busy
                statusText: docker.minioStatusText
                statusColor: root.minioColor()
                caption: "S3-compatible object store backing chunked binary storage (ARCHITECTURE.md §5.1). Container: lorehub-minio, ports 9000/9001."
                detailText: docker.minioStatus === DockerController.Running ? docker.minioStatsLabel : ""

                primaryActionLabel: docker.minioStatus === DockerController.Running ? "Stop" : "Start"
                primaryActionEnabled: docker.dockerAvailable
                onPrimaryActionTriggered: {
                    if (docker.minioStatus === DockerController.Running)
                        docker.stopMinio()
                    else
                        docker.startMinio()
                }

                secondaryActionLabel: "Refresh"
                secondaryActionEnabled: docker.dockerAvailable
                onSecondaryActionTriggered: docker.refreshMinioStatus()
            }

            StatusCard {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1

                title: "Lore Server (lorehub-api)"
                busy: loreServer.busy
                statusText: loreServer.statusText
                statusColor: root.loreServerColor()
                caption: "The real Rust/Axum backend that LoreHub Web and LoreForge Client talk to over HTTP. Controlled here as a local process (ARCHITECTURE.md §3.3 allows a Docker container or a local process)."
                detailText: loreServer.status === LoreServerController.Running
                            ? "PID %1 · %2".arg(loreServer.pid).arg(loreServer.memoryUsageLabel.length > 0 ? loreServer.memoryUsageLabel : "…")
                            : ""

                primaryActionLabel: (loreServer.status === LoreServerController.Running
                                      || loreServer.status === LoreServerController.Starting) ? "Stop" : "Start"
                primaryActionEnabled: loreServer.status !== LoreServerController.Starting
                onPrimaryActionTriggered: {
                    if (loreServer.status === LoreServerController.Running
                            || loreServer.status === LoreServerController.Starting)
                        loreServer.stopServer()
                    else
                        loreServer.startServer()
                }

                secondaryActionLabel: "Refresh"
                secondaryActionEnabled: loreServer.status === LoreServerController.Running
                onSecondaryActionTriggered: loreServer.refreshStatus()
            }
        }
    }
}
