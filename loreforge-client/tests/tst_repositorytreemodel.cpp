// Unit tests for RepositoryTreeModel's pure data-transformation logic: tree
// building/flattening from JSON (matching the shape of GET .../tree, see
// lorehub-api/src/models.rs's TreeNode), the Sparse Workspace Manager's
// include/exclude/cascade-exclude behavior, and staged-change bookkeeping.
//
// Deliberately does NOT exercise anything that requires a live network call
// (loadRepository(), uploadFile(), stageChange()/unstageChange() themselves)
// — those are integration-level concerns already covered by lorehub-api's
// own test suite. Where a method's *effect* is pure network-response
// handling (applyTreeJson/applyPendingJson), the tests call it directly via
// the RepositoryTreeModelTest friend declaration in RepositoryTreeModel.h.

#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonObject>
#include <QSettings>
#include <QTest>
#include <QVariantMap>

#include "RepositoryTreeModel.h"

class RepositoryTreeModelTest : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void testTreeBuildingAndFlattenAndCollapse();
    void testSparseWorkspaceDefaults();
    void testToggleWorkspaceInclusionRevealsChildren();
    void testCascadeExcludeAndReincludeResetsChild();
    void testStagedChanges();

private:
    static QJsonArray buildSampleTree();
};

// Sample tree shape (mirrors lorehub-api's TreeNode JSON exactly):
//
// dirA/                 (depth 0, directory)
//   dirB/                (depth 1, directory)
//     dirC/                (depth 2, directory)
//       fileZ.txt            (depth 3, text)
//     fileY.txt             (depth 2, text)
//   fileX.txt              (depth 1, text)
// dirRoot2/              (depth 0, directory)
//   fileW.txt              (depth 1, text)
QJsonArray RepositoryTreeModelTest::buildSampleTree()
{
    auto makeFile = [](const QString &path, const QString &name) {
        QJsonObject obj;
        obj["kind"] = QStringLiteral("text");
        obj["path"] = path;
        obj["name"] = name;
        obj["sizeLabel"] = QStringLiteral("1 KB");
        obj["updatedAt"] = QStringLiteral("2026-01-01T00:00:00Z");
        obj["lockedBy"] = QJsonValue();
        return obj;
    };
    auto makeDir = [](const QString &path, const QString &name, const QJsonArray &children) {
        QJsonObject obj;
        obj["kind"] = QStringLiteral("directory");
        obj["path"] = path;
        obj["name"] = name;
        obj["children"] = children;
        return obj;
    };

    const QJsonObject fileZ = makeFile(QStringLiteral("dirA/dirB/dirC/fileZ.txt"), QStringLiteral("fileZ.txt"));
    const QJsonObject dirC = makeDir(QStringLiteral("dirA/dirB/dirC"), QStringLiteral("dirC"),
                                      QJsonArray{ fileZ });
    const QJsonObject fileY = makeFile(QStringLiteral("dirA/dirB/fileY.txt"), QStringLiteral("fileY.txt"));
    const QJsonObject dirB = makeDir(QStringLiteral("dirA/dirB"), QStringLiteral("dirB"),
                                      QJsonArray{ dirC, fileY });
    const QJsonObject fileX = makeFile(QStringLiteral("dirA/fileX.txt"), QStringLiteral("fileX.txt"));
    const QJsonObject dirA = makeDir(QStringLiteral("dirA"), QStringLiteral("dirA"),
                                      QJsonArray{ dirB, fileX });

    const QJsonObject fileW = makeFile(QStringLiteral("dirRoot2/fileW.txt"), QStringLiteral("fileW.txt"));
    const QJsonObject dirRoot2 = makeDir(QStringLiteral("dirRoot2"), QStringLiteral("dirRoot2"),
                                          QJsonArray{ fileW });

    return QJsonArray{ dirA, dirRoot2 };
}

void RepositoryTreeModelTest::initTestCase()
{
    // Isolate QSettings (used by the Sparse Workspace Manager for
    // persisted include/exclude state) from the real app's registry
    // location — a distinct org/app name that only this test binary uses.
    QCoreApplication::setOrganizationName(QStringLiteral("LoreForgeClientTests"));
    QCoreApplication::setApplicationName(QStringLiteral("RepositoryTreeModelTests"));
}

