#pragma once

#include <QQuickAsyncImageProvider>
#include <QQuickImageResponse>
#include <QQuickTextureFactory>
#include <QImage>
#include <QNetworkReply>
#include <QString>
#include <QUrl>

/**
 * Serves `image://lore/<slug>/<variant>/<path>` URLs (variant is "current"
 * or "before") by fetching bytes from lorehub-api's authenticated image
 * endpoints through ApiClient::networkManager() — the shared, cookie-jarred
 * QNetworkAccessManager every network-touching class in this app must use,
 * since the session cookie set on login lives only in that jar. QML's plain
 * `Image { source: "http://..." }` element uses Qt's *default* network
 * access manager instead, which does not carry that cookie and would 401.
 *
 * Async (QQuickAsyncImageProvider + QQuickImageResponse) so the network
 * round-trip never blocks the GUI thread.
 */
class LoreImageResponse : public QQuickImageResponse
{
    Q_OBJECT

public:
    explicit LoreImageResponse(QUrl url);

    QQuickTextureFactory *textureFactory() const override;
    QString errorString() const override { return m_errorString; }

public slots:
    /** Must run on the same thread as ApiClient::networkManager() (the GUI
     *  thread) — requestImageResponse() below moves this object there
     *  before invoking this slot via a queued connection. */
    void start();

private slots:
    void handleFinished();

private:
    QUrl m_url;
    QImage m_image;
    QString m_errorString;
    QNetworkReply *m_reply = nullptr;
};

class LoreImageProvider : public QQuickAsyncImageProvider
{
public:
    QQuickImageResponse *requestImageResponse(const QString &id, const QSize &requestedSize) override;
};
