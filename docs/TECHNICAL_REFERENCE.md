# Lore Ecosystem — 技術資料

対象読者: このリポジトリで実装を引き継ぐ/レビューするエンジニア。設計思想ではなく「動かし方」「APIの正確な仕様」「詰まりやすい箇所」を扱う。

## 1. リポジトリ構成

```
LoreDesktopAndWebSystem/
├── ARCHITECTURE.md          # システム設計の一次情報源
├── DESIGN.md                 # デザイントークン(Web/Qt共通)
├── LOREHUB_UI_SPEC.md         # LoreHub Web の画面設計
├── QUALITY_STANDARDS.md       # コーディング/QA基準
├── docs/                      # 本ドキュメント一式
├── lorehub-api/                # Rust (Axum) バックエンド
│   └── src/
│       ├── main.rs             # ルーティング / CORS / サーバ起動
│       ├── handlers.rs         # 各エンドポイントのハンドラ
│       ├── state.rs             # AppState定義・シード生成
│       ├── db.rs                 # SQLite kv_store 永続化
│       ├── auth.rs                # パスワードハッシュ/セッション
│       ├── models.rs               # シリアライズ用データ型
│       └── image_assets.rs          # 画像プレビュー生成
├── lorehub-web/                 # Next.js フロントエンド
│   └── src/
│       ├── app/(app)/              # 認証必須ルートグループ
│       ├── app/login/               # ログイン画面
│       ├── components/                # UIコンポーネント群
│       └── lib/api.ts, auth-server.ts # APIクライアント/Cookie転送
├── loreforge-client/              # Qt6/QML デスクトップクライアント
│   ├── qml/                         # 画面・コンポーネント
│   └── src/                          # C++コントローラ/モデル
└── loreforge-server-admin/         # Qt6/QML サーバ管理アプリ
    ├── qml/
    └── src/
```

## 2. 開発環境セットアップ

### 2.1 lorehub-api (Rust)

```bash
cd lorehub-api
cargo run
# -> http://127.0.0.1:4000 で待ち受け
# 初回起動時は state::seed() でデモデータを生成し lorehub.db に保存
```

Lint/検証:
```bash
cargo fmt
cargo clippy
```

### 2.2 lorehub-web (Next.js)

```bash
cd lorehub-web
npm install
npm run dev      # http://localhost:3000 (Turbopack)
npm run lint      # eslint
npm run build      # 本番ビルド + 型チェック
```

環境変数 `NEXT_PUBLIC_API_URL` 未設定時は `http://localhost:4000` にフォールバック(`src/lib/api.ts`)。

### 2.3 loreforge-client / loreforge-server-admin (Qt6/C++20, Windows/MSVC)

```bat
:: vcvarsall.bat で MSVC 環境を有効化してから
cmake --preset default
cmake --build --preset default
```

Git Bash から呼ぶ場合は `MSYS_NO_PATHCONV=1` を付与しないと `cmd.exe /c` のパス変換で失敗する。`.bat` ファイルを書いて実行する方が `cmd.exe /c "vcvarsall && cmake ..."` の1行連結より安定する。

## 3. lorehub-api エンドポイント一覧

認証: `POST /api/auth/login` 以外は全て `require_auth` ミドルウェア配下(セッションCookie必須)。

| Method | Path | 説明 |
|---|---|---|
| POST | `/api/auth/login` | ログイン(公開エンドポイント) |
| POST | `/api/auth/logout` | ログアウト |
| GET | `/api/auth/me` | 現在のユーザー情報 |
| GET | `/api/repositories` | リポジトリ一覧 |
| POST | `/api/repositories` | リポジトリ作成 |
| GET | `/api/repositories/{slug}` | リポジトリ詳細 |
| PATCH | `/api/repositories/{slug}` | rename/description/visibility更新 |
| DELETE | `/api/repositories/{slug}` | リポジトリ削除(関連PRも削除) |
| GET | `/api/repositories/{slug}/tree` | ファイルツリー |
| POST | `/api/repositories/{slug}/tree/lock` | ファイルロック切替 |
| GET | `/api/repositories/{slug}/content/{*path}` | テキストファイル内容 |
| GET | `/api/repositories/{slug}/image/{*path}` | 画像プレビュー |
| GET | `/api/repositories/{slug}/image-before/{*path}` | 画像Before(diff用) |
| GET | `/api/repositories/{slug}/audio/{*path}` | 音声プレビュー(WAV) |
| GET | `/api/repositories/{slug}/commits` | コミット履歴(ブランチ情報込み) |
| POST | `/api/repositories/{slug}/commits` | ステージ済み変更からコミット作成 |
| GET | `/api/repositories/{slug}/commits/{hash}` | コミット詳細 |
| GET | `/api/repositories/{slug}/branches` | ブランチ一覧 |
| POST | `/api/repositories/{slug}/branches` | ブランチ作成 |
| GET | `/api/repositories/{slug}/branches/current` | 現在のブランチ名 |
| POST | `/api/repositories/{slug}/checkout` | ブランチ切替 |
| GET | `/api/repositories/{slug}/pending` | ステージ済み変更一覧 |
| POST | `/api/repositories/{slug}/tree/stage` | ファイルのステージ/アンステージ |
| GET | `/api/pulls?status=` | PR一覧(status絞り込み) |
| GET | `/api/pulls/{id}` | PR詳細+差分 |
| POST | `/api/pulls/{id}/comments` | PRへのコメント追加 |
| GET | `/api/access-control/entries` | パス別アクセス権限一覧 |
| POST | `/api/access-control/entries/toggle` | 権限トグル |
| PUT | `/api/access-control/entries` | 権限グラフの一括Apply(パスごとにマージ) |
| GET | `/api/org/members` | 組織メンバー一覧 |
| PATCH | `/api/org/members/{email}` | メンバーのロール変更 |
| GET | `/api/org/storage` | ストレージ使用量 |
| GET | `/api/org/audit-log` | 監査ログ |

