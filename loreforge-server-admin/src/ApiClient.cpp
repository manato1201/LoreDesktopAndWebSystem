#include "ApiClient.h"

QNetworkAccessManager &ApiClient::networkManager()
{
    static QNetworkAccessManager manager;
    return manager;
}

QString ApiClient::baseUrl()
{
    return QStringLiteral("http://localhost:4000");
}
