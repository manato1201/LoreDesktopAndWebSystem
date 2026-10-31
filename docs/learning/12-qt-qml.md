# 12. Qt / QML: デスクトップアプリ

**この章で分かること**: LoreForge Client と Server Admin が、どう作られているか。QML の基本、テーマ(`Theme`)の仕組みと「黙って壊れる」罠、C++ のモデルと QML のつなぎ方、ネットワークと Cookie、そして**実際に起きた「ログインできたのに一覧が出ない」**の原因。

前提: [01](01-web-basics.md)(Cookie)。C++ の基本(クラス・ポインタ)が分かると読みやすいです。Qt は初めてで構いません。

## 12.1 QML は「宣言する」言語

QML は、画面を**手順ではなく、状態として書く**言語です。[`PrimaryButton.qml`](../../loreforge-client/qml/PrimaryButton.qml) を読みます。

```qml
import QtQuick
import LoreForge                         // このアプリのモジュール(Theme などが入っている)

Rectangle {
    id: button
    property string text: ""             // 外から渡せる値(プロパティ)
    property bool busy: false
    signal clicked()                      // 外へ知らせるイベント(シグナル)

    implicitHeight: 40
    radius: Theme.radiusStandard
    color: !button.enabled ? Theme.colorSurfaceInteractive
         : mouseArea.pressed ? Theme.colorAccentBorder : Theme.colorAccent

    Text { text: button.busy ? "..." : button.text; color: Theme.colorBackgroundBase }
    MouseArea { id: mouseArea; anchors.fill: parent; onClicked: button.clicked() }
}
```

| 概念 | 意味 |
|---|---|
| **プロパティ** | 部品の状態。`property string text: ""` で自分の項目を足せる |
| **バインディング** | `color: cond ? A : B` のような**式**を書くと、**依存する値が変わるたびに自動で再計算**される(`mouseArea.pressed` が変われば色が変わる)。`if` で手で更新しなくてよい |
| **シグナル** | `signal clicked()`。使う側が `onClicked: ...` で受け取る |
| **`id`** | 同じファイル内で、他の部品を名前で参照する |
| **`anchors`** | 親や兄弟に対する位置の指定(`anchors.fill: parent` は親いっぱい) |

**考え方**: 「ボタンを押されたら色を変える」を**手続きで書かず**、「`color` は、押されているかに**応じて**こう決まる」と**関係**で書きます。UI の状態管理が、とても簡単になります。

## 12.2 `Theme`: シングルトンと、黙って壊れる罠

色や余白は、`Theme.colorAccent` のように**名前で**参照します。`Theme` は**シングルトン**(アプリに1つだけ)です。

```qml
// qml/Theme.qml(生成物。直接編集しない — 11章)
pragma Singleton
import QtQuick
QtObject {
    readonly property color colorAccent: "#1ed760"
    readonly property int radiusStandard: 6
    /* ... */
}
```

**罠**: `pragma Singleton` と書くだけでは**足りません**。CMake でも、そのファイルがシングルトンだと**教える**必要があります。

```cmake
set_source_files_properties(qml/Theme.qml PROPERTIES
    QT_QML_SINGLETON_TYPE TRUE
)
```

これを忘れると、`Theme.colorAccent` が**エラーも出さずに `undefined`** になり、ボタンの色が出ない、という「黙った失敗」になります。**何も言わずに壊れる**類の不具合は、まず「**設定が足りないのでは**」を疑います。

`Theme.qml` は、`design-tokens/tokens.json` から**生成**されます(11章)。色を変えたいときは、`tokens.json` を直して `node tools/gen-tokens.mjs`。Web と Qt の両方が、同時に変わります。

## 12.3 C++ のデータを、QML に見せる: モデルとビュー

画面の一覧(リポジトリ、ファイルツリー、コミット)は、**C++ のモデル**が持ち、QML の `ListView` が表示します。

```
C++  RepositoryTreeModel (QAbstractListModel)   ←→   QML  ListView { model: treeModel; delegate: ... }
     ・行の数(rowCount)
     ・行の中身(data)
     ・役割の名前(roleNames)
```

[`RepositoryTreeModel.h`](../../loreforge-client/src/RepositoryTreeModel.h) の要点:

