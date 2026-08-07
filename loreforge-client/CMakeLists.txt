cmake_minimum_required(VERSION 3.24)
project(LoreForgeClient LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# Route target output (both LoreForgeClient and LoreForgeClientTests) into a
# single flat directory so the windeployqt POST_BUILD step and the CPack
# install() rule further below have one well-known place to work from
# instead of guessing CMake generator-specific default layouts.
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 QuickDialogs2 Network Svg Multimedia Test)

qt_standard_project_setup(REQUIRES 6.5)

enable_testing()

# Windows exe/taskbar icon. `.rc` files are a Windows-only resource-compiler
# input (MSVC's `rc.exe`), guarded here since qt_add_executable's SOURCES
# would otherwise pass it straight through to a non-Windows compiler on any
# future non-Windows build of this currently Windows-only project.
if(WIN32)
    set(LOREFORGE_CLIENT_WIN32_RESOURCES resources/app.rc)
endif()

qt_add_executable(LoreForgeClient
    src/main.cpp
    ${LOREFORGE_CLIENT_WIN32_RESOURCES}
)

# pragma Singleton in Theme.qml isn't enough on its own — qt_add_qml_module
# needs this source property to mark it as a singleton in the generated
# qmldir, otherwise `Theme.colorAccent` etc. silently resolve to undefined.
set_source_files_properties(qml/Theme.qml PROPERTIES
    QT_QML_SINGLETON_TYPE TRUE
)

qt_add_qml_module(LoreForgeClient
    URI LoreForge
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/Theme.qml
        qml/RepositoryCard.qml
        qml/PrimaryButton.qml
        qml/LoginScreen.qml
        qml/RepositoryListScreen.qml
        qml/RepositoryWorkspaceScreen.qml
        qml/CommitHistoryView.qml
    SOURCES
        src/ApiClient.h
        src/ApiClient.cpp
        src/AuthController.h
        src/AuthController.cpp
        src/RepositoryListModel.h
        src/RepositoryListModel.cpp
        src/RepositoryTreeModel.h
        src/RepositoryTreeModel.cpp
        src/CommitListModel.h
        src/CommitListModel.cpp
        src/LoreImageProvider.h
        src/LoreImageProvider.cpp
        src/AudioPlayerController.h
        src/AudioPlayerController.cpp
)

target_include_directories(LoreForgeClient
    PRIVATE
        src
)

target_link_libraries(LoreForgeClient
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::QuickControls2
        Qt6::QuickDialogs2
        Qt6::Network
        Qt6::Svg
        Qt6::Multimedia
)

set_target_properties(LoreForgeClient PROPERTIES
    WIN32_EXECUTABLE TRUE
)

# Locate windeployqt next to the qmake that find_package(Qt6 ...) resolved,
# rather than hardcoding this dev machine's Qt install path — this keeps the
# build portable to any machine with an equivalent CMAKE_PREFIX_PATH.
get_target_property(_lfc_qmake_executable Qt6::qmake IMPORTED_LOCATION)
get_filename_component(_lfc_qt_bin_dir "${_lfc_qmake_executable}" DIRECTORY)
find_program(WINDEPLOYQT_EXECUTABLE windeployqt HINTS "${_lfc_qt_bin_dir}")

if(WINDEPLOYQT_EXECUTABLE)
    # Copies the Qt DLLs, platform/image-format plugins, and QML modules that
    # LoreForgeClient needs at runtime into its own output directory, so the
    # packaged app (see install()/CPack below) doesn't depend on the Qt SDK
    # being present on the target machine.
    add_custom_command(TARGET LoreForgeClient POST_BUILD
        COMMAND "${WINDEPLOYQT_EXECUTABLE}"
            --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
            $<IF:$<CONFIG:Debug>,--debug,--release>
            --no-translations
            "$<TARGET_FILE:LoreForgeClient>"
        COMMENT "Deploying Qt runtime dependencies for LoreForgeClient via windeployqt"
        VERBATIM
    )
else()
    message(WARNING "windeployqt not found next to Qt6::qmake — packaged builds of LoreForgeClient will be missing required Qt runtime DLLs")
endif()

# Test target: compiles the pure-logic model/controller sources directly
# (skipping main.cpp) into a plain QTEST_GUILESS_MAIN executable rather than
# restructuring LoreForgeClient's qt_add_qml_module SOURCES into a shared
# library — simpler given this CMakeLists' current shape, and it can't
# accidentally pull test-only code (Qt6::Test) into the production target
# since it's an entirely separate target with its own source list.
add_executable(LoreForgeClientTests
    tests/tst_repositorytreemodel.cpp
    src/RepositoryTreeModel.h
    src/RepositoryTreeModel.cpp
    src/ApiClient.h
    src/ApiClient.cpp
)

