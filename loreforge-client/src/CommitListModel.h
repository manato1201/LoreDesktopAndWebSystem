#pragma once

#include <QAbstractListModel>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QVariantMap>
#include <QVector>

class QJsonArray;

struct ChangedFileEntry
{
    QString path;
    QString changeType; // "added" | "modified" | "deleted"
    QString sizeDeltaLabel;
};

struct CommitEntry
{
    QString hash;
    QString shortHash;
    QString message;
    QString description;
    QString author;
    QString authorInitials;
    QString timestamp;
    QString branch;
    QVector<ChangedFileEntry> changedFiles;
    QStringList parents;
    int branchLane = 0;
};

/**
 * Backed by GET /api/repositories/{slug}/commits. The API delivers commits
 * oldest-first (every commit's parents appear earlier in the array); this
 * model reverses that so row 0 is the newest commit, matching how commit
 * history is usually read.
 *
 * branchLane is a lightweight nod to branch topology, not a full graph
 * layout: lanes are assigned in chronological (oldest-first) order, so lane
 * 0 is whichever branch the very first commit belongs to (the trunk in this
 * dataset) and every other branch gets the next free lane number the first
 * time it appears — mirroring the lane-assignment idea in lorehub-web's
 * graph-layout.ts without building an actual graph.
 */
class CommitListModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY errorMessageChanged)

public:
    enum Roles {
        HashRole = Qt::UserRole + 1,
        ShortHashRole,
        MessageRole,
        DescriptionRole,
        AuthorRole,
        AuthorInitialsRole,
        TimestampRole,
        BranchRole,
        BranchLaneRole,
        ChangedFileCountRole,
    };

    explicit CommitListModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    int count() const { return static_cast<int>(m_commits.size()); }
    bool busy() const { return m_busy; }
    QString errorMessage() const { return m_errorMessage; }

    Q_INVOKABLE void refresh(const QString &slug);
    Q_INVOKABLE QVariantMap rowForHash(const QString &hash) const;

signals:
    void countChanged();
    void busyChanged();
    void errorMessageChanged();

private:
    void setBusy(bool busy);
    void setErrorMessage(const QString &message);
    static QVariantMap toVariantMap(const CommitEntry &entry);

    QString m_slug;
    bool m_busy = false;
    QString m_errorMessage;
    QVector<CommitEntry> m_commits;
};
