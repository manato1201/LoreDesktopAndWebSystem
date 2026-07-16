#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <cstdio>

#include "LoreImageProvider.h"

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName("LoreForge Client");
    app.setOrganizationName("Nebula Studios");

    QQmlApplicationEngine engine;
    // Registers the image://lore/<slug>/<current|before>/<path> scheme used
    // by the Binary Diff Viewer's image slider (see LoreImageProvider.h).
    // Ownership transfers to the engine.
    engine.addImageProvider(QLatin1String("lore"), new LoreImageProvider);
    QObject::connect(
        &engine, &QQmlApplicationEngine::warnings, &app,
        [](const QList<QQmlError> &warnings) {
            for (const QQmlError &warning : warnings)
                std::fprintf(stderr, "QML warning: %s\n", qPrintable(warning.toString()));
        });
    QObject::connect(
        &engine, &QQmlApplicationEngine::objectCreationFailed, &app,
        []() { QCoreApplication::exit(-1); }, Qt::QueuedConnection);
    engine.loadFromModule("LoreForge", "Main");

    return app.exec();
}
