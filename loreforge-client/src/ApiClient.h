#pragma once

#include <QNetworkAccessManager>
#include <QString>

/**
 * Process-wide QNetworkAccessManager. Every controller (AuthController,
 * RepositoryListModel, RepositoryTreeModel, ...) must go through this
 * shared instance rather than owning its own QNetworkAccessManager —
 * QNetworkAccessManager owns a QNetworkCookieJar, and the session cookie
 * lorehub-api sets on login has to be replayed on every later request.
 * Separate managers would mean separate (empty) cookie jars.
 */
class ApiClient
{
public:
    static QNetworkAccessManager &networkManager();
    static QString baseUrl();
};
