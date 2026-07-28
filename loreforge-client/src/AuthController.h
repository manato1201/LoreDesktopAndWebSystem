#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QTimer>

class QNetworkReply;

/**
 * Wraps POST /api/auth/login and /api/auth/logout. Session state lives
 * server-side behind the `lorehub_token` cookie, which ApiClient's shared
 * QNetworkAccessManager stores and replays automatically — this class only
 * tracks the UI-facing loggedIn/currentUserName/errorMessage state.
 *
 * Also owns the session's proactive keep-alive: `lorehub_token` expires
 * ACCESS_TOKEN_TTL_SECS (30 min, lorehub-api/src/auth.rs) after issuance, so
 * once logged in, a QTimer periodically POSTs /api/auth/refresh through the
 * same shared ApiClient::networkManager() — the refresh cookie is already
 * sitting in its cookie jar from login, so nothing extra needs to be sent.
 * This mirrors lorehub-web's SSR proactive-refresh half (docs/TECHNICAL_REFERENCE.md
 * §4); unlike lorehub-web's CSR half there is no retry-on-401-and-replay here,
 * since Client's requests aren't funneled through one choke point the way
 * fetchWithRefresh is.
 */
class AuthController : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(bool loggedIn READ loggedIn NOTIFY loggedInChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString currentUserName READ currentUserName NOTIFY currentUserNameChanged)
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY errorMessageChanged)

public:
    explicit AuthController(QObject *parent = nullptr);

    bool loggedIn() const { return m_loggedIn; }
    bool busy() const { return m_busy; }
    QString currentUserName() const { return m_currentUserName; }
    QString errorMessage() const { return m_errorMessage; }

    Q_INVOKABLE void login(const QString &email, const QString &password);
    Q_INVOKABLE void logout();

signals:
    void loggedInChanged();
    void busyChanged();
    void currentUserNameChanged();
    void errorMessageChanged();

private:
    // How often the keep-alive timer fires while logged in. Well under the
    // 30-minute ACCESS_TOKEN_TTL_SECS server-side TTL so the access token is
    // always refreshed with a safety margin to spare, never right at the
    // boundary.
    static constexpr int kRefreshIntervalMs = 25 * 60 * 1000;

    void setBusy(bool busy);
    void setErrorMessage(const QString &message);
    void handleLoginReply(QNetworkReply *reply);
    void setLoggedOut();
    void refreshSession();

    bool m_loggedIn = false;
    bool m_busy = false;
    QString m_currentUserName;
    QString m_errorMessage;
    QTimer m_refreshTimer;
};
