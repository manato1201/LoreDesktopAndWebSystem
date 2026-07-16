#pragma once

#include <QAbstractListModel>
#include <QMap>
#include <QQmlEngine>
#include <QString>
#include <QVariantMap>
#include <QVector>

class QJsonArray;

/** One node of the nested tree as returned by GET .../tree. */
struct TreeItem
{
    QString path;
    QString name;
    QString kind; // "directory" | "text" | "image" | "model3d" | "audio" | "binary"
    QString sizeLabel;
    QString updatedAt;
    QString lockedBy; // empty when unlocked
    bool expanded = false;
    QVector<TreeItem> children;
};

/**
 * Flattened, expand/collapse-aware view over a repository's file tree
 * (GET /api/repositories/{slug}/tree) so QML can render it with a plain
 * ListView instead of QAbstractItemModel + TreeView's row/parent bookkeeping.
 * Visible rows are recomputed into m_flatRows whenever the tree changes;
 * this is a mock-sized tree so the O(n) rebuild is not a concern.
 *
 * Also owns the staging write-path (GET/POST .../pending, POST
 * .../tree/stage): staged paths are tracked in m_pendingChanges and merged
 * into each row's stagedChangeType at flatten time, so QML can render a
 * per-row staged indicator without a separate list model.
 */
class RepositoryTreeModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY errorMessageChanged)
    Q_PROPERTY(int pendingCount READ pendingCount NOTIFY pendingCountChanged)

public:
    enum Roles {
        PathRole = Qt::UserRole + 1,
        NameRole,
        KindRole,
        SizeLabelRole,
        UpdatedAtRole,
        LockedByRole,
        DepthRole,
        HasChildrenRole,
        ExpandedRole,
        IsDirectoryRole,
        StagedChangeTypeRole,
    };

    explicit RepositoryTreeModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    int count() const { return static_cast<int>(m_flatRows.size()); }
    bool busy() const { return m_busy; }
    QString errorMessage() const { return m_errorMessage; }
    int pendingCount() const { return m_pendingChanges.size(); }

    Q_INVOKABLE void loadRepository(const QString &slug);
    Q_INVOKABLE void toggleExpanded(const QString &path);
    Q_INVOKABLE void toggleLock(const QString &path, bool lock);
    Q_INVOKABLE QVariantMap rowForPath(const QString &path) const;

    /** POSTs .../tree/stage with staged=true and the given changeType. */
    Q_INVOKABLE void stageChange(const QString &path, const QString &changeType);
    /** POSTs .../tree/stage with staged=false, clearing any pending entry. */
    Q_INVOKABLE void unstageChange(const QString &path);

signals:
    void countChanged();
    void busyChanged();
    void errorMessageChanged();
    void treeLoaded();
    void pendingCountChanged();

private:
    struct FlatRow
    {
        QString path;
        QString name;
        QString kind;
        QString sizeLabel;
        QString updatedAt;
        QString lockedBy;
        QString stagedChangeType; // empty when not staged
        int depth = 0;
        bool hasChildren = false;
        bool expanded = false;
        bool isDirectory = false;
    };

    void setBusy(bool busy);
    void setErrorMessage(const QString &message);
    void applyTreeJson(const QJsonArray &array);
    void applyPendingJson(const QJsonArray &array);
    void fetchPending();
    static QVector<TreeItem> parseNodes(const QJsonArray &array);
    void collectExpanded(const QVector<TreeItem> &nodes, QMap<QString, bool> &state) const;
    static void applyExpandedState(QVector<TreeItem> &nodes, const QMap<QString, bool> &state,
                                    bool firstLoad, int depth);
    void rebuildFlatRows();
    static void flattenInto(const QVector<TreeItem> &nodes, int depth, QVector<FlatRow> &out,
                             const QMap<QString, QString> &staged);
    static TreeItem *findByPath(QVector<TreeItem> &nodes, const QString &path);

    QString m_slug;
    bool m_busy = false;
    QString m_errorMessage;
    QVector<TreeItem> m_roots;
    QVector<FlatRow> m_flatRows;
    /** path -> changeType ("added" | "modified" | "deleted") for staged files. */
    QMap<QString, QString> m_pendingChanges;
};
