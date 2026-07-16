#include "LoreImageProvider.h"
#include "ApiClient.h"

#include <QCoreApplication>
#include <QMetaObject>
#include <QNetworkRequest>
#include <QThread>
#include <cstdio>

namespace {

/**
 * image://lore/<slug>/<variant>/<path...> — QML strips the "image://lore/"
 * prefix before handing us `id`, so `id` is "<slug>/<variant>/<path...>".
 * `path` itself may contain further slashes, so only the first two
 * separators are structural.
 */
QUrl resolveEndpoint(const QString &id)
{
    const QString slug = id.section(QLatin1Char('/'), 0, 0);
    const QString variant = id.section(QLatin1Char('/'), 1, 1);
    const QString path = id.section(QLatin1Char('/'), 2);

    const QLatin1String segment = variant == QLatin1String("before")
        ? QLatin1String("image-before")
        : QLatin1String("image");

    return QUrl(ApiClient::baseUrl() + "/api/repositories/" + slug + "/" + segment + "/" + path);
}

} // namespace

LoreImageResponse::LoreImageResponse(QUrl url)
    : m_url(std::move(url))
{
}

void LoreImageResponse::start()
{
    QNetworkRequest request(m_url);
    m_reply = ApiClient::networkManager().get(request);
    connect(m_reply, &QNetworkReply::finished, this, &LoreImageResponse::handleFinished);
}

void LoreImageResponse::handleFinished()
{
    QNetworkReply *reply = m_reply;
    m_reply = nullptr;

    if (reply->error() != QNetworkReply::NoError) {
        m_errorString = reply->errorString();
        std::fprintf(stderr, "LoreImageProvider: fetch failed for %s (%s)\n",
                     qPrintable(m_url.toString()), qPrintable(m_errorString));
        std::fflush(stderr);
        reply->deleteLater();
        emit finished();
        return;
    }

    const QByteArray bytes = reply->readAll();
    const int httpStatus = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
    const bool loaded = m_image.loadFromData(bytes);
    std::fprintf(stderr, "LoreImageProvider: %s -> HTTP %d, %lld bytes, decoded=%s\n",
                 qPrintable(m_url.toString()), httpStatus, static_cast<qint64>(bytes.size()),
                 loaded ? "yes" : "no");
    std::fflush(stderr);

    if (!loaded) {
        m_errorString = QStringLiteral("could not decode image data");
        m_image = QImage();
    }

    reply->deleteLater();
    emit finished();
}

QQuickTextureFactory *LoreImageResponse::textureFactory() const
{
    if (m_image.isNull())
        return nullptr;
    return QQuickTextureFactory::textureFactoryForImage(m_image);
}

QQuickImageResponse *LoreImageProvider::requestImageResponse(const QString &id, const QSize &requestedSize)
{
    Q_UNUSED(requestedSize)

    auto *response = new LoreImageResponse(resolveEndpoint(id));

    // requestImageResponse() runs on one of QML's image-loading worker
    // threads, but ApiClient::networkManager() is affine to the GUI thread
    // (it's a plain function-local static first touched from there) — move
    // the response to the GUI thread and kick off the fetch via a queued
    // invoke so the actual QNetworkAccessManager::get() call happens on its
    // home thread. `finished()` still gets delivered back to QML's worker
    // thread automatically since Qt's signal/slot system queues across
    // threads based on the *receiver's* affinity.
    response->moveToThread(QCoreApplication::instance()->thread());
    QMetaObject::invokeMethod(response, "start", Qt::QueuedConnection);

    return response;
}