### 3.1 PATCH /api/repositories/{slug} の仕様

リクエストボディ(全フィールド省略可、指定したもののみ更新):
```json
{ "name": "string?", "description": "string?", "visibility": "private|internal|public" }
```
- `name` を送る場合は trim後に空文字だと `400 Bad Request`
- 対象が存在しなければ `404 Not Found`
- 成功時は更新後の `Repository` を返す(`200 OK`)、監査ログに `"updated settings for"` を記録

### 3.2 DELETE /api/repositories/{slug} の仕様

- 対象リポジトリと、それに紐づく `pull_requests` を全て削除
- `seeded_repo_slugs` からも除去(空ツリー扱いの対象から外す)
- 成功時 `204 No Content`、存在しなければ `404 Not Found`
- 監査ログに `"deleted repository"` を記録

### 3.3 VCS書き込みAPI(ステージ・コミット・ブランチ)

`tree`/`commits`/`branches` は元々全リポジトリで共有される単一のグローバルデータだったが、書き込み操作を導入するにあたりリポジトリごとの `HashMap<slug, Vec<T>>` へリファクタリングした(GETレスポンスの形状は変更なし、単なる内部ストレージの変更)。

- `POST /api/repositories/{slug}/tree/stage` — body: `{ "path": "...", "changeType": "added"|"modified"|"deleted", "staged": true|false }`。`toggle_lock` と同じパターンでパスごとの保留変更を追加/削除。
- `POST /api/repositories/{slug}/commits` — body: `{ "message": "...", "description": "" }`。保留変更が空なら `400`。`rand` crateで40文字の16進フェイクハッシュを生成し、`Commit` を該当ブランチの末尾に追加、ブランチの `head` を更新、保留変更をクリア。
- `POST /api/repositories/{slug}/branches` — body: `{ "name": "...", "from": "main" }`。`from` 省略時は現在のブランチ。既存名なら `400`。
- `POST /api/repositories/{slug}/checkout` — body: `{ "branch": "..." }`。`current_branch` (repo slug単位のHashMap) を更新。

### 3.4 PUT /api/access-control/entries の仕様

body: `HashMap<path, Vec<AccessEntry>>`(`GET` と同じ形状)。**全置換ではなくパスごとのマージ**(insert-or-overwrite) — リクエストボディに含まれないパスの既存データは一切変更しない。監査ログに `"applied access control configuration from Server Admin"` を1エントリ記録。Server Adminのノードエディタからのapplyで使用。

## 4. 認証・セッションの実装詳細

- パスワードは argon2 でハッシュ化。デモアカウントは全員パスワード `"lorehub"` 共通(`state.rs` コメント参照)。
- ログイン成功時、`Set-Cookie: lorehub_token=<token>; HttpOnly; SameSite=Lax; Max-Age=604800`
- Cookieは `localhost` ドメインに対して発行されるため、ポートが異なる `localhost:3000`(Web) と `localhost:4000`(API) の両方に自動送信される(ブラウザのCookieはホスト単位でありポート単位ではない)。
- Next.jsのServer Componentはブラウザとは別プロセスなのでCookieジャーを持たない。`src/lib/auth-server.ts` の `getSessionCookieHeader()` が `next/headers` の `cookies()` から手動で取得し、API呼び出し時にヘッダとして転送する。

## 5. 永続化: kv_store 方式

`lorehub-api/src/db.rs` は `AppState` の各フィールドを個別のJSONブロブとして1テーブル(`kv_store: key TEXT, value TEXT`)に保存する。

利点: Rustの構造体をそのまま `serde_json::to_string` / `from_str` でき、マイグレーション不要。
制約: SQLでの複雑な検索・集計はできない(全件ロードしてRust側でフィルタする設計)。デモ〜小規模組織のデータ量を前提とした意図的なトレードオフ。

