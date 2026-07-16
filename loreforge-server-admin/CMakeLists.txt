cmake_minimum_required(VERSION 3.24)
project(LoreForgeServerAdmin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Network)

qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(LoreForgeServerAdmin
    src/main.cpp
)

# pragma Singleton in Theme.qml isn't enough on its own — qt_add_qml_module
# needs this source property to mark it as a singleton in the generated
# qmldir, otherwise `Theme.colorAccent` etc. silently resolve to undefined.
set_source_files_properties(qml/Theme.qml PROPERTIES
    QT_QML_SINGLETON_TYPE TRUE
)

qt_add_qml_module(LoreForgeServerAdmin
    URI LoreForgeServerAdmin
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/Theme.qml
        qml/EnvironmentPanel.qml
        qml/AccessControlPanel.qml
        qml/StatusCard.qml
        qml/DirectoryNode.qml
        qml/RoleNode.qml
    SOURCES
        src/ApiClient.h
        src/ApiClient.cpp
        src/DockerController.h
        src/DockerController.cpp
        src/LoreServerController.h
        src/LoreServerController.cpp
        src/PermissionConfigController.h
        src/PermissionConfigController.cpp
)

target_include_directories(LoreForgeServerAdmin
    PRIVATE
        src
)

target_link_libraries(LoreForgeServerAdmin
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::QuickControls2
        Qt6::Network
)

set_target_properties(LoreForgeServerAdmin PROPERTIES
    WIN32_EXECUTABLE TRUE
)
