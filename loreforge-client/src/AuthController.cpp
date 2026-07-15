#include "AuthController.h"
#include "ApiClient.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>

AuthController::AuthController(QObject *parent)
    : QObject(parent)
{
}

void AuthController::login(const QString &email, const QString &password)
{
    setErrorMessage(QString());
    setBusy(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/auth/login"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject body;
    body["email"] = email;
    body["password"] = password;

    QNetworkReply *reply = ApiClient::networkManager().post(request, QJsonDocument(body).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        handleLoginReply(reply);
        reply->deleteLater();
    });
}

void AuthController::handleLoginReply(QNetworkReply *reply)
{
    setBusy(false);
    const QByteArray data = reply->readAll();

    if (reply->error() != QNetworkReply::NoError) {
        QString message = QStringLiteral("Login failed. Check your email and password.");
        const QJsonObject obj = QJsonDocument::fromJson(data).object();
        if (obj.contains("error"))
            message = obj.value("error").toString();
        setErrorMessage(message);
        return;
    }

    const QJsonObject obj = QJsonDocument::fromJson(data).object();
    const QJsonObject user = obj.value("user").toObject();

    m_currentUserName = user.value("name").toString();
    emit currentUserNameChanged();

    m_loggedIn = true;
    emit loggedInChanged();
}

void AuthController::logout()
{
    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/auth/logout"));
    QNetworkReply *reply = ApiClient::networkManager().post(request, QByteArray());
    connect(reply, &QNetworkReply::finished, this, [reply]() { reply->deleteLater(); });

    m_loggedIn = false;
    emit loggedInChanged();
    m_currentUserName.clear();
    emit currentUserNameChanged();
}

void AuthController::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void AuthController::setErrorMessage(const QString &message)
{
    if (m_errorMessage == message)
        return;
    m_errorMessage = message;
    emit errorMessageChanged();
}