```cpp
class RepositoryTreeModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT                                         // QML から型として使える
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)  // 読める・変わったら通知する
public:
    enum Roles { PathRole = Qt::UserRole + 1, NameRole, KindRole, /* ... */ };
    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;       // 役割の番号 → QML での名前
    Q_INVOKABLE void loadRepository(const QString &slug);     // QML から呼べる関数
};
```

- **`roleNames`**: 行の各項目(`PathRole` など)に、QML で使う名前(`path`)を付ける。デリゲート(1行の見た目)の中で `model.path` のように読める。
- **`Q_PROPERTY(... NOTIFY ...)`**: これで QML のバインディングが**値の変化を追える**。`NOTIFY` を忘れると、値が変わっても画面が更新されない。
- **`Q_INVOKABLE`**: QML のボタンから `treeModel.loadRepository("audio-master")` と呼べる。
- **`beginResetModel()` / `endResetModel()`**: 中身を丸ごと入れ替えるときに挟む。QML に「全部変わった」と知らせる。

ファイルツリーは入れ子ですが、`ListView` は平らな一覧です。そこで、**開いているフォルダの中身だけを平らにして**行にしています(`m_flatRows`)。木を平らな列にする、というアルゴリズムです([10章](10-algorithms.md) の `build_tree` の逆向き)。

## 12.4 ネットワーク: 1つの `QNetworkAccessManager` を共有する

HTTP は、`QNetworkAccessManager`(以下 QNAM)で行います。**Cookie 保管庫(Cookie jar)は QNAM の持ち物**です。

```cpp
// ApiClient.cpp — アプリ全体で1つ
QNetworkAccessManager &ApiClient::networkManager()
{
    static QNetworkAccessManager manager;      // 関数の中の static: 最初に呼ばれたとき1回だけ作る
    return manager;
}
```

**全部のコントローラが、この共有の QNAM を通る**必要があります。別々に作ると、**空の Cookie 保管庫が複数**でき、ログインで受け取った Cookie が、他の要求に付きません(ログイン画面は通るのに、次の画面で 401)。これが `ApiClient.h` の長いコメントの意味です。

非同期の書き方(Qt は同期で待たない):

```cpp
QNetworkReply *reply = ApiClient::networkManager().get(request);
connect(reply, &QNetworkReply::finished, this, [this, reply]() {      // 終わったら、この関数が呼ばれる
    if (reply->error() != QNetworkReply::NoError) {
        setErrorMessage(reply->errorString());                         // 失敗の文言
        reply->deleteLater();                                           // 応答の後片付け(必ず)
        return;
    }
    /* 成功: JSON を読む */
    reply->deleteLater();
});
```

- **`finished` シグナル + ラムダ**: 待たずに、結果が来たときの処理を登録する(JavaScript の `fetch().then()` に近い)。
- **`deleteLater()`**: `reply` は自分で解放する決まり。忘れるとメモリが漏れる。

## 12.5 実例: 「ログインできたのに、一覧が出ない」

**症状**: LoreForge Client で**ログインはできる**のに、リポジトリ一覧が 0 件で、画面に「**Host requires authentication**」と出る。

**調べ方**:

1. エラー文言の出所を探す。`RepositoryListModel::refresh()` の `setErrorMessage(reply->errorString())` が、Qt の**標準の文言をそのまま**出していた。つまり、アプリが書いた文ではなく、Qt が作った文。
2. その文言は、HTTP の **401**(認証されていない)を、Qt が `AuthenticationRequiredError` に変換したときの標準の文。つまり、**サーバーは「Cookie が無い」と言っている**。
3. なぜ Cookie が無い? ログインは成功しているのに。→ **Cookie が保存されていない**。
4. サーバーの `Set-Cookie` を見ると `Secure` が付いている。**`Secure` の Cookie は HTTP の接続では保存されない**。このアプリは `http://localhost:4000`(平文の HTTP)に接続する。→ 原因。

**直し方**: サーバーを `LOREHUB_INSECURE_COOKIES=true` で起動する(`Secure` を外す)。公開用の Web も同じサーバーで扱うなら、`LOREHUB_WEB_ORIGIN` と**両方**を設定する([DEPLOYMENT.md](../DEPLOYMENT.md))。

**教訓**:

