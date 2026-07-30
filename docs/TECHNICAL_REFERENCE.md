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
cargo test    # 統合テスト59件。各テストが独立した sqlite://:memory: を使うため実DBには触れない
```

#### 環境変数(セキュリティ関連、公開デプロイ向け)

| 変数 | デフォルト | 説明 |
|---|---|---|
| `LOREHUB_INSECURE_COOKIES` | 未設定(=`false`) | `true`/`1` を明示的に設定した場合のみ、発行する4種のCookie(`session_cookie`/`refresh_cookie`/`cleared_session_cookie`/`cleared_refresh_cookie`、`src/auth.rs`)から `Secure` 属性を外す。**未設定時はデフォルトで安全側**(`Secure` 属性つき)に倒す設計 — 明示的なopt-outを忘れてもセキュアな挙動になる。**ローカルで `cargo run` を `http://localhost:4000` (平文HTTP)のまま使う場合はこの変数が必須**: ブラウザは非HTTPSレスポンスで発行された `Secure` Cookieを保存しないため、これを設定し忘れるとログイン自体は`200`を返すのにCookieがブラウザに保存されず、以降のリクエストが常に401になる(症状だけ見るとログインが壊れているように見える罠なので注意)。 |
| `LOREHUB_WEB_ORIGIN` | `http://localhost:3000` | CORSの `Access-Control-Allow-Origin` に使う単一オリジン(`allow_credentials(true)` のためワイルドカード不可)。値を明示的に設定したのに `HeaderValue` としてパースできない場合は起動時に `panic`(起動時設定ミスとして扱い、黙ってデフォルトにフォールバックしない)。 |
| `LOREHUB_MAX_BODY_BYTES` | `50331648`(48MiB) | `tower_http::limit::RequestBodyLimitLayer` に渡すリクエストボディの上限バイト数。超過時は `413 Payload Too Large`。 |

`LOREHUB_INSECURE_COOKIES=true cargo run` のようにインライン指定するか、`.env`相当の仕組みでシェルにexportしてから起動する。

#### ログインのレート制限

`POST /api/auth/login` は送信元IPアドレス単位(メールアドレス単位ではない — メール単位だと攻撃者が正規ユーザーのメールアドレスへ故意に失敗リクエストを送りつけてアカウントをロックできてしまう)でレート制限される(`tower_governor` クレート、`main.rs::build_router` の `login_route`)。デフォルトはバースト8回、以降60秒に1回のペースで補充(`RouterConfig::from_env` のデフォルト値、環境変数化はしていない)。超過時は `429 Too Many Requests`。サーバーが送信元IPを見るためには `axum::serve` を `into_make_service_with_connect_info::<SocketAddr>()` 経由で起動する必要がある(素の `axum::serve(listener, app)` では `ConnectInfo` が配線されず機能しない)。

#### アップロードのボディサイズ上限に関する既知の制約

`POST /api/repositories/{slug}/upload` はファイル実体をbase64文字列としてJSONボディに埋め込む方式(§3.5参照)であり、base64のオーバーヘッドによりエンコード後のボディサイズは生バイト数の約1.33倍になる。`LOREHUB_MAX_BODY_BYTES` のデフォルト48MiBは生データ換算で約36MiBに相当する。この方式は `ARCHITECTURE.md` が本来想定する「数十GBの巨大バイナリアセット」(チャンク分割 + MinIO格納 + Chunk-based Streaming Viewer、`ARCHITECTURE.md` の該当節参照)のシナリオには構造的に不向きである — 今回追加した上限はその根本的な設計上の制約を隠さず可視化するものであり、この制約自体を解消するには実際のチャンク/マルチパート/ストリーミングアップロード経路の導入が必要(本タスクのスコープ外、既知の課題として残す)。

### 2.2 lorehub-web (Next.js)

```bash
cd lorehub-web
npm install
npm run dev      # http://localhost:3000 (Turbopack)
npm run lint      # eslint
npm run build      # 本番ビルド + 型チェック
npm run test       # Vitest。unit test 22件(node環境、jsdom不要)
```

