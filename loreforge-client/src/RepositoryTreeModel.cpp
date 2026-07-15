#include "RepositoryTreeModel.h"
#include "ApiClient.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>

RepositoryTreeModel::RepositoryTreeModel(QObject *parent)
    : QAbstractListModel(parent)
{
}

int RepositoryTreeModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return 0;
    return static_cast<int>(m_flatRows.size());
}

QVariant RepositoryTreeModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_flatRows.size())
        return {};

    const FlatRow &row = m_flatRows.at(index.row());
    switch (role) {
    case PathRole:
        return row.path;
    case NameRole:
        return row.name;
    case KindRole:
        return row.kind;
    case SizeLabelRole:
        return row.sizeLabel;
    case UpdatedAtRole:
        return row.updatedAt;
    case LockedByRole:
        return row.lockedBy;
    case DepthRole:
        return row.depth;
    case HasChildrenRole:
        return row.hasChildren;
    case ExpandedRole:
        return row.expanded;
    case IsDirectoryRole:
        return row.isDirectory;
    default:
        return {};
    }
}

QHash<int, QByteArray> RepositoryTreeModel::roleNames() const
{
    return {
        { PathRole, "path" },
        { NameRole, "name" },
        { KindRole, "kind" },
        { SizeLabelRole, "sizeLabel" },
        { UpdatedAtRole, "updatedAt" },
        { LockedByRole, "lockedBy" },
        { DepthRole, "depth" },
        { HasChildrenRole, "hasChildren" },
        { ExpandedRole, "expanded" },
        { IsDirectoryRole, "isDirectory" },
    };
}

void RepositoryTreeModel::loadRepository(const QString &slug)
{
    m_slug = slug;
    m_roots.clear();

    beginResetModel();
    m_flatRows.clear();
    endResetModel();
    emit countChanged();

    setErrorMessage(QString());
    setBusy(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + slug + "/tree"));
    QNetworkReply *reply = ApiClient::networkManager().get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        setBusy(false);

        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        applyTreeJson(QJsonDocument::fromJson(reply->readAll()).array());
        reply->deleteLater();
    });
}

void RepositoryTreeModel::toggleExpanded(const QString &path)
{
    TreeItem *item = findByPath(m_roots, path);
    if (!item || item->kind != QLatin1String("directory"))
        return;

    item->expanded = !item->expanded;

    beginResetModel();
    rebuildFlatRows();
    endResetModel();
    emit countChanged();
}

void RepositoryTreeModel::toggleLock(const QString &path, bool lock)
{
    if (m_slug.isEmpty())
        return;

    setErrorMessage(QString());
    setBusy(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + m_slug + "/tree/lock"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject body;
    body["path"] = path;
    body["lock"] = lock;

    QNetworkReply *reply = ApiClient::networkManager().post(request, QJsonDocument(body).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        setBusy(false);

        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        applyTreeJson(QJsonDocument::fromJson(reply->readAll()).array());
        reply->deleteLater();
    });
}

QVariantMap RepositoryTreeModel::rowForPath(const QString &path) const
{
    for (const FlatRow &row : m_flatRows) {
        if (row.path == path) {
            QVariantMap map;
            map["path"] = row.path;
            map["name"] = row.name;
            map["kind"] = row.kind;
            map["sizeLabel"] = row.sizeLabel;
            map["updatedAt"] = row.updatedAt;
            map["lockedBy"] = row.lockedBy;
            map["depth"] = row.depth;
            map["hasChildren"] = row.hasChildren;
            map["expanded"] = row.expanded;
            map["isDirectory"] = row.isDirectory;
            return map;
        }
    }
    return {};
}

void RepositoryTreeModel::applyTreeJson(const QJsonArray &array)
{
    QMap<QString, bool> expandedState;
    collectExpanded(m_roots, expandedState);
    const bool firstLoad = expandedState.isEmpty();

    QVector<TreeItem> parsed = parseNodes(array);
    applyExpandedState(parsed, expandedState, firstLoad, 0);

    beginResetModel();
    m_roots = parsed;
    rebuildFlatRows();
    endResetModel();
    emit countChanged();

    emit treeLoaded();
}

QVector<TreeItem> RepositoryTreeModel::parseNodes(const QJsonArray &array)
{
    QVector<TreeItem> result;
    result.reserve(array.size());

    for (const QJsonValue &value : array) {
        const QJsonObject obj = value.toObject();
        TreeItem item;
        item.kind = obj.value("kind").toString();
        item.path = obj.value("path").toString();
        item.name = obj.value("name").toString();

        if (item.kind == QLatin1String("directory")) {
            item.children = parseNodes(obj.value("children").toArray());
        } else {
            item.sizeLabel = obj.value("sizeLabel").toString();
            item.updatedAt = obj.value("updatedAt").toString();
            const QJsonValue lockedBy = obj.value("lockedBy");
            if (!lockedBy.isNull())
                item.lockedBy = lockedBy.toString();
        }

        result.append(item);
    }

    return result;
}

void RepositoryTreeModel::collectExpanded(const QVector<TreeItem> &nodes, QMap<QString, bool> &state) const
{
    for (const TreeItem &n : nodes) {
        if (n.kind == QLatin1String("directory")) {
            state.insert(n.path, n.expanded);
            collectExpanded(n.children, state);
        }
    }
}

void RepositoryTreeModel::applyExpandedState(QVector<TreeItem> &nodes, const QMap<QString, bool> &state,
                                              bool firstLoad, int depth)
{
    for (TreeItem &n : nodes) {
        if (n.kind == QLatin1String("directory")) {
            n.expanded = firstLoad ? (depth == 0) : state.value(n.path, false);
            applyExpandedState(n.children, state, firstLoad, depth + 1);
        }
    }
}

void RepositoryTreeModel::rebuildFlatRows()
{
    m_flatRows.clear();
    flattenInto(m_roots, 0, m_flatRows);
}

void RepositoryTreeModel::flattenInto(const QVector<TreeItem> &nodes, int depth, QVector<FlatRow> &out)
{
    for (const TreeItem &n : nodes) {
        FlatRow row;
        row.path = n.path;
        row.name = n.name;
        row.kind = n.kind;
        row.sizeLabel = n.sizeLabel;
        row.updatedAt = n.updatedAt;
        row.lockedBy = n.lockedBy;
        row.depth = depth;
        row.isDirectory = n.kind == QLatin1String("directory");
        row.hasChildren = row.isDirectory && !n.children.isEmpty();
        row.expanded = n.expanded;
        out.append(row);

        if (row.isDirectory && n.expanded)
            flattenInto(n.children, depth + 1, out);
    }
}

TreeItem *RepositoryTreeModel::findByPath(QVector<TreeItem> &nodes, const QString &path)
{
    for (TreeItem &n : nodes) {
        if (n.path == path)
            return &n;
        if (!n.children.isEmpty()) {
            if (TreeItem *found = findByPath(n.children, path))
                return found;
        }
    }
    return nullptr;
}

void RepositoryTreeModel::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void RepositoryTreeModel::setErrorMessage(const QString &message)
{
    if (m_errorMessage == message)
        return;
    m_errorMessage = message;
    emit errorMessageChanged();
}
