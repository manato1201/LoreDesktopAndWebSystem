// Unit tests for PermissionConfigController's on-disk JSON schema: the
// saveConfig()/loadConfig() round-trip of directories/roles/connections/
// positions, hasSavedConfig() state transitions, safe behavior with no
// config file present, and the permission-label <-> permissions-array
// conversion staying consistent through a save/load cycle.
//
// The config file lives at QCoreApplication::applicationDirPath() +
// "/config/access-control.json" — this test binary has its own
// applicationDirPath() distinct from the real LoreForgeServerAdmin.exe, so
// nothing here can collide with a real user's saved config. Still, init()/
// cleanup() wipe that directory before and after every test so tests don't
// accumulate state across runs.
//
// Deliberately does NOT exercise login()/applyToServer()/refreshSession() —
// those need a live lorehub-api backend and are integration-level concerns
// covered by lorehub-api's own test suite.

#include <QCoreApplication>
#include <QDir>
#include <QTest>
#include <QVariantList>
#include <QVariantMap>

#include "PermissionConfigController.h"

namespace {

QVariantMap makeDirectoryNode(const QString &id, const QString &path, double x, double y)
{
    QVariantMap node;
    node["id"] = id;
    node["type"] = QStringLiteral("directory");
    node["path"] = path;
    node["x"] = x;
    node["y"] = y;
    return node;
}

QVariantMap makeRoleNode(const QString &id, const QString &principal, const QString &permissionLabel,
                          double x, double y)
{
    QVariantMap node;
    node["id"] = id;
    node["type"] = QStringLiteral("role");
    node["principal"] = principal;
    node["permissionLabel"] = permissionLabel;
    node["x"] = x;
    node["y"] = y;
    return node;
}

QVariantMap makeConnection(const QString &from, const QString &to)
{
    QVariantMap connection;
    connection["from"] = from;
    connection["to"] = to;
    return connection;
}

QVariantMap findById(const QVariantList &nodes, const QString &id)
{
    for (const QVariant &nodeVariant : nodes) {
        const QVariantMap node = nodeVariant.toMap();
        if (node.value("id").toString() == id)
            return node;
    }
    return {};
}

} // namespace

class PermissionConfigControllerTest : public QObject
{
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void testNoConfigFileIsSafeAndDefaultsFalse();
    void testSaveLoadRoundTrip();
    void testSaveConfigDeleteThenReloadCascade();
    void testPermissionLabelRoundTrip();

private:
    static void wipeConfigDir();
};

void PermissionConfigControllerTest::wipeConfigDir()
{
    const QString configDir = QCoreApplication::applicationDirPath() + QStringLiteral("/config");
    QDir dir(configDir);
    if (dir.exists())
        dir.removeRecursively();
}

void PermissionConfigControllerTest::init()
{
    wipeConfigDir();
}

void PermissionConfigControllerTest::cleanup()
{
    wipeConfigDir();
}

void PermissionConfigControllerTest::testNoConfigFileIsSafeAndDefaultsFalse()
{
    PermissionConfigController controller;

    QCOMPARE(controller.hasSavedConfig(), false);
    QVERIFY(controller.directoryNodes().isEmpty());
    QVERIFY(controller.roleNodes().isEmpty());
    QVERIFY(controller.connections().isEmpty());

    // Calling loadConfig() explicitly with still no file present must not
    // crash and must leave the controller in the same sane default state.
    QCOMPARE(controller.loadConfig(), false);
    QCOMPARE(controller.hasSavedConfig(), false);
}

void PermissionConfigControllerTest::testSaveLoadRoundTrip()
{
    PermissionConfigController controller;
    QCOMPARE(controller.hasSavedConfig(), false);

    QVariantList nodes;
    nodes.append(makeDirectoryNode("d1", "/repo/src", 10.5, 20.25));
    nodes.append(makeRoleNode("r1", "team-engineering", "Read / Write / Lock", 100.0, 200.0));

    QVariantList connections;
    connections.append(makeConnection("d1", "r1"));

    QVERIFY(controller.saveConfig(nodes, connections));
    QCOMPARE(controller.hasSavedConfig(), true);

    const QVariantMap d1 = findById(controller.directoryNodes(), "d1");
    QVERIFY(!d1.isEmpty());
    QCOMPARE(d1.value("path").toString(), QStringLiteral("/repo/src"));
    QCOMPARE(d1.value("x").toDouble(), 10.5);
    QCOMPARE(d1.value("y").toDouble(), 20.25);

    const QVariantMap r1 = findById(controller.roleNodes(), "r1");
    QVERIFY(!r1.isEmpty());
    QCOMPARE(r1.value("principal").toString(), QStringLiteral("team-engineering"));
    QCOMPARE(r1.value("permissionLabel").toString(), QStringLiteral("Read / Write / Lock"));
    QCOMPARE(r1.value("x").toDouble(), 100.0);
    QCOMPARE(r1.value("y").toDouble(), 200.0);

    QCOMPARE(controller.connections().size(), 1);
    const QVariantMap connection = controller.connections().first().toMap();
    QCOMPARE(connection.value("from").toString(), QStringLiteral("d1"));
    QCOMPARE(connection.value("to").toString(), QStringLiteral("r1"));

    // Round-trip through the actual file on disk, not just the in-memory
    // reload saveConfig() already does internally: a second, independent
    // controller instance reading the same applicationDirPath() must see
    // exactly the same data.
    PermissionConfigController reloaded;
    QCOMPARE(reloaded.hasSavedConfig(), true);
    QCOMPARE(reloaded.directoryNodes().size(), 1);
    QCOMPARE(reloaded.roleNodes().size(), 1);
    QCOMPARE(reloaded.connections().size(), 1);
    QCOMPARE(findById(reloaded.directoryNodes(), "d1").value("path").toString(),
             QStringLiteral("/repo/src"));
    QCOMPARE(findById(reloaded.roleNodes(), "r1").value("permissionLabel").toString(),
             QStringLiteral("Read / Write / Lock"));
}

