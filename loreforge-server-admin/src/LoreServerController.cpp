#include "LoreServerController.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>

namespace {

constexpr const char *kListeningMarker = "lorehub-api listening on http://127.0.0.1:4000";

// tasklist's CSV output quotes every field but the "Mem Usage" column itself
// contains a thousands-separator comma (e.g. "12,345 K"), so a naive
// String::split(',') would corrupt it. This respects quotes.
QStringList parseCsvLine(const QString &line)
{
    QStringList fields;
    QString current;
    bool inQuotes = false;
    for (const QChar &c : line) {
        if (c == QLatin1Char('"')) {
            inQuotes = !inQuotes;
        } else if (c == QLatin1Char(',') && !inQuotes) {
            fields.append(current);
            current.clear();
        } else {
            current.append(c);
        }
    }
    fields.append(current);
    return fields;
}

} // namespace

LoreServerController::LoreServerController(QObject *parent)
    : QObject(parent)
{
    // Safety net: if the QML engine tears down the object graph without
    // running our destructor in time (e.g. abrupt app quit), still make
    // sure we don't orphan a running lorehub-api.exe.
    if (QCoreApplication *app = QCoreApplication::instance())
        connect(app, &QCoreApplication::aboutToQuit, this, &LoreServerController::killProcessNow);
}

LoreServerController::~LoreServerController()
{
    killProcessNow();
}

QString LoreServerController::statusText() const
{
    switch (m_status) {
    case Running:
        return QStringLiteral("Running");
    case Starting:
        return QStringLiteral("Starting…");
    case Error:
        return QStringLiteral("Error");
    case Stopped:
    default:
        return QStringLiteral("Stopped");
    }
}

void LoreServerController::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void LoreServerController::setStatus(ServerStatus status)
{
    if (m_status == status)
        return;
    m_status = status;
    emit statusChanged();
}

void LoreServerController::setPid(int pid)
{
    if (m_pid == pid)
        return;
    m_pid = pid;
    emit pidChanged();
}

void LoreServerController::setMemoryUsageLabel(const QString &label)
{
    if (m_memoryUsageLabel == label)
        return;
    m_memoryUsageLabel = label;
    emit memoryUsageLabelChanged();
}

void LoreServerController::setLastError(const QString &error)
{
    m_lastError = error;
    emit lastErrorChanged();
}

bool LoreServerController::resolvePaths()
{
    // Repo layout is fixed: loreforge-server-admin/ and lorehub-api/ are
    // siblings under the repo root. Walk up from wherever the Server Admin
    // executable actually landed (build/, build/Debug/, etc.) looking for a
    // sibling lorehub-api/ directory rather than hardcoding a fixed relative
    // depth.
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 8; ++i) {
        const QString candidateExe = dir.filePath(QStringLiteral("lorehub-api/target/debug/lorehub-api.exe"));
        if (QFileInfo::exists(candidateExe)) {
            m_exePath = candidateExe;
            m_workingDir = dir.filePath(QStringLiteral("lorehub-api"));
            return true;
        }
        if (!dir.cdUp())
            break;
    }

    setLastError(QStringLiteral(
        "lorehub-api.exe not found — build it first with `cargo build` in lorehub-api/"));
    return false;
}

void LoreServerController::startServer()
{
    if (m_status == Running || m_status == Starting)
        return;

    if (!resolvePaths()) {
        setStatus(Error);
        return;
    }

    setLastError(QString());
    setBusy(true);
    setStatus(Starting);
    m_startupConfirmed = false;
    m_pendingStartupOutput.clear();

    m_process = new QProcess(this);
    m_process->setWorkingDirectory(m_workingDir);
    m_process->setProcessChannelMode(QProcess::SeparateChannels);

    connect(m_process, &QProcess::readyReadStandardOutput, this,
            &LoreServerController::handleOutputForReadySignal);
    connect(m_process, &QProcess::readyReadStandardError, this,
            &LoreServerController::handleOutputForReadySignal);

    connect(m_process, &QProcess::started, this, [this]() {
        if (m_process)
            setPid(int(m_process->processId()));
    });

    connect(m_process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            [this](int exitCode, QProcess::ExitStatus) {
                if (m_startupTimer)
                    m_startupTimer->stop();

                const bool wasStarting = (m_status == Starting) && !m_startupConfirmed;
                setBusy(false);
                setPid(0);
                setMemoryUsageLabel(QString());

                if (wasStarting) {
                    setLastError(QStringLiteral(
                        "lorehub-api exited before it finished starting (exit code %1). "
                        "Check that lorehub.db isn't locked by another instance.")
                                      .arg(exitCode));
                    setStatus(Error);
                } else {
                    // A deliberate stopServer() ends up here even when the
                    // child was killed rather than gracefully terminated
                    // (see the errorOccurred comment below) — clear any
                    // transient error text so a normal Stop doesn't leave a
                    // stale "could not launch" message on screen.
                    setLastError(QString());
                    setStatus(Stopped);
                }

                emit serverStopped();

                if (m_process) {
                    m_process->deleteLater();
                    m_process = nullptr;
                }
            });

    connect(m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        // Only a genuine launch failure is handled here. lorehub-api.exe is
        // a console app with no window, so QProcess::terminate() (which
        // posts WM_CLOSE to the process's top-level windows on Windows) is
        // a no-op for it in practice — stopServer()'s kill() fallback is
        // what actually ends it, and Qt reports that as QProcess::Crashed.
        // finished() (connected above) is the authoritative handler for
        // that terminal state; treating Crashed as an Error here would
        // flash a misleading "could not launch" message on every normal
        // Stop.
        if (error != QProcess::FailedToStart)
            return;
        const QString err = m_process ? m_process->errorString() : QString();
        setLastError(QStringLiteral("Could not launch lorehub-api.exe: %1").arg(err));
        setStatus(Error);
        setBusy(false);
    });

    m_process->start(m_exePath, {});

    if (!m_startupTimer) {
        m_startupTimer = new QTimer(this);
        m_startupTimer->setSingleShot(true);
    } else {
        m_startupTimer->disconnect();
    }
    connect(m_startupTimer, &QTimer::timeout, this, [this]() {
        if (m_status == Starting && !m_startupConfirmed) {
            setLastError(QStringLiteral(
                "Timed out waiting for lorehub-api to report it is listening on port 4000."));
            setStatus(Error);
            setBusy(false);
            killProcessNow();
        }
    });
    m_startupTimer->start(10000);
}

