#pragma once

#include <QObject>
#include <QProcess>
#include <QQmlEngine>
#include <QString>

/**
 * Wraps the `docker` CLI via QProcess, mirroring ARCHITECTURE.md §2.2's
 * stated precedent for LoreForge Client ("QProcessでラップする") rather than
 * talking to the Docker daemon's HTTP API directly. Controls the MinIO
 * container that backs LoreHub's S3-compatible storage (see ARCHITECTURE.md
 * §3.3). There is no real Lore server binary in this project, so this class
 * intentionally has no Lore-server equivalent — that stays a UI placeholder.
 */
class DockerController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(bool dockerAvailable READ dockerAvailable NOTIFY dockerAvailableChanged)
    Q_PROPERTY(MinioStatus minioStatus READ minioStatus NOTIFY minioStatusChanged)
    Q_PROPERTY(QString minioStatusText READ minioStatusText NOTIFY minioStatusChanged)
    Q_PROPERTY(QString minioStatsLabel READ minioStatsLabel NOTIFY minioStatsLabelChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)

public:
    enum MinioStatus {
        NotInstalled,
        Stopped,
        Running,
    };
    Q_ENUM(MinioStatus)

    explicit DockerController(QObject *parent = nullptr);

    bool dockerAvailable() const { return m_dockerAvailable; }
    MinioStatus minioStatus() const { return m_minioStatus; }
    QString minioStatusText() const;
    QString minioStatsLabel() const { return m_minioStatsLabel; }
    QString lastError() const { return m_lastError; }
    bool busy() const { return m_busy; }

public slots:
    void checkDockerAvailable();
    void startMinio();
    void stopMinio();
    void refreshMinioStatus();

signals:
    void dockerAvailableChanged();
    void minioStatusChanged();
    void minioStatsLabelChanged();
    void lastErrorChanged();
    void busyChanged();
    void minioStarted();
    void minioStopped();

private:
    void setBusy(bool busy);
    void setDockerAvailable(bool available);
    void setMinioStatus(MinioStatus status);
    void setMinioStatsLabel(const QString &label);
    void setLastError(const QString &error);
    void refreshMinioStats();

    bool m_dockerAvailable = false;
    MinioStatus m_minioStatus = NotInstalled;
    QString m_minioStatsLabel;
    QString m_lastError;
    bool m_busy = false;
};
