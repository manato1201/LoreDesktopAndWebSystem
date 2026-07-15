#pragma once

#include <QAbstractListModel>
#include <QQmlEngine>

struct RepositoryEntry
{
    QString slug;
    QString name;
    QString organization;
    QString description;
    QString updatedAt;
    QString sizeLabel;
    int lockedFileCount = 0;
    QString visibility;
};

/**
 * Placeholder repository list mirroring lorehub-web's original mock
 * dataset, so the desktop client and the web app tell the same story
 * until this is wired up to a real `lore` CLI / Rust FFI backend (see
 * ARCHITECTURE.md §2.2).
 */
class RepositoryListModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(int count READ count CONSTANT)

public:
    enum Roles {
        SlugRole = Qt::UserRole + 1,
        NameRole,
        OrganizationRole,
        DescriptionRole,
        UpdatedAtRole,
        SizeLabelRole,
        LockedFileCountRole,
        VisibilityRole,
    };

    explicit RepositoryListModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    int count() const { return static_cast<int>(m_repositories.size()); }

private:
    QList<RepositoryEntry> m_repositories;
};
