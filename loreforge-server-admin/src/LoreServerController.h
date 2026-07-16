#pragma once

#include <QObject>
#include <QProcess>
#include <QQmlEngine>
#include <QString>
#include <QTimer>

/**
 * Controls the real `lorehub-api.exe` process (the Rust/Axum backend that
 * LoreHub Web and LoreForge Client already talk to over HTTP) via QProcess.
 * ARCHITECTURE.md §3.3 states the Lore Server should be controllable
 * "Dockerコンテナまたはローカルプロセスとして" (as a Docker container OR a
 * local process) — this class implements the local-process path, mirroring
 * DockerController's QProcess-wrapping shape and conventions.
 */
class LoreServerController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(ServerStatus status READ status NOTIFY statusChanged)
    Q_PROPERTY(QString statusText READ statusText NOTIFY statusChanged)
    Q_PROPERTY(int pid READ pid NOTIFY pidChanged)
    Q_PROPERTY(QString memoryUsageLabel READ memoryUsageLabel NOTIFY memoryUsageLabelChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)

public:
    enum ServerStatus {
        Stopped,
        Starting,
        Running,
        Error,
    };
    Q_ENUM(ServerStatus)

    explicit LoreServerController(QObject *parent = nullptr);
    ~LoreServerController() override;

    ServerStatus status() const { return m_status; }
    QString statusText() const;
    int pid() const { return m_pid; }
    QString memoryUsageLabel() const { return m_memoryUsageLabel; }
    QString lastError() const { return m_lastError; }
    bool busy() const { return m_busy; }

public slots:
    void startServer();
    void stopServer();
    void refreshStatus();

signals:
    void statusChanged();
    void pidChanged();
    void memoryUsageLabelChanged();
    void lastErrorChanged();
    void busyChanged();
    void serverStarted();
    void serverStopped();

private:
    void setBusy(bool busy);
    void setStatus(ServerStatus status);
    void setPid(int pid);
    void setMemoryUsageLabel(const QString &label);
    void setLastError(const QString &error);

    bool resolvePaths();
    void handleOutputForReadySignal();
    void killProcessNow();

    QProcess *m_process = nullptr;
    QTimer *m_startupTimer = nullptr;
    QString m_exePath;
    QString m_workingDir;
    ServerStatus m_status = Stopped;
    int m_pid = 0;
    QString m_memoryUsageLabel;
    QString m_lastError;
    bool m_busy = false;
    bool m_startupConfirmed = false;
    QByteArray m_pendingStartupOutput;
};
