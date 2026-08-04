# Lore Ecosystem — 技術資料

対象読者: このリポジトリで実装を引き継ぐ/レビューするエンジニア。設計思想ではなく「動かし方」「APIの正確な仕様」「詰まりやすい箇所」を扱う。

## 1. リポジトリ構成

```
LoreDesktopAndWebSystem/
├── ARCHITECTURE.md          # システム設計の一次情報源
├── DESIGN.md                 # デザイントークン(Web/Qt共通)
├── LOREHUB_UI_SPEC.md         # LoreHub Web の画面設計
├── QUALITY_STANDARDS.md       # コーディング/QA基準
├── docs/                      # 本ドキュメント一式(DEPLOYMENT.md/DESKTOP_PACKAGING.mdもここ)
├── docker-compose.yml          # lorehub-api + lorehub-web の2サービス構成
├── .github/workflows/ci.yml     # cargo test/clippy/fmt + npm build/lint/test
├── lorehub-api/                # Rust (Axum) バックエンド
│   ├── Dockerfile                # マルチステージビルド(distroless実行イメージ)
│   ├── migrations/                # sqlx::migrate!() 適用対象。0001_vcs_schema.sqlがVCSスキーマの一次情報源
│   └── src/
│       ├── main.rs                 # ルーティング / CORS / レート制限 / メトリクス / サーバ起動
│       ├── handlers.rs              # 各エンドポイントのハンドラ
│       ├── authz.rs                  # パスACL解決(check_path_permission)
│       ├── state.rs                   # AppState定義・シード生成(非VCSデータのみ)
│       ├── db.rs                       # 永続化(非VCS: kv_storeブロブ)
│       ├── repo_store.rs                # リポジトリの正規化テーブルI/O
│       ├── vcs_store.rs                  # コミット/ブランチ/ツリー/ステージのVCS書き込み・読み取り
│       ├── lock_store.rs                  # ファイルロックの正規化テーブルI/O
│       ├── blob_store.rs                   # コンテンツアドレス方式のファイルシステムblob保存
│       ├── blob_meta_store.rs               # file_blobsテーブルI/O(メタデータ)
│       ├── content_hash.rs                   # SHA-256ハッシュ計算ヘルパー
│       ├── email.rs                           # 招待/パスワードリセットのSMTP送信(lettre)
│       ├── auth.rs                             # パスワードハッシュ/セッション/Cookie
│       ├── models.rs                            # シリアライズ用データ型
│       └── image_assets.rs                       # 画像プレビュー生成
├── lorehub-web/                 # Next.js フロントエンド
│   ├── Dockerfile                # マルチステージビルド(.next/standalone)
│   ├── next.config.ts             # output: standalone + クロスドメイン用rewrites()
│   └── src/
│       ├── app/(app)/              # 認証必須ルートグループ
│       ├── app/login/, accept-invite/, forgot-password/, reset-password/  # 公開ルート
│       ├── components/                # UIコンポーネント群
│       └── lib/api.ts, auth-server.ts # APIクライアント/Cookie転送
├── loreforge-client/              # Qt6/QML デスクトップクライアント
│   ├── qml/                         # 画面・コンポーネント
│   ├── src/                          # C++コントローラ/モデル
│   └── tests/                         # Qt Test(LoreForgeClientTests、ctestで実行)
└── loreforge-server-admin/         # Qt6/QML サーバ管理アプリ
    ├── qml/
    ├── src/
    └── tests/                        # Qt Test(LoreForgeServerAdminTests、ctestで実行)
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
cargo test    # 統合テスト118件。各テストが独立した sqlite://:memory: を使うため実DBには触れない
```

#### 環境変数(セキュリティ関連、公開デプロイ向け)

