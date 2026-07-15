import QtQuick
import QtQuick.Layouts
import LoreForge

Item {
    id: loginScreen

    property var authController

    Rectangle {
        anchors.fill: parent
        color: Theme.colorBackgroundBase
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: 360
        spacing: Theme.spacingUnit * 2

        Text {
            text: "LoreForge"
            color: Theme.colorAccent
            font.bold: true
            font.pixelSize: Theme.fontSizeSectionTitle
            Layout.alignment: Qt.AlignHCenter
        }

        Text {
            text: "Sign in to your LoreHub account"
            color: Theme.colorTextSecondary
            font.pixelSize: Theme.fontSizeCaption
            Layout.alignment: Qt.AlignHCenter
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.topMargin: Theme.spacingUnit * 2
            spacing: Theme.spacingUnit

            Text {
                text: "Email"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeSmall
            }

            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Theme.radiusStandard
                color: Theme.colorSurfaceInteractive
                border.width: emailField.activeFocus ? 1 : 0
                border.color: Theme.colorAccent

                TextInput {
                    id: emailField
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.colorTextPrimary
                    font.pixelSize: Theme.fontSizeBody
                    selectByMouse: true
                    text: "aiko.tanaka@nebula.studio"
                    KeyNavigation.tab: passwordField
                }
            }

            Text {
                text: "Password"
                color: Theme.colorTextSecondary
                font.pixelSize: Theme.fontSizeSmall
                Layout.topMargin: Theme.spacingUnit
            }

            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Theme.radiusStandard
                color: Theme.colorSurfaceInteractive
                border.width: passwordField.activeFocus ? 1 : 0
                border.color: Theme.colorAccent

                TextInput {
                    id: passwordField
                    anchors.fill: parent
                    anchors.margins: Theme.spacingUnit
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.colorTextPrimary
                    font.pixelSize: Theme.fontSizeBody
                    echoMode: TextInput.Password
                    selectByMouse: true
                    text: "lorehub"
                    Keys.onReturnPressed: loginScreen.submit()
                }
            }
        }

        Text {
            text: loginScreen.authController ? loginScreen.authController.errorMessage : ""
            visible: text.length > 0
            color: Theme.colorNegative
            font.pixelSize: Theme.fontSizeSmall
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        PrimaryButton {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Theme.spacingUnit
            text: "Log In"
            busy: loginScreen.authController ? loginScreen.authController.busy : false
            onClicked: loginScreen.submit()
        }
    }

    function submit() {
        if (authController)
            authController.login(emailField.text, passwordField.text)
    }
}