環境変数 `NEXT_PUBLIC_API_URL` 未設定時は `http://localhost:4000` にフォールバック(`src/lib/api.ts`)。

`lorehub-api` 側を素の `cargo run`(平文HTTP、環境変数未設定)で動かしている場合、`lorehub-web` からのログインが一見成功したように見えて実際にはCookieが保存されずセッションが維持されない(ブラウザが非HTTPS応答の `Secure` Cookieを保存しないため)。この場合は `lorehub-api` 起動時に `LOREHUB_INSECURE_COOKIES=true` を設定すること(§2.1参照、`lorehub-web` 側に対応する環境変数は無い)。

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
| POST | `/api/auth/login` | ログイン(公開エンドポイント)。アクセス+リフレッシュの2Cookieを発行 |
| POST | `/api/auth/refresh` | リフレッシュトークンをローテーションし新しいトークン対を発行(公開エンドポイント) |
| POST | `/api/auth/logout` | ログアウト(両トークンを失効) |
| GET | `/api/auth/me` | 現在のユーザー情報 |
| GET | `/api/repositories` | リポジトリ一覧 |
| POST | `/api/repositories` | リポジトリ作成 |
| GET | `/api/repositories/{slug}` | リポジトリ詳細 |
| PATCH | `/api/repositories/{slug}` | rename/description/visibility更新 |
| DELETE | `/api/repositories/{slug}` | リポジトリ削除(関連PRも削除) |
| GET | `/api/repositories/{slug}/tree` | ファイルツリー |
| POST | `/api/repositories/{slug}/upload` | 画像/テキスト/音声ファイルの実バイトをアップロード(リポジトリ単位で分離、種別はpathの拡張子から推定) |
| POST | `/api/repositories/{slug}/tree/lock` | ファイルロック切替 |
| GET | `/api/repositories/{slug}/content/{*path}` | テキストファイル内容(`text/plain`直返し、HTTP Range対応) |
| GET | `/api/repositories/{slug}/image/{*path}` | 画像プレビュー(HTTP Range対応) |
| GET | `/api/repositories/{slug}/image-before/{*path}` | 画像Before(diff用) |
| GET | `/api/repositories/{slug}/audio/{*path}` | 音声プレビュー(WAV、HTTP Range対応) |
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

### 3.5 アップロードの種別推定とHTTP Range対応

`POST .../upload` は body `{ "path": "...", "contentBase64": "..." }` を受け取り、`path` の拡張子から種別を推定して `uploaded_images`/`uploaded_text`/`uploaded_audio`(いずれも `HashMap<slug, HashMap<path, Vec<u8>>>`、リポジトリ単位で分離)のいずれかへ保存する(`.txt`/`.md`/`.json`/`.yaml`/`.yml` → text、`.wav`/`.mp3` → audio、それ以外 → image)。3Dモデルは意図的に対象外(§7参照)。

`get_image`/`get_image_before`/`get_audio`/`get_file_content` は共通の `ranged_bytes_response` ヘルパーを通り、`Range: bytes=<start>-<end>` リクエストヘッダを解釈する:
- ヘッダ無し → `200 OK`、全量を返す(`Accept-Ranges: bytes` を追加)
- 充足可能な範囲 → `206 Partial Content` + `Content-Range: bytes <start>-<end>/<total>`
- 範囲外 → `416 Range Not Satisfiable` + `Content-Range: bytes */<total>`

## 4. 認証・セッションの実装詳細

