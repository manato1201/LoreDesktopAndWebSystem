#include "AudioPlayerController.h"
#include "ApiClient.h"

#include <QAudioOutput>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>
#include <cstdio>

AudioPlayerController::AudioPlayerController(QObject *parent)
    : QObject(parent)
    , m_player(new QMediaPlayer(this))
    , m_audioOutput(new QAudioOutput(this))
{
    m_player->setAudioOutput(m_audioOutput);

    connect(m_player, &QMediaPlayer::playingChanged, this, &AudioPlayerController::playingChanged);
    connect(m_player, &QMediaPlayer::positionChanged, this, &AudioPlayerController::positionChanged);
    connect(m_player, &QMediaPlayer::durationChanged, this, &AudioPlayerController::durationChanged);
    connect(m_player, &QMediaPlayer::errorOccurred, this,
            [this](QMediaPlayer::Error error, const QString &errorString) {
                if (error == QMediaPlayer::NoError)
                    return;
                setErrorMessage(errorString);
                std::fprintf(stderr, "AudioPlayerController: QMediaPlayer error: %s\n",
                             qPrintable(errorString));
                std::fflush(stderr);
            });
    connect(m_player, &QMediaPlayer::mediaStatusChanged, this,
            [](QMediaPlayer::MediaStatus status) {
                std::fprintf(stderr, "AudioPlayerController: mediaStatusChanged -> %d\n",
                             static_cast<int>(status));
                std::fflush(stderr);
            });
}

bool AudioPlayerController::playing() const
{
    return m_player->isPlaying();
}

qint64 AudioPlayerController::position() const
{
    return m_player->position();
}

qint64 AudioPlayerController::duration() const
{
    return m_player->duration();
}

void AudioPlayerController::load(const QString &slug, const QString &path)
{
    if (slug.isEmpty() || path.isEmpty())
        return;

    m_player->stop();
    setErrorMessage(QString());
    setLoading(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + slug + "/audio/" + path));
    QNetworkReply *reply = ApiClient::networkManager().get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply, path]() {
        setLoading(false);

        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            std::fprintf(stderr, "AudioPlayerController: fetch failed for %s (%s)\n",
                         qPrintable(path), qPrintable(reply->errorString()));
            std::fflush(stderr);
            reply->deleteLater();
            return;
        }

        const QByteArray bytes = reply->readAll();
        const int httpStatus = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
        reply->deleteLater();

        if (httpStatus != 200 || bytes.isEmpty()) {
            setErrorMessage(QStringLiteral("failed to load audio (HTTP %1)").arg(httpStatus));
            return;
        }

        std::fprintf(stderr, "AudioPlayerController: fetched audio for %s -> %lld bytes\n",
                     qPrintable(path), static_cast<qint64>(bytes.size()));
        std::fflush(stderr);

        // Replace the previous buffer (if any) — it must stay alive as a
        // member for as long as the player holds it as its source device,
        // so a local QBuffer here would leave a dangling QIODevice* once
        // this lambda returns.
        if (m_buffer) {
            m_buffer->close();
            delete m_buffer;
            m_buffer = nullptr;
        }

        m_audioBytes = bytes;
        m_buffer = new QBuffer(&m_audioBytes, this);
        m_buffer->open(QIODevice::ReadOnly);

        m_player->setSourceDevice(m_buffer, QUrl(QStringLiteral("audio-preview.wav")));
    });
}

void AudioPlayerController::play()
{
    m_player->play();
}

void AudioPlayerController::pause()
{
    m_player->pause();
}

void AudioPlayerController::seek(qint64 positionMs)
{
    m_player->setPosition(positionMs);
}

void AudioPlayerController::setLoading(bool loading)
{
    if (m_loading == loading)
        return;
    m_loading = loading;
    emit loadingChanged();
}

void AudioPlayerController::setErrorMessage(const QString &message)
{
    if (m_errorMessage == message)
        return;
    m_errorMessage = message;
    emit errorMessageChanged();
}