| 変数 | デフォルト | 説明 |
|---|---|---|
| `LOREHUB_INSECURE_COOKIES` | 未設定(=`false`) | `true`/`1` を明示的に設定した場合のみ、発行する4種のCookie(`session_cookie`/`refresh_cookie`/`cleared_session_cookie`/`cleared_refresh_cookie`、`src/auth.rs`)から `Secure` 属性を外す。**未設定時はデフォルトで安全側**(`Secure` 属性つき)に倒す設計 — 明示的なopt-outを忘れてもセキュアな挙動になる。**ローカルで `cargo run` を `http://localhost:4000` (平文HTTP)のまま使う場合はこの変数が必須**: ブラウザは非HTTPSレスポンスで発行された `Secure` Cookieを保存しないため、これを設定し忘れるとログイン自体は`200`を返すのにCookieがブラウザに保存されず、以降のリクエストが常に401になる(症状だけ見るとログインが壊れているように見える罠なので注意)。 |
| `LOREHUB_WEB_ORIGIN` | `http://localhost:3000` | CORSの `Access-Control-Allow-Origin` に使う単一オリジン(`allow_credentials(true)` のためワイルドカード不可)。値を明示的に設定したのに `HeaderValue` としてパースできない場合は起動時に `panic`(起動時設定ミスとして扱い、黙ってデフォルトにフォールバックしない)。 |
| `LOREHUB_MAX_BODY_BYTES` | `50331648`(48MiB) | `tower_http::limit::RequestBodyLimitLayer` に渡すリクエストボディの上限バイト数。超過時は `413 Payload Too Large`。 |
| `LOREHUB_LOG_FORMAT` | 未設定(=人間可読テキスト) | `json` を指定すると `tower_http::TraceLayer` が出す1リクエスト1行のアクセスログが構造化JSON(1行1オブジェクト)に切り替わる。ログ集約基盤(Loki/CloudWatch等)がフィールドを機械的にパースするための形式。人間可読形式は開発者がターミナルで直接読むこと前提。 |
| `LOREHUB_SMTP_HOST` / `_PORT` / `_USERNAME` / `_PASSWORD` / `_FROM` | 全て未設定 | 招待(`POST /api/org/invites`)・パスワードリセット(`POST /api/auth/forgot-password`)メールの送信元SMTPリレー(`lettre` crate、`src/email.rs`)。`LOREHUB_SMTP_HOST` が未設定の場合は実送信をスキップし、件名・本文(リンク込み)をINFOレベルでログ出力するだけになる——ローカル開発でSMTPサーバーを用意せずに招待/リセットフローを最後まで試せるフォールバック。`_PORT` はデフォルト587。ホストは設定済みだが値が不正(ポートがパースできない等)な場合は起動時にpanicする(`RouterConfig::from_env`と同じ「設定ミスは黙ってフォールバックせず起動時に落とす」方針)。メール送信自体の失敗(リレー到達不可・認証拒否等)はERRORログに記録して握りつぶされ、HTTPリクエスト自体は失敗させない——招待/リセットのトークンは`AppState`側で既に発行済みであり、メールは配送の利便性でしかないため。 |

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

#### クロスドメイン配置時のCookie問題(`NEXT_PUBLIC_USE_API_PROXY`)

ローカル開発とDocker Composeでは`lorehub-web`と`lorehub-api`は常に同一ホスト名(`localhost`、ポートのみ違う)経由でブラウザからアクセスされるため意識する必要が無いが、実際に`lorehub-web`をVercel等・`lorehub-api`を別ドメイン(トンネル等)へデプロイすると、`lorehub-api`が発行するセッションCookieは**API自身のドメインにしか保存されない**——`lorehub-web`のドメインへのリクエストには一切送信されず、CORSやSameSiteの設定では解決できない(そもそもCookieがフロントエンドのドメインに保存されていないため)。

対処は「ブラウザがバックエンドの実ドメインへ直接触れないようにする」こと: `src/lib/api.ts` の `API_BASE` は `NEXT_PUBLIC_USE_API_PROXY=true` の場合ブラウザ向けには常に相対パス `""` を返し、全ての `/api/...` 呼び出しは `lorehub-web` 自身のオリジンへ向かう。`next.config.ts` の `rewrites()` がサーバーサイドでそれを実バックエンド(`API_INTERNAL_URL`)へ転送するため、ブラウザが目にする`Set-Cookie`は常に`lorehub-web`自身のオリジンのものになる。**`next.config.ts`の`rewrites()`はビルド時に評価される**(実行時に毎回読まれるわけではない)ため、`API_INTERNAL_URL`は`npm run build`実行時点で設定済みである必要がある——コンテナ/プロセス起動時に後から設定しても反映されない。