void PermissionConfigControllerTest::testSaveConfigDeleteThenReloadCascade()
{
    PermissionConfigController controller;

    QVariantList nodes;
    nodes.append(makeDirectoryNode("d1", "/repo/src", 0.0, 0.0));
    nodes.append(makeDirectoryNode("d2", "/repo/docs", 50.0, 50.0));
    nodes.append(makeRoleNode("r1", "team-engineering", "Read / Write", 100.0, 0.0));

    QVariantList connections;
    connections.append(makeConnection("d1", "r1"));
    connections.append(makeConnection("d2", "r1"));

    QVERIFY(controller.saveConfig(nodes, connections));
    QCOMPARE(controller.directoryNodes().size(), 2);
    QCOMPARE(controller.connections().size(), 2);

    // Simulate the user deleting d2's node in the QML graph editor and
    // pressing save again: saveConfig() must fully replace the on-disk
    // schema, not merge with what was there before.
    QVariantList remainingNodes;
    remainingNodes.append(makeDirectoryNode("d1", "/repo/src", 0.0, 0.0));
    remainingNodes.append(makeRoleNode("r1", "team-engineering", "Read / Write", 100.0, 0.0));

    QVariantList remainingConnections;
    remainingConnections.append(makeConnection("d1", "r1"));

    QVERIFY(controller.saveConfig(remainingNodes, remainingConnections));
    QCOMPARE(controller.directoryNodes().size(), 1);
    QCOMPARE(controller.connections().size(), 1);
    QVERIFY(findById(controller.directoryNodes(), "d2").isEmpty());

    // Confirm the deletion actually landed on disk, not just in memory, by
    // reloading from a fresh instance.
    PermissionConfigController reloaded;
    QCOMPARE(reloaded.directoryNodes().size(), 1);
    QCOMPARE(reloaded.connections().size(), 1);
    QVERIFY(findById(reloaded.directoryNodes(), "d2").isEmpty());
    QVERIFY(!findById(reloaded.directoryNodes(), "d1").isEmpty());
}

void PermissionConfigControllerTest::testPermissionLabelRoundTrip()
{
    PermissionConfigController controller;

    QVariantList nodes;
    nodes.append(makeDirectoryNode("d1", "/repo", 0.0, 0.0));
    // Canonical "Word / Word / Word" form must round-trip byte-for-byte.
    nodes.append(makeRoleNode("r1", "team-a", "Read / Write / Lock", 0.0, 0.0));
    // Sloppier input (no spaces, lowercase) must normalize to the same
    // canonical form on the way back out.
    nodes.append(makeRoleNode("r2", "team-b", "read/write", 0.0, 0.0));

    QVariantList connections;
    connections.append(makeConnection("d1", "r1"));
    connections.append(makeConnection("d1", "r2"));

    QVERIFY(controller.saveConfig(nodes, connections));

    QCOMPARE(findById(controller.roleNodes(), "r1").value("permissionLabel").toString(),
             QStringLiteral("Read / Write / Lock"));
    QCOMPARE(findById(controller.roleNodes(), "r2").value("permissionLabel").toString(),
             QStringLiteral("Read / Write"));

    // Re-saving the already-normalized labels must be a stable fixed point
    // (no further drift on a second save/load cycle).
    QVariantList normalizedNodes;
    normalizedNodes.append(makeDirectoryNode("d1", "/repo", 0.0, 0.0));
    normalizedNodes.append(makeRoleNode("r1", "team-a", "Read / Write / Lock", 0.0, 0.0));
    normalizedNodes.append(makeRoleNode("r2", "team-b", "Read / Write", 0.0, 0.0));
    QVERIFY(controller.saveConfig(normalizedNodes, connections));

    QCOMPARE(findById(controller.roleNodes(), "r1").value("permissionLabel").toString(),
             QStringLiteral("Read / Write / Lock"));
    QCOMPARE(findById(controller.roleNodes(), "r2").value("permissionLabel").toString(),
             QStringLiteral("Read / Write"));
}

QTEST_GUILESS_MAIN(PermissionConfigControllerTest)
#include "tst_permissionconfigcontroller.moc"