- パスワードは argon2 でハッシュ化。デモアカウントは全員パスワード `"lorehub"` 共通(`state.rs` コメント参照)。
- ログイン成功時、2つのCookieを発行: `lorehub_token`(アクセストークン、`Max-Age=1800`=30分)と `lorehub_refresh`(リフレッシュトークン、`Max-Age=604800`=7日)。両方とも `HttpOnly; SameSite=Lax`。
- サーバー側は `AppState.sessions`/`refresh_tokens: HashMap<token, SessionEntry{email, expires_at}>` で有効期限を管理。`require_auth` はアクセストークンの `expires_at` を毎回チェックし、期限切れなら失効エントリを削除して401を返す。
- `POST /api/auth/refresh` はリフレッシュトークンを検証し、**ローテーション**(古いリフレッシュトークンを破棄し新しいアクセス+リフレッシュ両方を再発行)する。古いリフレッシュトークンの再利用は401になる(盗用されたトークンの使い回しを防ぐ標準的な対策)。
- Cookieは `localhost` ドメインに対して発行されるため、ポートが異なる `localhost:3000`(Web) と `localhost:4000`(API) の両方に自動送信される(ブラウザのCookieはホスト単位でありポート単位ではない)。
- Next.jsのServer Componentはブラウザとは別プロセスなのでCookieジャーを持たない。`src/lib/auth-server.ts` の `getSessionCookieHeader()` が `next/headers` の `cookies()` から手動で取得し、API呼び出し時にヘッダとして転送する。
- **透過的リフレッシュ(lorehub-web)**: CSR(ブラウザからの直接fetch)は `src/lib/api.ts` の `fetchWithRefresh` が401を検知して1回だけ `/api/auth/refresh` を叩きリトライする(同時に複数リクエストが401した場合もリフレッシュ呼び出しは1回に共有され、リトライも無限ループしない)。SSR(Server Component)はCookieを書き換えられないため、`src/proxy.ts`(このNext.jsバージョンで `middleware.ts` から改名された規約 — `node_modules/next/dist/docs/01-app/03-api-reference/03-file-conventions/proxy.md` 参照)がレンダリング前にアクセスCookie欠落を検知して先回りでリフレッシュする。
- **プロアクティブリフレッシュ(LoreForge Client / Server Admin)**: `AuthController`(Client)と `PermissionConfigController`(Server Admin)はどちらもログイン成功時に `QTimer`(25分間隔、定数 `kRefreshIntervalMs`)を起動し、30分のアクセストークンTTLより先に `POST /api/auth/refresh` を送る。リフレッシュトークンは共有 `QNetworkAccessManager` のCookie jarに既に保持されているため、追加の配線は不要。リフレッシュが失敗した場合(リフレッシュトークン自体の失効・サーバーダウン)はタイマーを止めてログアウト/未接続状態へ遷移する。個々のリクエストへ401リトライを後付けするより単純で、デスクトップアプリのセッションライフサイクルに適した設計として意図的に選択(Web版のリアクティブ方式とは異なる)。

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
- LoreForge Clientの3Dモデルdiffビューアはスタイライズされた代替表現であり、実際のFBX/OBJ等のモデルローダーは未実装(Web版の3Dビューアと同じ意図的な簡略化)。同じ理由でアセットアップロードも3Dモデルは対象外(反映先が存在しないため)。
- `kv_store` 方式は将来的にリレーショナルスキーマへ移行する余地を残す(現状はデータ量的に不要と判断)。
- LoreForge Client/Server AdminにもQt Testによる単体テストを導入済み(`loreforge-client/tests/tst_repositorytreemodel.cpp`、`loreforge-server-admin/tests/tst_permissionconfigcontroller.cpp`)。GUI操作の自動化(`SendInput`等)はこのサンドボックスでは信頼できないため対象外とし、`RepositoryTreeModel`のツリー構築/Sparse Workspace Managerのinclude/exclude/cascade-exclude/ステージ管理、`PermissionConfigController`のJSON永続化ラウンドトリップなど、GUI非依存の純粋なモデル/コントローラロジックのみを対象にしている。各`CMakeLists.txt`に`enable_testing()`と`LoreForgeClientTests`/`LoreForgeServerAdminTests`ターゲットを追加し、`main.cpp`を除いた対象`.cpp`をテスト実行ファイルへ直接コンパイル(`QTEST_GUILESS_MAIN`が独自の`main()`を提供)。`ctest`(各`build/`ディレクトリ内)で実行できる。