当初は「`NEXT_PUBLIC_API_URL`を空文字にする」ことでプロキシモードを暗黙的に示す設計だったが、ホスティングダッシュボードによっては空文字を確実に保存できない(フィールドに前の値が残ったまま「クリアしたつもり」になりやすい)ことが実デプロイで判明し、明示的な`NEXT_PUBLIC_USE_API_PROXY=true`フラグへ置き換えた(`NEXT_PUBLIC_API_URL`の値に依存しない)。Docker Compose環境ではこのフラグは不要(未設定のまま)。

### 2.3 loreforge-client / loreforge-server-admin (Qt6/C++20, Windows/MSVC)

```bat
:: vcvarsall.bat で MSVC 環境を有効化してから
cmake --preset default
cmake --build --preset default
```

Git Bash から呼ぶ場合は `MSYS_NO_PATHCONV=1` を付与しないと `cmd.exe /c` のパス変換で失敗する。`.bat` ファイルを書いて実行する方が `cmd.exe /c "vcvarsall && cmake ..."` の1行連結より安定する。

## 3. lorehub-api エンドポイント一覧

認証: `POST /api/auth/login`/`refresh`/`forgot-password`/`reset-password`/`accept-invite`、`GET /api/auth/invite/{token}`、`GET /api/health`、`GET /metrics` 以外は全て `require_auth` ミドルウェア配下(セッションCookie必須)。さらにパス配下の読み書き系(tree/content/image/audio/upload/lock/stage/diff)は `authz::check_path_permission` によるACLチェックを、組織操作の一部(下表 †)はOwner/Adminロールチェックを追加で通過する必要がある(§4.1参照)。

| Method | Path | 説明 |
|---|---|---|
| GET | `/api/health` | 生存確認(公開・認証不要・DBアクセスなし)。Dockerの`HEALTHCHECK`が使用 |
| GET | `/metrics` | Prometheusテキスト形式のメトリクス(公開・認証不要。リクエスト数+レイテンシヒストグラムをmethod/route/statusで集計) |
| POST | `/api/auth/login` | ログイン(公開エンドポイント)。アクセス+リフレッシュの2Cookieを発行。送信元IP単位でレート制限(§2.1) |
| POST | `/api/auth/refresh` | リフレッシュトークンをローテーションし新しいトークン対を発行(公開エンドポイント) |
| POST | `/api/auth/logout` | ログアウト(両トークンを失効) |
| GET | `/api/auth/me` | 現在のユーザー情報 |
| POST | `/api/auth/change-password` | 自己サービスのパスワード変更(§3.7) |
| POST | `/api/auth/forgot-password` | パスワードリセットのメール送信要求(公開・常に204、§3.7) |
| POST | `/api/auth/reset-password` | リセットトークンで新パスワードを設定(公開、§3.7) |
| GET | `/api/auth/invite/{token}` | 招待プレビュー(公開、承諾前の確認用、§3.7) |
| POST | `/api/auth/accept-invite` | 招待を承諾しアカウント作成+自動ログイン(公開、§3.7) |
| GET / POST | `/api/org/invites` | 保留中の招待一覧取得 / 新規招待作成 †(§3.7) |
| DELETE | `/api/org/invites/{email}` | 招待の取り消し †(§3.7) |
| GET | `/api/repositories` | リポジトリ一覧 |
| POST | `/api/repositories` | リポジトリ作成 |
| GET | `/api/repositories/{slug}` | リポジトリ詳細 |
| PATCH | `/api/repositories/{slug}` | rename/description/visibility更新 |
| DELETE | `/api/repositories/{slug}` | リポジトリ削除(関連PRも削除) † |
| GET | `/api/repositories/{slug}/tree` | ファイルツリー |
| GET | `/api/repositories/{slug}/diff/{*path}?from=&to=` | 2コミット間の行レベルdiff(テキストのみ、§3.6) |
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
| PUT | `/api/access-control/entries` | 権限グラフの一括Apply(パスごとにマージ) † |
| GET | `/api/org/members` | 組織メンバー一覧 |
| PATCH | `/api/org/members/{email}` | メンバーのロール変更 † |
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
- 実際には `repositories` テーブルの行を1つ削除するだけ——全VCSテーブルの `repo_slug` 外部キーが `ON DELETE CASCADE`(`PRAGMA foreign_keys = ON` を接続ごとに有効化)で宣言されているため、`commits`/`branches`/`tree_entries`/`file_blobs`/`pending_changes`/`file_locks`等の関連行が同一操作で原子的にすべて消える。ファイルシステム上のblobディレクトリ(`{blob_base_dir}/blobs/{slug}/`)も後処理で削除する
- 成功時 `204 No Content`、存在しなければ `404 Not Found`
- 監査ログに `"deleted repository"` を記録

