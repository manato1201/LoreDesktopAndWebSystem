#include "RepositoryTreeModel.h"
#include "ApiClient.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QSettings>
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
    case StagedChangeTypeRole:
        return row.stagedChangeType;
    case IncludedRole:
        return row.included;
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
        { StagedChangeTypeRole, "stagedChangeType" },
        { IncludedRole, "included" },
    };
}

void RepositoryTreeModel::loadRepository(const QString &slug)
{
    m_slug = slug;
    m_roots.clear();
    m_pendingChanges.clear();

    beginResetModel();
    m_flatRows.clear();
    endResetModel();
    emit countChanged();
    emit pendingCountChanged();

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

    fetchPending();
}

void RepositoryTreeModel::toggleExpanded(const QString &path)
{
    TreeItem *item = findByPath(m_roots, path);
    if (!item || item->kind != QLatin1String("directory"))
        return;
    // Sparse Workspace Manager: a directory not yet "checked out" into the
    // workspace has no arrow in the UI (it shows a "+" affordance instead),
    // but guard here too in case something else calls this directly.
    if (!item->included)
        return;

    item->expanded = !item->expanded;

    beginResetModel();
    rebuildFlatRows();
    endResetModel();
    emit countChanged();
}

void RepositoryTreeModel::toggleWorkspaceInclusion(const QString &path)
{
    TreeItem *item = findByPath(m_roots, path);
    if (!item || item->kind != QLatin1String("directory"))
        return;

    const bool nowIncluded = !item->included;
    item->included = nowIncluded;

    if (nowIncluded) {
        // Reveal this directory's immediate children right away, matching
        // the "+" affordance's promise — children stay excluded themselves
        // (one level at a time), per real sparse-checkout semantics.
        item->expanded = true;
    } else {
        item->expanded = false;
        // Don't leave orphaned included children under an excluded parent.
        cascadeExcludeChildren(item->children);
    }

    saveInclusion(path, nowIncluded);

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
            map["stagedChangeType"] = row.stagedChangeType;
            map["included"] = row.included;
            return map;
        }
    }
    return {};
}

void RepositoryTreeModel::stageChange(const QString &path, const QString &changeType)
{
    if (m_slug.isEmpty())
        return;

    setErrorMessage(QString());

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + m_slug + "/tree/stage"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject body;
    body["path"] = path;
    body["changeType"] = changeType;
    body["staged"] = true;

    QNetworkReply *reply = ApiClient::networkManager().post(request, QJsonDocument(body).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        applyPendingJson(QJsonDocument::fromJson(reply->readAll()).array());
        reply->deleteLater();
    });
}

void RepositoryTreeModel::unstageChange(const QString &path)
{
    if (m_slug.isEmpty())
        return;

    setErrorMessage(QString());

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + m_slug + "/tree/stage"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject body;
    body["path"] = path;
    // The server ignores changeType when staged is false, but the field is
    // required on the wire — reuse whatever this path was staged as if we
    // still know it, otherwise a harmless placeholder.
    body["changeType"] = m_pendingChanges.value(path, QStringLiteral("modified"));
    body["staged"] = false;

    QNetworkReply *reply = ApiClient::networkManager().post(request, QJsonDocument(body).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        applyPendingJson(QJsonDocument::fromJson(reply->readAll()).array());
        reply->deleteLater();
    });
}

void RepositoryTreeModel::fetchPending()
{
    if (m_slug.isEmpty())
        return;

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + m_slug + "/pending"));
    QNetworkReply *reply = ApiClient::networkManager().get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        if (reply->error() != QNetworkReply::NoError) {
            // Non-fatal: staged-row indicators just won't show until the
            // next successful refresh.
            reply->deleteLater();
            return;
        }

        applyPendingJson(QJsonDocument::fromJson(reply->readAll()).array());
        reply->deleteLater();
    });
}

void RepositoryTreeModel::applyPendingJson(const QJsonArray &array)
{
    QMap<QString, QString> pending;
    for (const QJsonValue &value : array) {
        const QJsonObject obj = value.toObject();
        pending.insert(obj.value("path").toString(), obj.value("changeType").toString());
    }
    m_pendingChanges = pending;

    beginResetModel();
    rebuildFlatRows();
    endResetModel();
    emit countChanged();
    emit pendingCountChanged();
}

void RepositoryTreeModel::applyTreeJson(const QJsonArray &array)
{
    QMap<QString, bool> expandedState;
    collectExpanded(m_roots, expandedState);
    const bool firstLoad = expandedState.isEmpty();

    QVector<TreeItem> parsed = parseNodes(array);
    applyExpandedState(parsed, expandedState, firstLoad, 0);
    // Sparse Workspace Manager: inclusion is client-local persisted state
    // (QSettings), unrelated to the expand/collapse session state above —
    // reload it fresh from disk every time the tree arrives from the server.
    applyInclusionState(parsed, 0);

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

void RepositoryTreeModel::applyInclusionState(QVector<TreeItem> &nodes, int depth) const
{
    if (m_slug.isEmpty())
        return;

    // Sparse Workspace Manager default: top-level directories start
    // "checked out" so a first-time user sees something rather than an
    // empty, all-collapsed tree; anything deeper starts excluded until the
    // user explicitly opts in.
    const bool defaultIncluded = depth == 0;

    QSettings settings;
    settings.beginGroup(QLatin1String("SparseWorkspace/") + m_slug);
    for (TreeItem &n : nodes) {
        if (n.kind == QLatin1String("directory")) {
            n.included = settings.value(n.path, defaultIncluded).toBool();
            applyInclusionState(n.children, depth + 1);
        }
    }
    settings.endGroup();
}

void RepositoryTreeModel::cascadeExcludeChildren(QVector<TreeItem> &nodes) const
{
    for (TreeItem &n : nodes) {
        if (n.kind != QLatin1String("directory"))
            continue;
        if (n.included) {
            n.included = false;
            saveInclusion(n.path, false);
        }
        n.expanded = false;
        cascadeExcludeChildren(n.children);
    }
}

void RepositoryTreeModel::saveInclusion(const QString &path, bool included) const
{
    if (m_slug.isEmpty())
        return;

    QSettings settings;
    settings.beginGroup(QLatin1String("SparseWorkspace/") + m_slug);
    settings.setValue(path, included);
    settings.endGroup();
}

void RepositoryTreeModel::rebuildFlatRows()
{
    m_flatRows.clear();
    flattenInto(m_roots, 0, m_flatRows, m_pendingChanges);
}

void RepositoryTreeModel::flattenInto(const QVector<TreeItem> &nodes, int depth, QVector<FlatRow> &out,
                                       const QMap<QString, QString> &staged)
{
    for (const TreeItem &n : nodes) {
        FlatRow row;
        row.path = n.path;
        row.name = n.name;
        row.kind = n.kind;
        row.sizeLabel = n.sizeLabel;
        row.updatedAt = n.updatedAt;
        row.lockedBy = n.lockedBy;
        row.stagedChangeType = staged.value(n.path);
        row.depth = depth;
        row.isDirectory = n.kind == QLatin1String("directory");
        row.hasChildren = row.isDirectory && !n.children.isEmpty();
        row.expanded = n.expanded;
        row.included = n.included;
        out.append(row);

        // A directory that isn't included (checked out) into the workspace
        // still shows its own row (so the "+" affordance is reachable) but
        // collapses its children out of the flattened list entirely — same
        // mechanism as the expanded-gate below, just a second condition.
        if (row.isDirectory && n.expanded && n.included)
            flattenInto(n.children, depth + 1, out, staged);
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
