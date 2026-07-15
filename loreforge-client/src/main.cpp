#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <cstdio>

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName("LoreForge Client");
    app.setOrganizationName("Nebula Studios");

    QQmlApplicationEngine engine;
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
