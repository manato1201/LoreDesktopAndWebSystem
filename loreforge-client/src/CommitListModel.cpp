#include "CommitListModel.h"
#include "ApiClient.h"

#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>

#include <algorithm>

CommitListModel::CommitListModel(QObject *parent)
    : QAbstractListModel(parent)
{
}

int CommitListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return 0;
    return static_cast<int>(m_commits.size());
}

QVariant CommitListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_commits.size())
        return {};

    const CommitEntry &entry = m_commits.at(index.row());
    switch (role) {
    case HashRole:
        return entry.hash;
    case ShortHashRole:
        return entry.shortHash;
    case MessageRole:
        return entry.message;
    case DescriptionRole:
        return entry.description;
    case AuthorRole:
        return entry.author;
    case AuthorInitialsRole:
        return entry.authorInitials;
    case TimestampRole:
        return entry.timestamp;
    case BranchRole:
        return entry.branch;
    case BranchLaneRole:
        return entry.branchLane;
    case ChangedFileCountRole:
        return static_cast<int>(entry.changedFiles.size());
    default:
        return {};
    }
}

QHash<int, QByteArray> CommitListModel::roleNames() const
{
    return {
        { HashRole, "hash" },
        { ShortHashRole, "shortHash" },
        { MessageRole, "message" },
        { DescriptionRole, "description" },
        { AuthorRole, "author" },
        { AuthorInitialsRole, "authorInitials" },
        { TimestampRole, "timestamp" },
        { BranchRole, "branch" },
        { BranchLaneRole, "branchLane" },
        { ChangedFileCountRole, "changedFileCount" },
    };
}

void CommitListModel::refresh(const QString &slug)
{
    m_slug = slug;

    setErrorMessage(QString());
    setBusy(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories/" + slug + "/commits"));
    QNetworkReply *reply = ApiClient::networkManager().get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        setBusy(false);

        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        const QJsonArray array = QJsonDocument::fromJson(reply->readAll()).array();

        QVector<CommitEntry> parsed;
        parsed.reserve(array.size());
        for (const QJsonValue &value : array) {
            const QJsonObject obj = value.toObject();
            CommitEntry entry;
            entry.hash = obj.value("hash").toString();
            entry.shortHash = obj.value("shortHash").toString();
            entry.message = obj.value("message").toString();
            entry.description = obj.value("description").toString();
            entry.author = obj.value("author").toString();
            entry.authorInitials = obj.value("authorInitials").toString();
            entry.timestamp = obj.value("timestamp").toString();
            entry.branch = obj.value("branch").toString();

            for (const QJsonValue &parentValue : obj.value("parents").toArray())
                entry.parents.append(parentValue.toString());

            for (const QJsonValue &fileValue : obj.value("changedFiles").toArray()) {
                const QJsonObject fileObj = fileValue.toObject();
                ChangedFileEntry file;
                file.path = fileObj.value("path").toString();
                file.changeType = fileObj.value("changeType").toString();
                file.sizeDeltaLabel = fileObj.value("sizeDeltaLabel").toString();
                entry.changedFiles.append(file);
            }

            parsed.append(entry);
        }

        // parsed is still oldest-first here, so lane assignment sees branches
        // in true first-appearance order before we reverse for display.
        QHash<QString, int> lanes;
        for (CommitEntry &entry : parsed) {
            auto it = lanes.find(entry.branch);
            if (it == lanes.end())
                it = lanes.insert(entry.branch, lanes.size());
            entry.branchLane = it.value();
        }

        std::reverse(parsed.begin(), parsed.end());

        beginResetModel();
        m_commits = parsed;
        endResetModel();
        emit countChanged();

        reply->deleteLater();
    });
}

QVariantMap CommitListModel::rowForHash(const QString &hash) const
{
    for (const CommitEntry &entry : m_commits) {
        if (entry.hash == hash)
            return toVariantMap(entry);
    }
    return {};
}

QVariantMap CommitListModel::toVariantMap(const CommitEntry &entry)
{
    QVariantMap map;
    map["hash"] = entry.hash;
    map["shortHash"] = entry.shortHash;
    map["message"] = entry.message;
    map["description"] = entry.description;
    map["author"] = entry.author;
    map["authorInitials"] = entry.authorInitials;
    map["timestamp"] = entry.timestamp;
    map["branch"] = entry.branch;
    map["branchLane"] = entry.branchLane;

    QVariantList files;
    for (const ChangedFileEntry &file : entry.changedFiles) {
        QVariantMap fileMap;
        fileMap["path"] = file.path;
        fileMap["changeType"] = file.changeType;
        fileMap["sizeDeltaLabel"] = file.sizeDeltaLabel;
        files.append(fileMap);
    }
    map["changedFiles"] = files;

    QVariantList parents;
    for (const QString &p : entry.parents)
        parents.append(p);
    map["parents"] = parents;

    return map;
}

void CommitListModel::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void CommitListModel::setErrorMessage(const QString &message)
{
    if (m_errorMessage == message)
        return;
    m_errorMessage = message;
    emit errorMessageChanged();
}