void RepositoryTreeModelTest::init()
{
    QSettings settings;
    settings.clear();
}

void RepositoryTreeModelTest::cleanup()
{
    QSettings settings;
    settings.clear();
}

void RepositoryTreeModelTest::testTreeBuildingAndFlattenAndCollapse()
{
    RepositoryTreeModel model;
    model.m_slug = QStringLiteral("test-repo-flatten");
    model.applyTreeJson(buildSampleTree());

    auto pathAt = [&](int row) {
        return model.data(model.index(row), RepositoryTreeModel::PathRole).toString();
    };
    auto depthAt = [&](int row) {
        return model.data(model.index(row), RepositoryTreeModel::DepthRole).toInt();
    };

    // dirA and dirRoot2 default-include (depth 0) and start expanded on
    // first load, so dirA's depth-1 children show; dirB (depth 1) defaults
    // to excluded so ITS children stay hidden.
    QCOMPARE(model.rowCount(), 5);
    QCOMPARE(pathAt(0), QStringLiteral("dirA"));
    QCOMPARE(depthAt(0), 0);
    QCOMPARE(pathAt(1), QStringLiteral("dirA/dirB"));
    QCOMPARE(depthAt(1), 1);
    QCOMPARE(pathAt(2), QStringLiteral("dirA/fileX.txt"));
    QCOMPARE(depthAt(2), 1);
    QCOMPARE(pathAt(3), QStringLiteral("dirRoot2"));
    QCOMPARE(depthAt(3), 0);
    QCOMPARE(pathAt(4), QStringLiteral("dirRoot2/fileW.txt"));
    QCOMPARE(depthAt(4), 1);

    // Collapsing dirA must actually remove its descendants from rowCount(),
    // not just mark them hidden.
    model.toggleExpanded(QStringLiteral("dirA"));
    QCOMPARE(model.rowCount(), 3);
    QCOMPARE(pathAt(0), QStringLiteral("dirA"));
    QCOMPARE(pathAt(1), QStringLiteral("dirRoot2"));
    QCOMPARE(pathAt(2), QStringLiteral("dirRoot2/fileW.txt"));

    // Re-expanding restores the full flattened view.
    model.toggleExpanded(QStringLiteral("dirA"));
    QCOMPARE(model.rowCount(), 5);
}

void RepositoryTreeModelTest::testSparseWorkspaceDefaults()
{
    RepositoryTreeModel model;
    model.m_slug = QStringLiteral("test-repo-defaults");
    model.applyTreeJson(buildSampleTree());

    // Depth-0 directories default to included=true.
    QCOMPARE(model.rowForPath(QStringLiteral("dirA")).value("included").toBool(), true);
    QCOMPARE(model.rowForPath(QStringLiteral("dirRoot2")).value("included").toBool(), true);
    // Deeper directories default to included=false.
    QCOMPARE(model.rowForPath(QStringLiteral("dirA/dirB")).value("included").toBool(), false);
}

void RepositoryTreeModelTest::testToggleWorkspaceInclusionRevealsChildren()
{
    RepositoryTreeModel model;
    model.m_slug = QStringLiteral("test-repo-reveal");
    model.applyTreeJson(buildSampleTree());

    // dirB (depth 1) starts excluded, so its children aren't in the
    // flattened view at all.
    QVERIFY(model.rowForPath(QStringLiteral("dirA/dirB/dirC")).isEmpty());
    QVERIFY(model.rowForPath(QStringLiteral("dirA/dirB/fileY.txt")).isEmpty());

    model.toggleWorkspaceInclusion(QStringLiteral("dirA/dirB"));

    // Including dirB reveals its immediate children...
    const QVariantMap dirCRow = model.rowForPath(QStringLiteral("dirA/dirB/dirC"));
    const QVariantMap fileYRow = model.rowForPath(QStringLiteral("dirA/dirB/fileY.txt"));
    QVERIFY(!dirCRow.isEmpty());
    QVERIFY(!fileYRow.isEmpty());

    // ...but a nested directory child (dirC) stays excluded itself, one
    // level of reveal at a time (real sparse-checkout semantics).
    QCOMPARE(dirCRow.value("included").toBool(), false);
}

