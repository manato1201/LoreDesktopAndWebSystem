#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QVariantList>

/**
 * Serializes the Access Control node-editor graph (ARCHITECTURE.md §3.3:
 * "ノードエディタ風のUIで設定し、裏側でLoreのサーバー設定ファイル（JSON等）を
 * 生成・適用する") to a JSON file and reloads it on startup so the graph
 * (connections + node positions) survives an app relaunch. QML hands over
 * two flat QVariantLists built from its existing nodeRegistry/connections
 * state — this class owns the on-disk schema and the directory/role
 * reconstruction, QML owns the canvas interaction untouched.
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

public:
    explicit PermissionConfigController(QObject *parent = nullptr);

    QVariantList directoryNodes() const { return m_directoryNodes; }
    QVariantList roleNodes() const { return m_roleNodes; }
    QVariantList connections() const { return m_connections; }
    bool hasSavedConfig() const { return m_hasSavedConfig; }
    QString lastError() const { return m_lastError; }
    QString configPath() const { return m_configPath; }

public slots:
    bool saveConfig(const QVariantList &nodes, const QVariantList &connections);
    bool loadConfig();

signals:
    void configLoaded();
    void configSaved();
    void lastErrorChanged();

private:
    void setLastError(const QString &error);

    QString m_configPath;
    QVariantList m_directoryNodes;
    QVariantList m_roleNodes;
    QVariantList m_connections;
    bool m_hasSavedConfig = false;
    QString m_lastError;
};