target_include_directories(LoreForgeClientTests
    PRIVATE
        src
)

target_link_libraries(LoreForgeClientTests
    PRIVATE
        Qt6::Core
        Qt6::Network
        Qt6::Qml
        Qt6::Test
)

add_test(NAME RepositoryTreeModelTests COMMAND LoreForgeClientTests)

# ---------------------------------------------------------------------------
# Packaging (CPack)
# ---------------------------------------------------------------------------
# Installs LoreForgeClient.exe plus everything windeployqt (POST_BUILD, above)
# dropped next to it in CMAKE_RUNTIME_OUTPUT_DIRECTORY — Qt6*.dll, platforms/,
# imageformats/, qml/, etc. LoreForgeClientTests.exe lands in the same output
# directory (see CMAKE_RUNTIME_OUTPUT_DIRECTORY above) but is test-only and is
# explicitly excluded so it doesn't ship in the package.
install(TARGETS LoreForgeClient
    RUNTIME DESTINATION .
)

install(DIRECTORY "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/"
    DESTINATION .
    PATTERN "LoreForgeClientTests*" EXCLUDE
    PATTERN "*.pdb" EXCLUDE
    PATTERN "*.ilk" EXCLUDE
)

set(CPACK_PACKAGE_NAME "LoreForgeClient")
set(CPACK_PACKAGE_VERSION "0.1.0")
set(CPACK_PACKAGE_VENDOR "Nebula Studios")
# windeployqt's QML deployment includes every QtQuickControls2 style
# implementation (Basic/Fusion/Imagine/Material/Universal/Windows/
# FluentWinUI3/native) since the app doesn't pin one at compile time; some of
# those file paths (e.g. qml/QtQuick/Controls/FluentWinUI3/light/images/...)
# are long enough that the default CPACK_PACKAGE_FILE_NAME
# ("LoreForgeClient-0.1.0-win64") pushes the NSIS staging path over Windows'
# ~260-char MAX_PATH limit, which makes makensis fail with
# "File: failed opening file ...". Use a short package file name to keep
# headroom; CPACK_PACKAGE_NAME above still controls the installed
# folder/registry display name.
set(CPACK_PACKAGE_FILE_NAME "LFC-${CPACK_PACKAGE_VERSION}-win64")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "LoreForge Client - VCS desktop client")
set(CPACK_PACKAGE_EXECUTABLES "LoreForgeClient" "LoreForge Client")
set(CPACK_PACKAGE_INSTALL_DIRECTORY "LoreForgeClient")

# Generator choice: this dev environment has NSIS (makensis.exe) installed
# via winget, so we use it for a proper installer .exe with Start Menu
# shortcuts and an uninstaller. Machines without NSIS should override
# CPACK_GENERATOR to "ZIP" (e.g. -DCPACK_GENERATOR=ZIP) for a relocatable
# folder with zero extra dependencies.
find_program(MAKENSIS_EXECUTABLE makensis
    HINTS "C:/Program Files (x86)/NSIS" "C:/Program Files/NSIS"
)
if(MAKENSIS_EXECUTABLE)
    set(CPACK_GENERATOR "NSIS")
    set(CPACK_NSIS_DISPLAY_NAME "LoreForge Client")
    set(CPACK_NSIS_PACKAGE_NAME "LoreForge Client")
    set(CPACK_NSIS_ENABLE_UNINSTALL_BEFORE_INSTALL ON)
    set(CPACK_NSIS_MODIFY_PATH OFF)
    set(CPACK_NSIS_CREATE_ICONS_EXTRA
        "CreateShortCut '$SMPROGRAMS\\\\$STARTMENU_FOLDER\\\\LoreForge Client.lnk' '$INSTDIR\\\\LoreForgeClient.exe'"
    )
    set(CPACK_NSIS_DELETE_ICONS_EXTRA
        "Delete '$SMPROGRAMS\\\\$STARTMENU_FOLDER\\\\LoreForge Client.lnk'"
    )
else()
    message(STATUS "NSIS (makensis) not found — falling back to CPACK_GENERATOR=ZIP for LoreForgeClient")
    set(CPACK_GENERATOR "ZIP")
endif()

include(CPack)