### 3.3 VCS書き込みAPI(ステージ・コミット・ブランチ)— 実コンテンツアドレス方式

`tree`/`commits`/`branches` は当初 `AppState` 内のリポジトリごとの `HashMap<slug, Vec<T>>` だったが、VCS再設計により実コンテンツアドレス方式の正規化SQLテーブル(`commits`/`branches`/`commit_files`/`tree_entries`/`file_blobs`/`pending_changes`/`staged_content`など)へ全面移行した(正確なスキーマは `lorehub-api/migrations/0001_vcs_schema.sql` を参照)。GETレスポンスの形状は変更なし——値だけ本物になった(`hash`は40→64桁hex、`shortHash`は7→12桁、`sizeDeltaLabel`は常に`"—"`だったのが実計算値になった)。

- `POST /api/repositories/{slug}/tree/stage` — body: `{ "path": "...", "changeType": "added"|"modified"|"deleted", "staged": true|false }`。`toggle_lock` と同じパターンでパスごとの保留変更(`pending_changes`テーブル)を追加/削除。ステージ時点で `staged_content` テーブルから実際の `content_hash`/`size_bytes` を確定する。
- `POST /api/repositories/{slug}/commits` — body: `{ "message": "...", "description": "" }`。保留変更が空なら `400`。追加/変更パスに未アップロードのコンテンツ(`content_hash IS NULL`)が残っていれば `400`。`commit_hash` は `repo_slug`+親hash+ブランチ+author+message+description+ソート済み変更一覧+作成時刻からSHA-256で実際に導出される64桁hex(旧実装の`rand` crateによる40桁フェイクハッシュを置き換え)。親コミットの `tree_entries` をコピーしてpending_changesを適用した新しいツリースナップショットを構築し、`commits`/`commit_files`/`tree_entries` へ書き込み、ブランチの `head_commit_hash` を更新、`pending_changes`/`staged_content` から該当パスを削除する。
- `POST /api/repositories/{slug}/branches` — body: `{ "name": "...", "from": "main" }`。`from` 省略時は現在のブランチ。既存名なら `400`。作成した行は実際の `branches` テーブルの1行であり、`head_commit_hash` を持つ。
- `POST /api/repositories/{slug}/checkout` — body: `{ "branch": "..." }`。`current_branch` テーブル(`repo_slug`単位)を更新するだけだが、これにより後続の `GET /tree`/`GET /commits` が実際に別ブランチの内容を返すようになる(チェックアウトが読み取り結果を一切変えなかった旧実装からの挙動修正)。

### 3.4 PUT /api/access-control/entries の仕様

body: `HashMap<path, Vec<AccessEntry>>`(`GET` と同じ形状)。**全置換ではなくパスごとのマージ**(insert-or-overwrite) — リクエストボディに含まれないパスの既存データは一切変更しない。**Owner/Adminロール必須**(`is_owner_or_admin`、§4.1)、それ以外は権限拒否。監査ログに `"applied access control configuration from Server Admin"` を1エントリ記録。Server Adminのノードエディタからのapplyで使用。