- **エラー文言を、アプリの文だと思い込まない。** どこから出た文かを、コードで探す。
- 「**ログインできる**」と「**その後の要求に Cookie が付く**」は別の話。後者を確かめる(`curl -i` で `Set-Cookie` を見る)。
- 設定(`Secure`)は**接続の種類(HTTP/HTTPS)**と整合させる。

## 12.5.1 このアプリの限界(知っておく)

- 接続先は `ApiClient::baseUrl()` が **`http://localhost:4000` に固定**。別のサーバーに向けるには、設定にする改修が要る(演習)。
- 期限切れ(401)の自動更新は、一覧のモデルには無く、**25分ごとの先回りの更新**(セッション自動延長)に頼っている。Web の `fetchWithRefresh`(401 を受けてから更新)とは方式が違う([講義資料 Chapter 7](../LECTURE_NOTES.md))。

## 12.6 ビルドとテスト(Windows)

Qt のビルドは、**MSVC の環境**を整えてから行います。

```bat
rem 1) MSVC の環境を読み込む(Visual Studio の場所は、環境により違う)
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat" x64

rem 2) ビルド(既に構成済みの build/ を使う場合)
cd loreforge-client
cmake --build build

rem 3) テスト(画面を出さない設定で)
set QT_QPA_PLATFORM=offscreen
set PATH=<Qt のフォルダ>\6.8.3\msvc2022_64\bin;%PATH%
ctest --test-dir build --output-on-failure
```

- 初回は `cmake --preset default`(Debug)または `--preset release` で構成する(`CMakePresets.json`)。
- `windeployqt` が、実行に必要な Qt の DLL を集める(ビルドの最後に自動)。配布用インストーラーは [DESKTOP_PACKAGING.md](../DESKTOP_PACKAGING.md)。
- 実際の検証: トークン生成に変えた `Theme.qml` で、**両アプリをビルドし、Qt Test が通る**ことを確認しています(`RepositoryTreeModelTests`、`PermissionConfigControllerTests`)。

## 12.7 まとめ

- QML は**状態の関係**を宣言する。バインディングが、更新を自動にする。
- `Theme` はシングルトン。**CMake での指定も必要**で、忘れると**黙って undefined**。
- C++ のモデル(`QAbstractListModel`)を、`roleNames`・`Q_PROPERTY(NOTIFY)`・`Q_INVOKABLE` で QML に見せる。
- HTTP は**共有の** `QNetworkAccessManager`(Cookie 保管庫のため)。結果は `finished` シグナルで受け、`deleteLater()` する。
- 「ログインできたのに 401」は、**`Secure` の Cookie が HTTP で保存されなかった**ことが原因だった。

## 12.8 演習

1. `PrimaryButton.qml` の `Theme.colorAccent` を、`"#ff0000"` に**直書き**して実行する(ビルドして目で見る)。それから、**`tokens.json` を直して生成**する正しい方法で同じことをする。違いは?
2. `CMakeLists.txt` の `QT_QML_SINGLETON_TYPE TRUE` を**コメントアウト**して、ビルド・実行する。何が起きるか(何も言わずに色が消える)を確かめて、元に戻す。
3. `RepositoryTreeModel` の `rowCount` と `data` を読み、`roleNames` の役割名が QML の `delegate` でどう使われているかを、`qml/RepositoryWorkspaceScreen.qml` で探す。
4. `ApiClient::baseUrl()` を、環境変数 `LOREHUB_API_URL` があればそれを使う、に変える方針を書く(`qEnvironmentVariable`)。テストはどう書く?
5. **再現**: `LOREHUB_INSECURE_COOKIES` を**付けずに** API を起動し、`curl -i` でログインの `Set-Cookie` に `Secure` が付くのを確かめる。次に、付けて起動して、`Secure` が消えるのを確かめる。Qt のアプリで一覧が出る/出ないが変わることも確かめる。
6. **設計**: Qt 側にも、Web の `ApiError` に相当する「ステータスを持つエラー」を導入するとしたら、`QNetworkReply` のどの情報(`attribute(QNetworkRequest::HttpStatusCodeAttribute)`)を使うか。403 のときに、画面に何を出すか。

前: [11. スクリプト・テスト・CI](11-tooling-and-ci.md) / 次: [13. 拡張レシピ集](13-extend-recipes.md)