void RepositoryTreeModelTest::testCascadeExcludeAndReincludeResetsChild()
{
    RepositoryTreeModel model;
    model.m_slug = QStringLiteral("test-repo-cascade");
    model.applyTreeJson(buildSampleTree());

    // Explicitly include dirB, a child of dirA.
    model.toggleWorkspaceInclusion(QStringLiteral("dirA/dirB"));
    QCOMPARE(model.rowForPath(QStringLiteral("dirA/dirB")).value("included").toBool(), true);

    // Excluding the parent (dirA) must cascade-exclude the included
    // descendant (dirB) too, not leave it orphaned.
    model.toggleWorkspaceInclusion(QStringLiteral("dirA"));
    QCOMPARE(model.rowForPath(QStringLiteral("dirA")).value("included").toBool(), false);

    // dirA is now collapsed/excluded so dirB has no flattened row any more
    // — inspect the underlying tree directly (friend access) to confirm
    // the cascade actually reset dirB's state rather than just hiding it.
    TreeItem *dirBItem = RepositoryTreeModel::findByPath(model.m_roots, QStringLiteral("dirA/dirB"));
    QVERIFY(dirBItem != nullptr);
    QCOMPARE(dirBItem->included, false);

    // Re-including dirA must NOT bring dirB back included — the cascade
    // reset its state, it didn't just visually hide it.
    model.toggleWorkspaceInclusion(QStringLiteral("dirA"));
    QCOMPARE(model.rowForPath(QStringLiteral("dirA")).value("included").toBool(), true);
    const QVariantMap dirBRow = model.rowForPath(QStringLiteral("dirA/dirB"));
    QVERIFY(!dirBRow.isEmpty());
    QCOMPARE(dirBRow.value("included").toBool(), false);
}

void RepositoryTreeModelTest::testStagedChanges()
{
    RepositoryTreeModel model;
    model.m_slug = QStringLiteral("test-repo-staged");
    model.applyTreeJson(buildSampleTree());

    QCOMPARE(model.pendingCount(), 0);

    // applyPendingJson() is exactly what stageChange()/unstageChange()'s
    // network-completion handlers call with the server's .../pending
    // response — exercising it directly tests the real update logic
    // without needing a live backend.
    QJsonObject entry1;
    entry1["path"] = QStringLiteral("dirA/fileX.txt");
    entry1["changeType"] = QStringLiteral("modified");

    QJsonArray pending{ entry1 };
    model.applyPendingJson(pending);

    QCOMPARE(model.pendingCount(), 1);
    QCOMPARE(model.rowForPath(QStringLiteral("dirA/fileX.txt")).value("stagedChangeType").toString(),
             QStringLiteral("modified"));

    QJsonObject entry2;
    entry2["path"] = QStringLiteral("dirRoot2/fileW.txt");
    entry2["changeType"] = QStringLiteral("added");
    pending.append(entry2);
    model.applyPendingJson(pending);
    QCOMPARE(model.pendingCount(), 2);
    QCOMPARE(model.rowForPath(QStringLiteral("dirRoot2/fileW.txt")).value("stagedChangeType").toString(),
             QStringLiteral("added"));

    // Server drops an entry from .../pending once it's unstaged.
    QJsonArray onlyFirst{ entry1 };
    model.applyPendingJson(onlyFirst);
    QCOMPARE(model.pendingCount(), 1);
    QVERIFY(model.rowForPath(QStringLiteral("dirRoot2/fileW.txt")).value("stagedChangeType").toString().isEmpty());

    // Clearing all staged changes returns pendingCount to 0.
    model.applyPendingJson(QJsonArray());
    QCOMPARE(model.pendingCount(), 0);
}

QTEST_GUILESS_MAIN(RepositoryTreeModelTest)
#include "tst_repositorytreemodel.moc"
