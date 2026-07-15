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
 * Backed by GET /api/repositories on lorehub-api. Call refresh() once the
 * user is authenticated (the endpoint requires the session cookie); see
 * ARCHITECTURE.md §2.2.
 */
class RepositoryListModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY errorMessageChanged)

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
    bool busy() const { return m_busy; }
    QString errorMessage() const { return m_errorMessage; }

    Q_INVOKABLE void refresh();

signals:
    void countChanged();
    void busyChanged();
    void errorMessageChanged();

private:
    void setBusy(bool busy);
    void setErrorMessage(const QString &message);

    QList<RepositoryEntry> m_repositories;
    bool m_busy = false;
    QString m_errorMessage;
};
