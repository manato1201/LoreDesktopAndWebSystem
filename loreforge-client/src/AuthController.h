#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>

class QNetworkReply;

/**
 * Wraps POST /api/auth/login and /api/auth/logout. Session state lives
 * server-side behind the `lorehub_token` cookie, which ApiClient's shared
 * QNetworkAccessManager stores and replays automatically — this class only
 * tracks the UI-facing loggedIn/currentUserName/errorMessage state.
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
    void setBusy(bool busy);
    void setErrorMessage(const QString &message);
    void handleLoginReply(QNetworkReply *reply);

    bool m_loggedIn = false;
    bool m_busy = false;
    QString m_currentUserName;
    QString m_errorMessage;
};