### 3.5 アップロードの種別推定とHTTP Range対応

`POST .../upload` は body `{ "path": "...", "contentBase64": "..." }` を受け取り、実バイト列から `content_hash = SHA-256` を算出してファイルシステム(`{blob_base_dir}/blobs/{slug}/{hash[..2]}/{hash}`、既存パスなら書き込みをスキップ)へ保存し、`file_blobs` テーブルへ重複排除INSERT、`staged_content` テーブルをupsertする(旧実装の `uploaded_images`/`uploaded_text`/`uploaded_audio` という3つの生バイトHashMapは廃止)。`path` の拡張子から種別を推定する部分は変更なし(`.txt`/`.md`/`.json`/`.yaml`/`.yml` → text、`.wav`/`.mp3` → audio、それ以外 → image)。同一内容を複数回アップロードしても `file_blobs` には1行しか残らず、ディスク上のblobファイルも1つだけになる(コンテンツの重複排除)。3Dモデルは意図的に対象外(§7参照)。

`get_image`/`get_image_before`/`get_audio`/`get_file_content` は共通の `ranged_bytes_response` ヘルパーを通り、`Range: bytes=<start>-<end>` リクエストヘッダを解釈する:
- ヘッダ無し → `200 OK`、全量を返す(`Accept-Ranges: bytes` を追加)
- 充足可能な範囲 → `206 Partial Content` + `Content-Range: bytes <start>-<end>/<total>`
- 範囲外 → `416 Range Not Satisfiable` + `Content-Range: bytes */<total>`

### 3.6 GET /api/repositories/{slug}/diff/{*path} の仕様

VCS再設計フェーズ5で追加。既存の `PullRequest.changed_files`/`PrDiffFile`(PR差分レビュー用、シードデータの決め打ち)とは完全に独立した、コミット間の実diff計算エンドポイント。

- クエリパラメータ `from`/`to`(共にコミットハッシュ、必須)。どちらか欠落/空文字なら `400`。
- `from`/`to` がそのリポジトリの実在コミットでなければ(`vcs_store::commit_exists`で直接照合、空の`tree_entries`から推測しない)`404`。
- `path` の `content_hash` を両コミット時点で独立に解決: どちらにも無ければ `404`。`to`のみに有れば`added`、`from`のみに有れば`deleted`、両方に有り一致すれば`unchanged`、不一致なら`modified`。
- `file_kind_for_path` が`"text"`と判定した`modified`のケースのみ、`similar::TextDiff::from_lines`で行レベルdiff(`FileDiffLine{type: context|add|remove, text, oldLine, newLine}`)を計算して`lines`に格納。それ以外(`added`/`deleted`/`unchanged`、またはどんな`change_type`でもバイナリ種別)は`lines: null`。
- 差分対象いずれかの内容が `MAX_DIFFABLE_FILE_BYTES`(5MiB)を超える場合は`400`(巨大テキストファイルの誤タグ付けを弾く安全装置、メモリ上に両ファイル全体+行テーブルを保持する`similar::TextDiff`の計算コストを有界に保つため)。
- 通常のGET同様、パスに対する`Read`権限のACLチェック(§4.1)を通過する必要がある。

レスポンス形状(`FileDiff`): `{ path, fromHash, toHash, changeType, isBinary, lines }`。

### 3.7 アカウントライフサイクル(パスワード変更・招待・パスワードリセット)

