#include "PermissionConfigController.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QVariantMap>

namespace {

QStringList permissionsFromLabel(const QString &label)
{
    QStringList result;
    for (const QString &part : label.split(QLatin1Char('/'), Qt::SkipEmptyParts)) {
        const QString trimmed = part.trimmed().toLower();
        if (!trimmed.isEmpty())
            result.append(trimmed);
    }
    return result;
}

QString labelFromPermissions(const QJsonArray &permissions)
{
    QStringList parts;
    for (const QJsonValue &value : permissions) {
        QString word = value.toString();
        if (word.isEmpty())
            continue;
        word[0] = word[0].toUpper();
        parts.append(word);
    }
    return parts.join(QStringLiteral(" / "));
}

} // namespace

PermissionConfigController::PermissionConfigController(QObject *parent)
    : QObject(parent)
{
    m_configPath = QCoreApplication::applicationDirPath()
        + QStringLiteral("/config/access-control.json");
    loadConfig();
}

void PermissionConfigController::setLastError(const QString &error)
{
    m_lastError = error;
    emit lastErrorChanged();
}

bool PermissionConfigController::saveConfig(const QVariantList &nodes, const QVariantList &connections)
{
    QHash<QString, QJsonObject> directoryById;
    QHash<QString, QJsonObject> roleById;
    QStringList directoryOrder;
    QStringList roleOrder;

    for (const QVariant &nodeVariant : nodes) {
        const QVariantMap node = nodeVariant.toMap();
        const QString id = node.value(QStringLiteral("id")).toString();
        const QString type = node.value(QStringLiteral("type")).toString();
        if (id.isEmpty())
            continue;

        if (type == QLatin1String("directory")) {
            QJsonObject entry;
            entry[QStringLiteral("id")] = id;
            entry[QStringLiteral("path")] = node.value(QStringLiteral("path")).toString();
            entry[QStringLiteral("x")] = node.value(QStringLiteral("x")).toDouble();
            entry[QStringLiteral("y")] = node.value(QStringLiteral("y")).toDouble();
            entry[QStringLiteral("grants")] = QJsonArray();
            directoryById.insert(id, entry);
            directoryOrder.append(id);
        } else if (type == QLatin1String("role")) {
            QJsonObject entry;
            entry[QStringLiteral("id")] = id;
            entry[QStringLiteral("principal")] = node.value(QStringLiteral("principal")).toString();
            QJsonArray permissions;
            for (const QString &permission : permissionsFromLabel(node.value(QStringLiteral("permissionLabel")).toString()))
                permissions.append(permission);
            entry[QStringLiteral("permissions")] = permissions;
            entry[QStringLiteral("x")] = node.value(QStringLiteral("x")).toDouble();
            entry[QStringLiteral("y")] = node.value(QStringLiteral("y")).toDouble();
            roleById.insert(id, entry);
            roleOrder.append(id);
        }
    }

    for (const QVariant &connectionVariant : connections) {
        const QVariantMap connection = connectionVariant.toMap();
        const QString from = connection.value(QStringLiteral("from")).toString();
        const QString to = connection.value(QStringLiteral("to")).toString();
        if (!directoryById.contains(from) || !roleById.contains(to))
            continue;

        const QJsonObject role = roleById.value(to);
        QJsonObject grant;
        grant[QStringLiteral("roleId")] = to;
        grant[QStringLiteral("principal")] = role.value(QStringLiteral("principal"));
        grant[QStringLiteral("permissions")] = role.value(QStringLiteral("permissions"));

        QJsonObject directory = directoryById.value(from);
        QJsonArray grants = directory.value(QStringLiteral("grants")).toArray();
        grants.append(grant);
        directory[QStringLiteral("grants")] = grants;
        directoryById.insert(from, directory);
    }

    QJsonArray directoriesArray;
    for (const QString &id : directoryOrder)
        directoriesArray.append(directoryById.value(id));

    QJsonArray rolesArray;
    for (const QString &id : roleOrder)
        rolesArray.append(roleById.value(id));

    QJsonObject root;
    root[QStringLiteral("directories")] = directoriesArray;
    root[QStringLiteral("roles")] = rolesArray;

    const QDir configDir(QFileInfo(m_configPath).absolutePath());
    if (!configDir.exists() && !configDir.mkpath(QStringLiteral("."))) {
        setLastError(QStringLiteral("Could not create the config directory."));
        return false;
    }

    QFile file(m_configPath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        setLastError(QStringLiteral("Could not open %1 for writing.").arg(m_configPath));
        return false;
    }
    file.write(QJsonDocument(root).toJson(QJsonDocument::Indented));
    file.close();

    setLastError(QString());
    loadConfig();
    emit configSaved();
    return true;
}

bool PermissionConfigController::loadConfig()
{
    QFile file(m_configPath);
    if (!file.exists()) {
        m_hasSavedConfig = false;
        emit configLoaded();
        return false;
    }

    if (!file.open(QIODevice::ReadOnly)) {
        setLastError(QStringLiteral("Could not open %1 for reading.").arg(m_configPath));
        m_hasSavedConfig = false;
        emit configLoaded();
        return false;
    }

    QJsonParseError parseError;
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &parseError);
    file.close();

    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        setLastError(QStringLiteral("Failed to parse access control config: %1").arg(parseError.errorString()));
        m_hasSavedConfig = false;
        emit configLoaded();
        return false;
    }

    const QJsonObject root = doc.object();
    const QJsonArray directories = root.value(QStringLiteral("directories")).toArray();
    const QJsonArray roles = root.value(QStringLiteral("roles")).toArray();

    QVariantList directoryNodes;
    QVariantList connectionList;

    for (const QJsonValue &directoryValue : directories) {
        const QJsonObject directory = directoryValue.toObject();
        const QString id = directory.value(QStringLiteral("id")).toString();

        QVariantMap node;
        node[QStringLiteral("id")] = id;
        node[QStringLiteral("path")] = directory.value(QStringLiteral("path")).toString();
        node[QStringLiteral("x")] = directory.value(QStringLiteral("x")).toDouble();
        node[QStringLiteral("y")] = directory.value(QStringLiteral("y")).toDouble();
        directoryNodes.append(node);

        for (const QJsonValue &grantValue : directory.value(QStringLiteral("grants")).toArray()) {
            const QJsonObject grant = grantValue.toObject();
            QVariantMap connection;
            connection[QStringLiteral("from")] = id;
            connection[QStringLiteral("to")] = grant.value(QStringLiteral("roleId")).toString();
            connectionList.append(connection);
        }
    }

    QVariantList roleNodes;
    for (const QJsonValue &roleValue : roles) {
        const QJsonObject role = roleValue.toObject();
        QVariantMap node;
        node[QStringLiteral("id")] = role.value(QStringLiteral("id")).toString();
        node[QStringLiteral("principal")] = role.value(QStringLiteral("principal")).toString();
        node[QStringLiteral("permissionLabel")] = labelFromPermissions(role.value(QStringLiteral("permissions")).toArray());
        node[QStringLiteral("x")] = role.value(QStringLiteral("x")).toDouble();
        node[QStringLiteral("y")] = role.value(QStringLiteral("y")).toDouble();
        roleNodes.append(node);
    }

    m_directoryNodes = directoryNodes;
    m_roleNodes = roleNodes;
    m_connections = connectionList;
    m_hasSavedConfig = true;
    setLastError(QString());
    emit configLoaded();
    return true;
}
