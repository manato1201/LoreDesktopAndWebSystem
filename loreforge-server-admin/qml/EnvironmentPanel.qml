import QtQuick
import QtQuick.Layouts
import LoreForgeServerAdmin

Item {
    id: root

    DockerController {
        id: docker
        Component.onCompleted: checkDockerAvailable()
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

                title: "Lore Server"
                statusText: "Not implemented"
                statusColor: Theme.colorTextSecondary
                caption: "Placeholder only — Lore is a fictional VCS invented for this project's premise, so there is no real Lore server binary to launch here. This card exists to show where server process control would live once a real implementation exists."
                captionItalic: true
            }
        }
    }
}
