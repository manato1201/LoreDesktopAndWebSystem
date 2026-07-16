#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QVariantList>

class QJsonArray;
class QNetworkReply;

/**
 * Serializes the Access Control node-editor graph (ARCHITECTURE.md §3.3:
 * "ノードエディタ風のUIで設定し、裏側でLoreのサーバー設定ファイル（JSON等）を
 * 生成・適用する") to a JSON file and reloads it on startup so the graph
 * (connections + node positions) survives an app relaunch. QML hands over
 * two flat QVariantLists built from its existing nodeRegistry/connections
 * state — this class owns the on-disk schema and the directory/role
 * reconstruction, QML owns the canvas interaction untouched.
 *
 * This is also where the "適用する" (apply) half of §3.3 lives: login()
 * and applyToServer() push the same directories/grants shape used by
 * saveConfig() to the real lorehub-api over HTTP, so LoreForge Server
 * Admin's graph and lorehub-api's live AppState.access_entries stay in
 * sync instead of the graph only ever reaching a local file.
 */
class PermissionConfigController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QVariantList directoryNodes READ directoryNodes NOTIFY configLoaded)
    Q_PROPERTY(QVariantList roleNodes READ roleNodes NOTIFY configLoaded)
    Q_PROPERTY(QVariantList connections READ connections NOTIFY configLoaded)
    Q_PROPERTY(bool hasSavedConfig READ hasSavedConfig NOTIFY configLoaded)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
    Q_PROPERTY(QString configPath READ configPath CONSTANT)

    Q_PROPERTY(bool connected READ connected NOTIFY connectedChanged)
    Q_PROPERTY(QString connectedAs READ connectedAs NOTIFY connectedChanged)
    Q_PROPERTY(bool applying READ applying NOTIFY applyingChanged)
    Q_PROPERTY(QString applyError READ applyError NOTIFY applyErrorChanged)
    Q_PROPERTY(QString applySuccess READ applySuccess NOTIFY applySuccessChanged)

public:
    explicit PermissionConfigController(QObject *parent = nullptr);

    QVariantList directoryNodes() const { return m_directoryNodes; }
    QVariantList roleNodes() const { return m_roleNodes; }
    QVariantList connections() const { return m_connections; }
    bool hasSavedConfig() const { return m_hasSavedConfig; }
    QString lastError() const { return m_lastError; }
    QString configPath() const { return m_configPath; }

    bool connected() const { return m_connected; }
    QString connectedAs() const { return m_connectedAs; }
    bool applying() const { return m_applying; }
    QString applyError() const { return m_applyError; }
    QString applySuccess() const { return m_applySuccess; }

public slots:
    bool saveConfig(const QVariantList &nodes, const QVariantList &connections);
    bool loadConfig();

    void login(const QString &email, const QString &password);
    void applyToServer();

signals:
    void configLoaded();
    void configSaved();
    void lastErrorChanged();

    void connectedChanged();
    void applyingChanged();
    void applyErrorChanged();
    void applySuccessChanged();

private:
    void setLastError(const QString &error);
    void setApplying(bool applying);
    void setApplyError(const QString &error);
    void setApplySuccess(const QString &message);

    // Shared by saveConfig() and applyToServer(): turns the combined
    // (directory + role, tagged by "type") node list and the from/to
    // connection list into the "directories" array where each directory
    // carries its resolved grants (roleId/principal/permissions). Also
    // hands back the flat "roles" array saveConfig() persists alongside it.
    void buildGraphJson(const QVariantList &nodes, const QVariantList &connections,
                         QJsonArray &directoriesArray, QJsonArray &rolesArray) const;

    QString m_configPath;
    QVariantList m_directoryNodes;
    QVariantList m_roleNodes;
    QVariantList m_connections;
    bool m_hasSavedConfig = false;
    QString m_lastError;

    bool m_connected = false;
    QString m_connectedAs;
    bool m_applying = false;
    QString m_applyError;
    QString m_applySuccess;
};
