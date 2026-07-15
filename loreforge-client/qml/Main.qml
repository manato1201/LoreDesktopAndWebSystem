import QtQuick
import QtQuick.Controls
import LoreForge

ApplicationWindow {
    id: window
    width: 1280
    height: 840
    visible: true
    title: "LoreForge Client"
    color: Theme.colorBackgroundBase

    property string activeScreen: "login" // "login" | "repositories" | "workspace"
    property string activeSlug: ""
    property string activeRepoName: ""

    // Ids are deliberately distinct from the screens' own `authController` /
    // `repositoryModel` / `treeModel` properties below — reusing the same
    // name creates a self-referential QML binding loop (the property
    // shadows the outer id inside its own binding expression) and silently
    // resolves to undefined instead of the intended object.
    AuthController {
        id: authControllerInstance
        onLoggedInChanged: window.activeScreen = authControllerInstance.loggedIn ? "repositories" : "login"
    }

    RepositoryListModel {
        id: repositoryModelInstance
    }

    RepositoryTreeModel {
        id: treeModelInstance
    }

    CommitListModel {
        id: commitModelInstance
    }

    Loader {
        anchors.fill: parent
        sourceComponent: {
            if (window.activeScreen === "workspace")
                return workspaceComponent
            if (window.activeScreen === "repositories")
                return repositoryListComponent
            return loginComponent
        }
    }

    Component {
        id: loginComponent
        LoginScreen {
            authController: authControllerInstance
        }
    }

    Component {
        id: repositoryListComponent
        RepositoryListScreen {
            repositoryModel: repositoryModelInstance
            authController: authControllerInstance
            onRepositorySelected: (slug, name) => {
                window.activeSlug = slug
                window.activeRepoName = name
                window.activeScreen = "workspace"
            }
        }
    }

    Component {
        id: workspaceComponent
        RepositoryWorkspaceScreen {
            treeModel: treeModelInstance
            commitModel: commitModelInstance
            authController: authControllerInstance
            slug: window.activeSlug
            repoName: window.activeRepoName
            onBackRequested: window.activeScreen = "repositories"
        }
    }
}
