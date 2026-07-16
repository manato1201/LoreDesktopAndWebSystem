#pragma once

#include <QNetworkAccessManager>
#include <QString>

/**
 * Process-wide QNetworkAccessManager, ported from loreforge-client's
 * ApiClient (src/ApiClient.h there) so PermissionConfigController can talk
 * to the real lorehub-api instead of only ever touching the local JSON
 * config file (ARCHITECTURE.md §3.3's "生成・適用する" — this is the
 * "適用" half). QNetworkAccessManager lazily owns a QNetworkCookieJar, and
 * the session cookie lorehub-api sets on login has to be replayed on the
 * later PUT /api/access-control/entries request — a single shared instance
 * is what makes that automatic.
 */
class ApiClient
{
public:
    static QNetworkAccessManager &networkManager();
    static QString baseUrl();
};