- **`POST /api/auth/change-password`** — body: `{ currentPassword, newPassword }`。現在のパスワードをargon2で検証、`newPassword`が8文字未満または`currentPassword`と同一なら`400`。成功時はこのアカウントの**全セッション/リフレッシュトークンを失効**(呼び出し元自身の現在のセッションも含む——`change_password`のコメントいわく「侵害されたパスワードで確立済みのセッションを1つも生き残らせない」ため意図的)、`204`を返す。
- **招待システム**(`POST /api/org/invites`ほか、全てOwner/Admin限定 †): `POST`は`InviteEntry`(email/name/role/teams/招待者/有効期限)を`AppState.invites`(トークン→エントリ)へ登録し、`inviteUrl`(`{LOREHUB_WEB_ORIGIN}/accept-invite?token=...`)をレスポンスボディに含めて返す——メール配送が失敗/未設定でも管理者がリンクを手動共有できるフォールバック(認証済みOwner/Adminだけがこのレスポンスを見られるため安全)。同一メールへの再招待は既存トークンを上書き(古いトークンは失効)。`GET`は期限切れを除外した一覧(トークン自体は返さない)。`DELETE /api/org/invites/{email}`はべき等(存在しなくても`204`)。
- **`GET /api/auth/invite/{token}`**(公開)/**`POST /api/auth/accept-invite`**(公開、body: `{ token, password }`): トークン検証は両方とも同じ(存在+未失効、`404`)。`accept-invite`成功時は`OrgMember`を新規作成し(`initials`は氏名の先頭2単語の頭文字から自動導出)、`login`と同じ形でアクセス/リフレッシュトークンを即発行してCookieをセット——招待承諾後の別ログイン操作は不要。
- **`POST /api/auth/forgot-password`**(公開、body: `{ email }`): **常に`204`**(該当アカウントの有無に関わらず)——レスポンスの違いでアカウント存在を推測できてしまう「アカウント列挙」攻撃を防ぐ意図的な設計。該当アカウントが実在する場合のみ`AppState.password_resets`にトークンを発行しメール送信、存在しない場合は監査ログにも一切残さない(存在しないメールアドレスを記録すること自体が列挙オラクルの移設になるため)。
- **`POST /api/auth/reset-password`**(公開、body: `{ token, newPassword }`): トークンが無効/期限切れなら`401`(セッション不正と同じ扱い)。成功時は`change-password`と同じパターンで対象アカウントの全セッション/リフレッシュトークンを失効。

## 4. 認証・セッションの実装詳細

- パスワードは argon2 でハッシュ化。デモアカウントは全員パスワード `"lorehub"` 共通(`state.rs` コメント参照)。
- ログイン成功時、2つのCookieを発行: `lorehub_token`(アクセストークン、`Max-Age=1800`=30分)と `lorehub_refresh`(リフレッシュトークン、`Max-Age=604800`=7日)。両方とも `HttpOnly; SameSite=Lax`。
- サーバー側は `AppState.sessions`/`refresh_tokens: HashMap<token, SessionEntry{email, expires_at}>` で有効期限を管理。`require_auth` はアクセストークンの `expires_at` を毎回チェックし、期限切れなら失効エントリを削除して401を返す。
- `POST /api/auth/refresh` はリフレッシュトークンを検証し、**ローテーション**(古いリフレッシュトークンを破棄し新しいアクセス+リフレッシュ両方を再発行)する。古いリフレッシュトークンの再利用は401になる(盗用されたトークンの使い回しを防ぐ標準的な対策)。
- Cookieは `localhost` ドメインに対して発行されるため、ポートが異なる `localhost:3000`(Web) と `localhost:4000`(API) の両方に自動送信される(ブラウザのCookieはホスト単位でありポート単位ではない)。
- Next.jsのServer Componentはブラウザとは別プロセスなのでCookieジャーを持たない。`src/lib/auth-server.ts` の `getSessionCookieHeader()` が `next/headers` の `cookies()` から手動で取得し、API呼び出し時にヘッダとして転送する。
- **透過的リフレッシュ(lorehub-web)**: CSR(ブラウザからの直接fetch)は `src/lib/api.ts` の `fetchWithRefresh` が401を検知して1回だけ `/api/auth/refresh` を叩きリトライする(同時に複数リクエストが401した場合もリフレッシュ呼び出しは1回に共有され、リトライも無限ループしない)。SSR(Server Component)はCookieを書き換えられないため、`src/proxy.ts`(このNext.jsバージョンで `middleware.ts` から改名された規約 — `node_modules/next/dist/docs/01-app/03-api-reference/03-file-conventions/proxy.md` 参照)がレンダリング前にアクセスCookie欠落を検知して先回りでリフレッシュする。
- **プロアクティブリフレッシュ(LoreForge Client / Server Admin)**: `AuthController`(Client)と `PermissionConfigController`(Server Admin)はどちらもログイン成功時に `QTimer`(25分間隔、定数 `kRefreshIntervalMs`)を起動し、30分のアクセストークンTTLより先に `POST /api/auth/refresh` を送る。リフレッシュトークンは共有 `QNetworkAccessManager` のCookie jarに既に保持されているため、追加の配線は不要。リフレッシュが失敗した場合(リフレッシュトークン自体の失効・サーバーダウン)はタイマーを止めてログアウト/未接続状態へ遷移する。個々のリクエストへ401リトライを後付けするより単純で、デスクトップアプリのセッションライフサイクルに適した設計として意図的に選択(Web版のリアクティブ方式とは異なる)。

### 4.1 認可: パスACL + RBAC(`authz.rs`)

`AppState.access_entries`(パス→principal別の`PermissionLevel`一覧)は、以前は `/api/access-control/entries*` のCRUDハンドラ自身以外どこからも参照されておらず、実際には**認証さえ通れば誰でも任意のパスを読み書き・ロックできた**——ACL設定は見た目上は存在するが何も強制していないハリボテだった。`src/authz.rs` の `check_path_permission(state, user, repo_slug, path, required)` がこれを解消し、パスが絡む全てのハンドラ(tree/content/image/audio/upload/lock/stage/diff)がこの1関数を通過する。

解決アルゴリズム:
1. `Owner`/`Admin` ロールは常に通過(組織全体の運用上のオーバーライド)。
2. それ以外は `path` の祖先チェーンを最も具体的なものから辿る(例: `"Assets/Characters/hero_rig.fbx"` → `"Assets/Characters/hero_rig.fbx"` → `"Assets/Characters"` → `"Assets"`)。**最初にエントリを持つ祖先が勝つ**——より具体的な祖先が見つかった時点で、それより上位の祖先のエントリは(たとえ許可されていても)一切参照しない。
3. どの階層にもエントリが無ければ**デフォルト許可**——未設定のパスには制限が無いという、この機能追加前からの事実上の全面公開挙動を維持し、既存デモリポジトリ/パスを壊さない。
4. エントリを持つ祖先が見つかった場合、`user` がそのエントリのいずれか(principalType=User なら名前一致、principalType=Team ならユーザーの所属チームのいずれかと一致)かつ `required` を含む権限を持つ必要がある。一致しなければ拒否。

`access_entries` は`repo_slug`をキーに含まない単一のフラットな組織全体マップのままである点に注意(意図的——アーキテクチャ図の`AppState`定義、Server Adminのノードエディタ、LoreHub WebのアクセスコントロールページのいずれもリポジトリごとのACLという概念を持たない。将来リポジトリスコープ化する場合は`authz.rs`のドキュメントコメントを参照)。

**RBAC(ロールゲート)**: パスACLとは別に、組織レベルの操作は `is_owner_or_admin(user)` で Owner/Admin ロールのみに制限される——リポジトリ削除、メンバーのロール変更、アクセス制御グラフのApply、招待の作成/一覧/取り消し(§3.4、§3.7の各エンドポイントの † 印)。パスACLが「どのファイル/ディレクトリを触れるか」を扱うのに対し、RBACは「組織そのものを構成する操作を行えるか」を扱う、独立した2層の認可。

### 4.2 ネットワークレベルのセキュリティ

- **Cookieセキュリティ**: `LOREHUB_INSECURE_COOKIES`未設定時、発行する4種のCookieは全て`Secure`属性つき(§2.1)。
- **CORS**: `LOREHUB_WEB_ORIGIN`で指定した単一オリジンのみ許可、`allow_credentials(true)`のためワイルドカード不可(§2.1)。
- **ボディサイズ上限**: `LOREHUB_MAX_BODY_BYTES`(既定48MiB)、`tower_http::limit::RequestBodyLimitLayer`(§2.1)。
- **ログインのレート制限**: 送信元IP単位、バースト8回+60秒に1回補充(`tower_governor`、§2.1)。`axum::serve`を`into_make_service_with_connect_info::<SocketAddr>()`経由で起動しないと送信元IPが見えず機能しない。

## 5. 永続化: kv_store ブロブ方式 + 正規化VCSテーブルの併存

2つの永続化方式が併存する(詳細は `lorehub-api/src/db.rs` のモジュールコメントを一次情報源とする)。

- **非VCSデータ**(`pull_requests`/`access_entries`/`org_members`/`storage`/`audit_log`/`credentials`/`sessions`/`refresh_tokens`/`invites`/`password_resets`): `AppState` の各フィールドを個別のJSONブロブとして1テーブル(`kv_store: key TEXT, value TEXT`)に保存する。利点はRustの構造体をそのまま `serde_json::to_string`/`from_str` でき、マイグレーション不要な点。制約はSQLでの複雑な検索・集計ができないこと(全件ロードしてRust側でフィルタする設計)。今回のVCS再設計ではスコープ外と判断し、このデモ〜小規模組織のデータ量を前提としたトレードオフをそのまま維持している。
- **VCSデータ**(`repositories`/`file_blobs`/`branches`/`current_branch`/`commits`/`commit_files`/`tree_entries`/`staged_content`/`pending_changes`/`file_locks`): 実コンテンツアドレス方式の正規化SQLテーブル。全ての `repo_slug` 外部キーが `ON DELETE CASCADE` で宣言されており、リポジトリ削除1文で関連行が原子的に全消去される(§3.2参照)。正確なスキーマ(カラム・制約・インデックス)は `lorehub-api/migrations/0001_vcs_schema.sql` を一次情報源とし、ここでは重複させない。`repo_store.rs`/`vcs_store.rs`/`lock_store.rs` が `AppState` のロックを一切経由せず直接読み書きする。

`load_state`(非VCSデータのみが対象)は保存されたキーが一部欠けていても(例: 過去バージョンのDBに `refresh_tokens` が無い場合)フォールバックして起動できるようになっている。

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
- `kv_store` 方式は非VCSデータについては引き続き将来的にリレーショナルスキーマへ移行する余地を残す(現状はデータ量的に不要と判断)。VCSデータ(リポジトリ・コミット・ブランチ・ツリー・ファイル実体)は本ドキュメント執筆時点で既に正規化SQLテーブルへ移行済み(§5参照)。
- LoreForge Client/Server AdminにもQt Testによる単体テストを導入済み(`loreforge-client/tests/tst_repositorytreemodel.cpp`、`loreforge-server-admin/tests/tst_permissionconfigcontroller.cpp`)。GUI操作の自動化(`SendInput`等)はこのサンドボックスでは信頼できないため対象外とし、`RepositoryTreeModel`のツリー構築/Sparse Workspace Managerのinclude/exclude/cascade-exclude/ステージ管理、`PermissionConfigController`のJSON永続化ラウンドトリップなど、GUI非依存の純粋なモデル/コントローラロジックのみを対象にしている。各`CMakeLists.txt`に`enable_testing()`と`LoreForgeClientTests`/`LoreForgeServerAdminTests`ターゲットを追加し、`main.cpp`を除いた対象`.cpp`をテスト実行ファイルへ直接コンパイル(`QTEST_GUILESS_MAIN`が独自の`main()`を提供)。`ctest`(各`build/`ディレクトリ内)で実行できる。
- `access_entries`(§4.1)はリポジトリスコープを持たない組織全体のフラットなマップのまま——両クライアントとも今のところリポジトリごとのACLという概念自体を持っていないための意図的な現状維持であり、将来リポジトリスコープ化する場合は`GET`/`PUT /api/access-control/entries`へのルート変更(`{slug}`セグメント追加)とクライアント側の対応改修が必要になる。
- デプロイ手順(Docker Compose・環境変数・TLS/リバースプロキシ・バックアップ)は `docs/DEPLOYMENT.md`、デスクトップインストーラーのビルド手順は `docs/DESKTOP_PACKAGING.md` を参照(本ドキュメントでは重複させない)。
