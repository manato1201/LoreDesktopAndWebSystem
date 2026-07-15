#include "DockerController.h"

namespace {
constexpr const char *kMinioContainerName = "lorehub-minio";
}

DockerController::DockerController(QObject *parent)
    : QObject(parent)
{
}

QString DockerController::minioStatusText() const
{
    switch (m_minioStatus) {
    case Running:
        return QStringLiteral("Running");
    case Stopped:
        return QStringLiteral("Stopped");
    case NotInstalled:
    default:
        return QStringLiteral("Not created");
    }
}

void DockerController::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void DockerController::setDockerAvailable(bool available)
{
    if (m_dockerAvailable == available)
        return;
    m_dockerAvailable = available;
    emit dockerAvailableChanged();
}

void DockerController::setMinioStatus(MinioStatus status)
{
    if (m_minioStatus == status)
        return;
    m_minioStatus = status;
    emit minioStatusChanged();
}

void DockerController::setLastError(const QString &error)
{
    m_lastError = error;
    emit lastErrorChanged();
}

void DockerController::checkDockerAvailable()
{
    setBusy(true);

    auto *process = new QProcess(this);
    connect(process, &QProcess::finished, this,
            [this, process](int exitCode, QProcess::ExitStatus) {
                setDockerAvailable(exitCode == 0);
                if (exitCode != 0)
                    setLastError(QStringLiteral("Docker is not available on this machine."));
                else
                    setLastError(QString());
                setBusy(false);
                refreshMinioStatus();
                process->deleteLater();
            });
    connect(process, &QProcess::errorOccurred, this,
            [this, process](QProcess::ProcessError) {
                setDockerAvailable(false);
                setLastError(QStringLiteral("Could not launch the docker CLI. Is Docker installed?"));
                setBusy(false);
                setMinioStatus(NotInstalled);
                process->deleteLater();
            });

    process->start(QStringLiteral("docker"), { QStringLiteral("info") });
}

void DockerController::refreshMinioStatus()
{
    if (!m_dockerAvailable) {
        setMinioStatus(NotInstalled);
        return;
    }

    auto *process = new QProcess(this);
    connect(process, &QProcess::finished, this,
            [this, process](int exitCode, QProcess::ExitStatus) {
                const QString output = QString::fromUtf8(process->readAllStandardOutput()).trimmed();
                if (exitCode == 0 && !output.isEmpty())
                    setMinioStatus(Running);
                else
                    setMinioStatus(Stopped);
                process->deleteLater();
            });
    connect(process, &QProcess::errorOccurred, this,
            [this, process](QProcess::ProcessError) {
                setMinioStatus(NotInstalled);
                process->deleteLater();
            });

    process->start(QStringLiteral("docker"),
                    { QStringLiteral("ps"),
                      QStringLiteral("--filter"), QStringLiteral("name=%1").arg(kMinioContainerName),
                      QStringLiteral("--filter"), QStringLiteral("status=running"),
                      QStringLiteral("--format"), QStringLiteral("{{.Status}}") });
}

void DockerController::startMinio()
{
    if (!m_dockerAvailable)
        return;

    setBusy(true);

    auto *process = new QProcess(this);
    connect(process, &QProcess::finished, this,
            [this, process](int exitCode, QProcess::ExitStatus) {
                setBusy(false);
                if (exitCode == 0) {
                    setLastError(QString());
                    emit minioStarted();
                } else {
                    const QString err = QString::fromUtf8(process->readAllStandardError()).trimmed();
                    setLastError(err.isEmpty()
                                      ? QStringLiteral("Failed to start the MinIO container.")
                                      : err);
                }
                process->deleteLater();
                refreshMinioStatus();
            });
    connect(process, &QProcess::errorOccurred, this,
            [this, process](QProcess::ProcessError) {
                setBusy(false);
                setLastError(QStringLiteral("Could not launch the docker CLI."));
                process->deleteLater();
            });

    process->start(QStringLiteral("docker"),
                    { QStringLiteral("run"), QStringLiteral("-d"),
                      QStringLiteral("--name"), QLatin1String(kMinioContainerName),
                      QStringLiteral("-p"), QStringLiteral("9000:9000"),
                      QStringLiteral("-p"), QStringLiteral("9001:9001"),
                      QStringLiteral("minio/minio"),
                      QStringLiteral("server"), QStringLiteral("/data"),
                      QStringLiteral("--console-address"), QStringLiteral(":9001") });
}

void DockerController::stopMinio()
{
    if (!m_dockerAvailable)
        return;

    setBusy(true);

    auto *process = new QProcess(this);
    connect(process, &QProcess::finished, this,
            [this, process](int exitCode, QProcess::ExitStatus) {
                setBusy(false);
                if (exitCode == 0) {
                    setLastError(QString());
                    emit minioStopped();
                } else {
                    const QString err = QString::fromUtf8(process->readAllStandardError()).trimmed();
                    setLastError(err.isEmpty()
                                      ? QStringLiteral("Failed to stop the MinIO container.")
                                      : err);
                }
                process->deleteLater();
                refreshMinioStatus();
            });
    connect(process, &QProcess::errorOccurred, this,
            [this, process](QProcess::ProcessError) {
                setBusy(false);
                setLastError(QStringLiteral("Could not launch the docker CLI."));
                process->deleteLater();
            });

    process->start(QStringLiteral("docker"),
                    { QStringLiteral("stop"), QLatin1String(kMinioContainerName) });
}
