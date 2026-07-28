#pragma once

#include <QBuffer>
#include <QByteArray>
#include <QMediaPlayer>
#include <QObject>
#include <QQmlEngine>
#include <QString>

class QAudioOutput;

/**
 * Audio preview playback for the detail panel's `kind === "audio"` branch
 * (see RepositoryWorkspaceScreen.qml). Mirrors LoreImageProvider's reasoning
 * for why a plain declarative source can't be used directly: lorehub-api's
 * GET .../audio/{path} endpoint is authenticated via the session cookie that
 * only ApiClient::networkManager()'s cookie jar carries, and QMediaPlayer's
 * own URL-based source loading would use Qt's *default* network manager
 * instead (a 401). Unlike the image provider, though, playback here is
 * imperative rather than declarative (Image { source: }), so the fix is
 * simpler: fetch the WAV bytes into a QByteArray via the shared network
 * manager first, then feed them to QMediaPlayer::setSourceDevice() through a
 * QBuffer wrapping that byte array. The buffer is kept alive as a member for
 * the player's lifetime — a QBuffer that goes out of scope after the fetch
 * would leave the player holding a dangling QIODevice*.
 *
 * load() is a plain Q_INVOKABLE called from QML's GUI-thread event loop, so
 * (unlike LoreImageProvider's requestImageResponse(), which runs on a QML
 * image-loading worker thread) no QMetaObject::invokeMethod thread marshaling
 * is needed here — the network call already happens on
 * ApiClient::networkManager()'s home (GUI) thread.
 */
class AudioPlayerController : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(bool playing READ playing NOTIFY playingChanged)
    Q_PROPERTY(qint64 position READ position NOTIFY positionChanged)
    Q_PROPERTY(qint64 duration READ duration NOTIFY durationChanged)
    Q_PROPERTY(bool loading READ loading NOTIFY loadingChanged)
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY errorMessageChanged)

public:
    explicit AudioPlayerController(QObject *parent = nullptr);

    bool playing() const;
    qint64 position() const;
    qint64 duration() const;
    bool loading() const { return m_loading; }
    QString errorMessage() const { return m_errorMessage; }

    /** Fetches GET /api/repositories/{slug}/audio/{path} and loads it. */
    Q_INVOKABLE void load(const QString &slug, const QString &path);
    Q_INVOKABLE void play();
    Q_INVOKABLE void pause();
    Q_INVOKABLE void seek(qint64 positionMs);

signals:
    void playingChanged();
    void positionChanged();
    void durationChanged();
    void loadingChanged();
    void errorMessageChanged();

private:
    void setLoading(bool loading);
    void setErrorMessage(const QString &message);

    QMediaPlayer *m_player = nullptr;
    QAudioOutput *m_audioOutput = nullptr;
    QBuffer *m_buffer = nullptr;
    QByteArray m_audioBytes;
    bool m_loading = false;
    QString m_errorMessage;
};