`load_state` は保存されたキーが一部欠けていても(例: 過去バージョンのDBに `seeded_repo_slugs` が無い場合)フォールバックして起動できるようになっている。

## 6. Qt/QML 実装上の注意点(既知の落とし穴)

| 症状 | 原因 | 対処 |
|---|---|---|
| QML singletonが `undefined` | `pragma Singleton` だけでは不十分 | `set_source_files_properties(...QT_QML_SINGLETON_TYPE TRUE)` を `qt_add_qml_module()` 前に追加 |
| `qmltyperegistrations.cpp` がカスタム型を見つけない | C++型が `qt_add_executable` 側にある | `qt_add_qml_module(...)` の `SOURCES` に移動 + `target_include_directories` 追加 |
| QMLバインディングループでプロパティが `undefined` | 内側の `id` と外側のプロパティ名が衝突 | idをリネーム(例: `repositoryModel` → `repositoryModelInstance`) |
| `QStringLiteral(定数)` がコンパイルエラー | マクロはリテラルトークンが必要、`const char*` 不可 | `QLatin1String` に置き換え |
| スクリーンショットが無関係なウィンドウを写す | `CopyFromScreen` が古い座標をキャッシュ | 撮影直前に `GetWindowRect` を再取得 + `SetForegroundWindow` |
| `QProcess::stop()` 相当のはずが `errorOccurred(Crashed)` を発火 | `terminate()` はウィンドウを持たないプロセスへは無効(`WM_CLOSE`相当が届かない)、3秒後の `kill()` フォールバックが実質毎回発動 | `errorOccurred` ハンドラで `FailedToStart` 以外は致命扱いしない。`Stopped` 経路で `lastError` をクリア |
| Qt標準 `TabBar`/`TabButton` がダークテーマに追従しない | アクティブスタイルが `background:` オーバーライドを無視(コンソールに "style does not support customization" 警告) | 手組みのPill型タブ(`Rectangle` + `Text` + `MouseArea`、Theme色を直接バインド)に置き換え |
| 認証付き画像がQMLの `Image` で読み込めない(401) | `Image { source: }` はデフォルトのQNetworkAccessManagerを使い、Cookie jarを共有しない | `QQuickAsyncImageProvider` を実装し、共有 `ApiClient::networkManager()` 経由でフェッチ、`image://<provider>/...` で公開 |
| `QQuickAsyncImageProvider::requestImageResponse()` からのネットワークアクセスがスレッド違反でクラッシュ/無反応 | ワーカースレッドで呼ばれるが `QNetworkAccessManager` はGUIスレッド専属 | レスポンスオブジェクトを `moveToThread()` でGUIスレッドへ移し、`QMetaObject::invokeMethod(..., Qt::QueuedConnection)` 経由で `start()` を呼ぶ |
| 検証用QMLの `console.log`/`console.error` がリダイレクトログに出ない | リダイレクト先がファイルだとフルバッファリングになり、プロセス終了までフラッシュされない | C++側に `Q_INVOKABLE` なロガーを用意し `fprintf(stderr, ...)` + `fflush(stderr)` で出力(QML側は `logger.log(...)` を呼ぶだけ) |
| 複雑な画面遷移(ログイン→一覧→詳細)を伴う検証用Timerチェーンが理由不明で無反応 | 原因未特定(QMLの id 解決タイミング等の可能性) | デバッグに沈まず、検証したいC++型だけを直接インスタンス化する専用の一時QMLファイルに切り替える(`main.cpp` の `loadFromModule` を一時的に差し替え) |
| `vcvarsall.bat` が見つからないと突然失敗する | 開発環境のVisual Studioインストールが `...\2022\Community\...` から `...\18\Community\...` へ自動更新されていた(セッション中に発生) | ハードコードせず `find "C:\Program Files\Microsoft Visual Studio" -iname vcvarsall.bat` 等で都度確認 |

## 7. 既知の制約 / 今後の課題

- LoreForge ClientのVCS操作はlorehub-apiへの直接書き込みで完結しており、`push`と`commit`の区別がない(ローカル/リモートの分離が存在しないため — 詳細はARCHITECTURE_AND_DESIGN.md §3.2)。
- LoreForge Server AdminのMinIO Docker制御(`DockerController`)はコード実装済みだが、この開発環境にDockerが無いため実機検証ができていない。
- LoreForge Clientの3Dモデルdiffビューアはスタイライズされた代替表現であり、実際のFBX/OBJ等のモデルローダーは未実装(Web版の3Dビューアと同じ意図的な簡略化)。
- Server Adminのノードエディタはディレクトリ/ロールの追加・削除UIが無く、既定の5+3ノード構成が前提。
- 認証は単一セッショントークン方式で、リフレッシュトークンやトークン失効APIは未実装。
- `kv_store` 方式は将来的にリレーショナルスキーマへ移行する余地を残す(現状はデータ量的に不要と判断)。
