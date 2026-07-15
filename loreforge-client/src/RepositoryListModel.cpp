#include "RepositoryListModel.h"
#include "ApiClient.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>

RepositoryListModel::RepositoryListModel(QObject *parent)
    : QAbstractListModel(parent)
{
}

int RepositoryListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return 0;
    return static_cast<int>(m_repositories.size());
}

QVariant RepositoryListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_repositories.size())
        return {};

    const RepositoryEntry &entry = m_repositories.at(index.row());
    switch (role) {
    case SlugRole:
        return entry.slug;
    case NameRole:
        return entry.name;
    case OrganizationRole:
        return entry.organization;
    case DescriptionRole:
        return entry.description;
    case UpdatedAtRole:
        return entry.updatedAt;
    case SizeLabelRole:
        return entry.sizeLabel;
    case LockedFileCountRole:
        return entry.lockedFileCount;
    case VisibilityRole:
        return entry.visibility;
    default:
        return {};
    }
}

QHash<int, QByteArray> RepositoryListModel::roleNames() const
{
    return {
        { SlugRole, "slug" },
        { NameRole, "name" },
        { OrganizationRole, "organization" },
        { DescriptionRole, "description" },
        { UpdatedAtRole, "updatedAt" },
        { SizeLabelRole, "sizeLabel" },
        { LockedFileCountRole, "lockedFileCount" },
        { VisibilityRole, "visibility" },
    };
}

void RepositoryListModel::refresh()
{
    setErrorMessage(QString());
    setBusy(true);

    QNetworkRequest request(QUrl(ApiClient::baseUrl() + "/api/repositories"));
    QNetworkReply *reply = ApiClient::networkManager().get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        setBusy(false);

        if (reply->error() != QNetworkReply::NoError) {
            setErrorMessage(reply->errorString());
            reply->deleteLater();
            return;
        }

        const QJsonArray array = QJsonDocument::fromJson(reply->readAll()).array();
        QList<RepositoryEntry> entries;
        entries.reserve(array.size());
        for (const QJsonValue &value : array) {
            const QJsonObject obj = value.toObject();
            RepositoryEntry entry;
            entry.slug = obj.value("slug").toString();
            entry.name = obj.value("name").toString();
            entry.organization = obj.value("organization").toString();
            entry.description = obj.value("description").toString();
            entry.updatedAt = obj.value("updatedAt").toString();
            entry.sizeLabel = obj.value("sizeLabel").toString();
            entry.lockedFileCount = obj.value("lockedFileCount").toInt();
            entry.visibility = obj.value("visibility").toString();
            entries.append(entry);
        }

        beginResetModel();
        m_repositories = entries;
        endResetModel();
        emit countChanged();

        reply->deleteLater();
    });
}

void RepositoryListModel::setBusy(bool busy)
{
    if (m_busy == busy)
        return;
    m_busy = busy;
    emit busyChanged();
}

void RepositoryListModel::setErrorMessage(const QString &message)
{
    if (m_errorMessage == message)
        return;
    m_errorMessage = message;
    emit errorMessageChanged();
}