void LoreServerController::handleOutputForReadySignal()
{
    if (!m_process)
        return;

    // Always drain both channels, even after startup is confirmed: the
    // server logs every HTTP request via tower_http::TraceLayer, and an
    // unread QProcess buffer grows without bound for the lifetime of a
    // long-running process if nobody reads it.
    const QByteArray out = m_process->readAllStandardOutput();
    const QByteArray err = m_process->readAllStandardError();

    if (m_startupConfirmed)
        return;

    // Accumulate across reads in case the log line straddles two chunks.
    m_pendingStartupOutput += out;
    m_pendingStartupOutput += err;
    if (!m_pendingStartupOutput.contains(kListeningMarker))
        return;

    m_pendingStartupOutput.clear();
    m_startupConfirmed = true;
    if (m_startupTimer)
        m_startupTimer->stop();

    setPid(m_process ? int(m_process->processId()) : 0);
    setStatus(Running);
    setBusy(false);
    setLastError(QString());
    emit serverStarted();
    refreshStatus();
}

void LoreServerController::stopServer()
{
    if (!m_process || m_status == Stopped)
        return;

    setBusy(true);
    m_process->terminate();

    QTimer::singleShot(3000, this, [this]() {
        if (m_process && m_process->state() != QProcess::NotRunning)
            m_process->kill();
    });
}

void LoreServerController::refreshStatus()
{
    if (m_status != Running || !m_process || m_pid == 0) {
        setMemoryUsageLabel(QString());
        return;
    }

    const int targetPid = m_pid;
    auto *query = new QProcess(this);
    connect(query, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            [this, query, targetPid](int, QProcess::ExitStatus) {
                const QString output = QString::fromLocal8Bit(query->readAllStandardOutput());
                query->deleteLater();

                // The process may have been stopped/restarted while the
                // tasklist query was in flight — ignore a stale result.
                if (m_pid != targetPid)
                    return;

                for (const QString &line : output.split(QStringLiteral("\n"), Qt::SkipEmptyParts)) {
                    const QString trimmedLine = line.trimmed();
                    if (trimmedLine.isEmpty())
                        continue;

                    const QStringList fields = parseCsvLine(trimmedLine);
                    if (fields.size() < 5)
                        continue;

                    QString mem = fields.at(4);
                    mem.remove(QLatin1Char('"'));
                    mem.remove(QStringLiteral(" K"));
                    mem.remove(QLatin1Char(','));
                    bool ok = false;
                    const qint64 kilobytes = mem.trimmed().toLongLong(&ok);
                    if (ok) {
                        const double megabytes = kilobytes / 1024.0;
                        setMemoryUsageLabel(QStringLiteral("%1 MB").arg(megabytes, 0, 'f', 1));
                    } else {
                        setMemoryUsageLabel(QString());
                    }
                    return;
                }

                setMemoryUsageLabel(QString());
            });
    connect(query, &QProcess::errorOccurred, this, [this, query](QProcess::ProcessError) {
        setMemoryUsageLabel(QString());
        query->deleteLater();
    });

    query->start(QStringLiteral("tasklist"),
                 { QStringLiteral("/fi"), QStringLiteral("PID eq %1").arg(targetPid),
                   QStringLiteral("/fo"), QStringLiteral("csv"),
                   QStringLiteral("/nh") });
}

void LoreServerController::killProcessNow()
{
    // Capture the pointer locally: waitForFinished() below can pump the
    // event loop and synchronously invoke our own `finished` handler (see
    // startServer()), which sets the m_process *member* to nullptr via
    // deleteLater(). Re-reading the member after that would dereference a
    // null pointer, so every subsequent call in this function goes through
    // the local copy instead.
    QProcess *process = m_process;
    if (!process)
        return;

    if (process->state() != QProcess::NotRunning) {
        process->terminate();
        if (!process->waitForFinished(3000))
            process->kill();
        process->waitForFinished(1000);
    }
}
