# Lore Ecosystem — アーキテクチャ & 設計ドキュメント

> このドキュメントは `ARCHITECTURE.md` / `DESIGN.md` / `LOREHUB_UI_SPEC.md` / `QUALITY_STANDARDS.md` と、ここまでの実装(git log)を統合した俯瞰資料です。個別の設計判断の一次情報源はリポジトリ直下の4ファイルを参照してください。

## 1. プロジェクト概要

**Lore** という架空の「巨大バイナリ対応の次世代VCS」を中心に、3つの独立コンポーネントからなるエコシステムを構築している。

| コンポーネント | 役割 | 立ち位置の例え |
|---|---|---|
| **LoreHub** | Webブラウザからリポジトリ閲覧・PR・権限管理 | GitHub / GitLab |
| **LoreForge Client** | デスクトップのVCSクライアント | Fork / GitKraken |
| **LoreForge Server Admin** | Loreサーバー環境をGUIで構築・管理 | Docker Desktop + 権限エディタ |

3つとも見た目の一貫性を保つため、単一のデザイントークン(`DESIGN.md`)を共有している。

## 2. システム構成図

```mermaid
graph TB
    subgraph Browser["ブラウザ"]
        WebUI["LoreHub Web UI<br/>Next.js 16 / React / Tailwind v4"]
    end

    subgraph Desktop["デスクトップ"]
        Client["LoreForge Client<br/>Qt6 / QML / C++20"]
        ServerAdmin["LoreForge Server Admin<br/>Qt6 / QML / C++20"]
    end

    subgraph Backend["lorehub-api (Rust / Axum)"]
        Handlers["Handlers<br/>(auth, repos, tree, commits, PR, access-control, org)"]
        AppState["AppState<br/>(RwLock, in-memory)"]
        DB["db.rs<br/>kv_store blob永続化"]
    end

    SQLite[("SQLite<br/>lorehub.db")]
    Docker[("Docker<br/>MinIO<br/>(Server Adminが制御)")]
    ServerProc(["lorehub-api.exe<br/>ローカルプロセス"])

    WebUI -- "fetch, credentials: include<br/>Cookie(session)" --> Handlers
    Client -- "QNetworkAccessManager<br/>Cookie jar" --> Handlers
    Handlers -- "read/write" --> AppState
    AppState -- "save_blob / load_state" --> DB
    DB --> SQLite
    ServerAdmin -- "docker run/stop/stats<br/>(QProcess)" --> Docker
    ServerAdmin -- "QProcess 起動/停止<br/>PID・メモリ監視" --> ServerProc
    ServerAdmin -- "PUT /api/access-control/entries<br/>(ログイン後)" --> Handlers
    ServerProc -.->|"実体"| Backend

    style WebUI fill:#1ed760,color:#121212
    style Client fill:#1ed760,color:#121212
    style ServerAdmin fill:#1ed760,color:#121212
    style Handlers fill:#181818,color:#fff,stroke:#1ed760
    style AppState fill:#181818,color:#fff,stroke:#4d4d4d
    style DB fill:#181818,color:#fff,stroke:#4d4d4d
```

**現状の実装範囲の注記**: `ServerAdmin → ServerProc`(実プロセス起動/停止/PID・メモリ監視)、`ServerAdmin → Handlers`(権限グラフのApply)、`ServerAdmin → Docker` のMinIO制御(`docker run`/`stop`/`ps`/`stats`)はすべて実データ・実Docker環境で動作確認済み(コンテナの起動・ポートマッピング・`docker stats`による実CPU/メモリ取得・停止まで実機検証)。LoreHub WebとLoreForge Clientはどちらも同一の `lorehub-api` に接続しており、片方で作った変更がもう片方にリアルタイムで反映されることを確認済み。

## 3. コンポーネント詳細

### 3.1 LoreHub (Web)

- **フロントエンド**: Next.js 16 (App Router, Server Components), TypeScript, Tailwind CSS v4
- **バックエンド**: Rust (Axum), tokio, sqlx(SQLite), argon2, tower-http CORS
- **主要画面**: リポジトリ一覧 / ツリー閲覧 / ファイル詳細(画像・音声・3Dプレビュー) / コミット履歴(ブランチグラフ) / PR差分レビュー / アクセス制御 / 組織設定 / **リポジトリ設定(rename・削除)**
- **認証**: HttpOnlyセッションCookie(アクセス+リフレッシュのデュアルトークン)。Server Componentは `next/headers` の `cookies()` からCookieを読み取りAPIへ転送(`auth-server.ts`)。Client Componentは `credentials: "include"` でブラウザが自動送信。トークン失効時はCSR/SSR両経路で透過的に自動リフレッシュ(§4.2参照)。
- **プレビューの実配信**: `StreamingPreview.tsx` は当初「700msの偽プログレスバー」で本物のチャンク配信を模していたが、`fetch().body.getReader()` で実バイト受信量に基づく進捗表示に置き換え、バックエンドも本物のHTTP Range(`206 Partial Content`/`416`)に対応済み(§4.3参照)。

### 3.2 LoreForge Client (Desktop)

- **UI**: Qt6 / QML、`QML_ELEMENT` マクロでC++型をQMLに公開
- **ロジック**: C++20、`QNetworkAccessManager` + Cookie jarでlorehub-apiと直接通信(Web版と同一バックエンドを共有)
- **実装済み**:
  - ログイン画面、リポジトリ一覧(実データ取得)
  - ファイルツリー閲覧 + ロック/アンロック操作、グローバル検索
  - コミット履歴ビュー(ブランチ別カラーリング、マージコミット表示)
  - **Fork並みの実操作**: ファイルのステージング(Added/Modified/Deleted)、コミット作成、ブランチ作成/切替、Pull(明示的な再フェッチ)
  - **バイナリDiffビューア**: 画像Before/Afterスライダー(`LoreImageProvider` による認証付き非同期画像取得)、3Dモデルの視覚的Diffトグル(スタイライズされたワイヤーフレーム代替表現)
  - **Sparse Workspace Manager**: ディレクトリ単位でワークスペースに含める/含めないを選択、`QSettings` でリポジトリごとに永続化。除外時は配下の選択も連鎖的に解除
  - **実アセットアップロード**: 画像/テキスト/音声ファイルをローカルから選んでlorehub-apiへアップロードし、実バイトを保存(リポジトリ単位で分離、種別はパスの拡張子から推定)。成功時に自動でAdded扱いとしてステージング。3Dモデルは意図的に対象外(Web/Client双方に実ジオメトリを読み込む仕組みが無く、アップロードしても反映先が無いため)
  - **テキスト/音声プレビュー**: テキストファイルは `Flickable` + 読み取り専用 `TextEdit` でスクロール可能表示。音声は `AudioPlayerController`(`QMediaPlayer` + 認証付き非同期フェッチ)で実再生・シーク対応
  - **セッション自動延長**: `AuthController` が25分ごとに `POST /api/auth/refresh` を送信し、30分のアクセストークンTTLより先にセッションを更新(§4.2参照)
- **今後**: なし(Fork並みの実操作・プレビュー・アップロード・セッション延長まで一通り完了)

### 3.3 LoreForge Server Admin (Desktop)

- **UI**: Qt6 / QML、ノードエディタ風の権限設定UI
- **ロジック**: C++20、`QProcess` によるDocker制御(`DockerController`)とローカルプロセス制御(`LoreServerController`)
- **実装済み**:
  - 環境ステータスパネル(MinIO/Lore Serverの2カード)
  - **Lore Server実制御**: `lorehub-api.exe` をローカルプロセスとして起動/停止、PID・メモリ使用量(`tasklist` 経由)を監視。アプリ終了時も子プロセスを確実に回収(orphan防止)
  - MinIODocker制御(`docker run`/`stop`/`ps`/`stats`、CPU/RAM表示)— 実Docker環境で動作確認済み
  - **動的ノードエディタ**: ディレクトリ/ロールノードを自由に追加・削除可能(既定の5+3ノード構成は初期値であり上限ではない)。ノード削除時は関連する接続も連鎖的に除去、`QSettings`ではなくJSON設定ファイルへ永続化
  - **権限グラフのApply**: ログインしてlorehub-apiへ `PUT /api/access-control/entries` を送信し、ノードエディタの権限グラフを実サーバーへ反映(パスごとのマージ、対象外パスは無傷)。動的追加したノードでも同じパイプラインで動作確認済み
  - **セッション自動延長**: `PermissionConfigController` も同じく25分ごとに `POST /api/auth/refresh`(Client側と全く同じパターン、Apply機能のログインセッションを維持)
- **今後**: Qt Test基盤の整備(lorehub-api/lorehub-webには自動テストスイートを追加済みだが、2つのデスクトップアプリは未整備。意図的に別タスクとして切り出し中)

## 4. データフロー: 認証シーケンス

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant Web as LoreHub Web<br/>(Server Component)
    participant API as lorehub-api<br/>(Axum)
    participant DB as SQLite

    U->>Web: /login でemail/password送信 (Client Component)
    Web->>API: POST /api/auth/login (credentials: include)
    API->>API: argon2でパスワード検証
    API->>DB: セッション/ユーザー情報照会
    API-->>Web: Set-Cookie: lorehub_token (HttpOnly)
    Web-->>U: ログイン成功、/ へリダイレクト

    Note over U,API: 以降のページ遷移(SSR)
    U->>Web: GET /repositories/xxx (ブラウザ→Next.js)
    Web->>Web: cookies() からセッションCookie取得
    Web->>API: GET /api/repositories/xxx (Cookieヘッダ手動転送)
    API->>API: require_auth ミドルウェアで検証
    API-->>Web: リポジトリJSON
    Web-->>U: SSRレンダリング済みHTML
```

**設計判断**: 当初は「書き込み系のみ認証必須」だったが、GETの素通し(読み取りが誰でも可能)を自己発見しギャップとして塞ぎ、全エンドポイントを `require_auth` ミドルウェア配下に統一した。

### 4.1 権限グラフのApply(Server Admin → lorehub-api)

Server Adminのノードエディタで組んだ権限グラフは、当初はローカルJSONファイルに保存するだけで実サーバーと無関係だった。以下のフローで実際にlorehub-apiへ反映されるようになっている。

```mermaid
sequenceDiagram
    participant Admin as Server Admin
    participant API as lorehub-api

    Admin->>API: POST /api/auth/login (email/password)
    API-->>Admin: Set-Cookie: lorehub_token
    Admin->>Admin: ノードエディタでdirectories×rolesのグラフを構築
    Admin->>API: PUT /api/access-control/entries<br/>{ "Assets/Characters": [{principal, permissions}] }
    API->>API: パスごとにマージ(insert-or-overwrite)<br/>グラフに無いパスは無傷のまま
    API-->>Admin: 更新後の access_entries 全体を返す
    Note over API: audit_logに "applied access control<br/>configuration from Server Admin" を記録
```

**設計判断**: `PUT` は全置換ではなく「グラフに含まれるパスだけを上書きし、それ以外は触らない」マージ方式。Server Adminの既定グラフ(5ディレクトリ)がlorehub-apiのデモ全パスを網羅していないため、全置換だと未対応パスのデモデータが消えてしまう。

### 4.2 セッションのリフレッシュ

アクセストークンは30分でサーバー側が失効させる。放置すると全リクエストが401になるため、クライアントごとに異なる復旧経路を持つ。

```mermaid
flowchart LR
    subgraph Web["lorehub-web"]
        CSR["CSR: fetchWithRefresh<br/>401検知→refresh→1回だけ再試行"]
        SSR["SSR: proxy.ts<br/>アクセスCookie欠落を検知→先回りrefresh"]
    end
    subgraph Desktop["Client / Server Admin"]
        Timer["QTimer: 25分ごとに<br/>POST /api/auth/refresh"]
    end
    API["lorehub-api<br/>POST /api/auth/refresh<br/>(ローテーション)"]

    CSR --> API
    SSR --> API
    Timer --> API
```

Web(CSR/SSR)は「失効を検知してから直す」リアクティブ方式、デスクトップアプリ(Client/Server Admin)は「失効前に定期更新する」プロアクティブ方式 — 個々のリクエストへの401リトライをQNetworkReplyベースの全呼び出し箇所に後付けするより、単純なタイマーの方がデスクトップアプリのセッションライフサイクルには適切と判断した。リフレッシュ失敗時(リフレッシュトークン自体の失効やサーバーダウン)はタイマーを止めてログアウト状態に遷移し、ゾンビタイマーが失敗し続けることを防ぐ。

### 4.3 バイナリプレビューの実配信(HTTP Range)

`get_image`/`get_image_before`/`get_audio`/`get_file_content` はいずれも `Range: bytes=<start>-<end>` リクエストヘッダを解釈し、`206 Partial Content`(`Content-Range`/`Accept-Ranges`付き)または範囲外なら `416 Range Not Satisfiable` を返す。`Range`ヘッダが無い場合は従来通り全量を `200` で返す(`Accept-Ranges: bytes` を追加で広告)。`get_file_content` はこの変更に合わせ `{"content": "..."}` のJSON包装をやめ、生の `text/plain; charset=utf-8` バイト列を直接返すよう変更した(JSONドキュメントの一部だけを切り出しても意味が無いため)。

## 5. データモデル (AppState)

```mermaid
classDiagram
    class AppState {
        RwLock~AppStateInner~
        +repositories: Vec~Repository~
        +seeded_repo_slugs: HashSet~String~
        +commits, branches: per-repo
        +pull_requests: Vec~PullRequest~
        +access_entries: Record~path, AccessEntry[]~
        +members: Vec~OrgMember~
        +audit_log: Vec~AuditLogEntry~
        +sessions: token → OrgMember
    }
    class Repository {
        +slug, name, organization
        +description, visibility
        +sizeLabel, lockedFileCount
        +updatedAt
    }
    class PullRequest {
        +id, repo_slug, status
        +diff_files, comments
    }
    class OrgMember {
        +email, name, role
        +password_hash
    }
    class AuditLogEntry {
        +actor, action, target, timestamp
    }
    AppState "1" --> "*" Repository
    AppState "1" --> "*" PullRequest
    AppState "1" --> "*" OrgMember
    AppState "1" --> "*" AuditLogEntry
```

**永続化方式**: フルリレーショナル正規化ではなく、フィールドごとに1レコードのJSONブロブを `kv_store` テーブルへ保存する方式を採用(`db.rs`)。トレードオフとして複雑なクエリはできないが、Rust側の構造体をそのまま `serde_json` でシリアライズでき、スキーマ移行の手間がない。デモ規模のデータ量では十分と判断し、意図的に選択した。

## 6. LoreHub Web サイトマップ

```mermaid
flowchart LR
    Login["/login"] --> Home["/ (リポジトリ一覧)"]
    Home --> Repo["/repositories/[slug]"]
    Repo --> Code["Code タブ<br/>ツリー+README"]
    Repo --> Commits["Commits タブ<br/>ブランチグラフ"]
    Repo --> Settings["Settings タブ<br/>rename/visibility/削除"]
    Code --> FileDetail["ファイル詳細<br/>(画像/音声/3Dプレビュー)"]
    Commits --> CommitDetail["/commits/[hash]"]
    Home --> Pulls["/pulls (PR一覧)"]
    Pulls --> PRDetail["/pulls/[id]<br/>差分レビュー+コメント"]
    Home --> AccessControl["/access-control<br/>ディレクトリ×権限マトリクス"]
    Home --> OrgSettings["/settings<br/>メンバー・監査ログ・容量"]
```

## 7. デザインシステム概要

Spotify風ダークUIをベースに、LoreHub Web(CSS変数/Tailwind theme)とQt/QML(`Theme.qml`)の両方が同じ値を参照する「Cross-Platform Token Mapping」を`DESIGN.md` §10に定義。

| トークン | 値 | Web (CSS変数) | Qt/QML |
|---|---|---|---|
| 背景(最深部) | `#121212` | `--color-bg-base` | `Theme.colorBackgroundBase` |
| サーフェス | `#181818` | `--color-bg-surface` | `Theme.colorSurface` |
| アクセント(機能用途限定) | `#1ed760` | `--color-accent` | `Theme.colorAccent` |
| テキスト(主) | `#ffffff` | `--color-text-base` | `Theme.colorTextPrimary` |
| テキスト(副) | `#b3b3b3` | `--color-text-muted` | `Theme.colorTextSecondary` |
| エラー | `#f3727f` | `--color-negative` | `Theme.colorNegative` |
| 標準角丸 | 6px | `--radius-standard` | `radius: Theme.radiusStandard` |
| ピル角丸 | 500px | `--radius-pill` | `radius: height / 2`(実行時計算) |

## 8. 開発の歩み

```mermaid
timeline
    title Lore Ecosystem 実装フェーズ
    Phase 0 設計固め : Initial commit : システムアーキテクチャ確定 : デザイントークン統一
    Phase 1 LoreHub Web 基本UI : リポジトリ一覧/ツリー/ファイル詳細 : コミット履歴/PR差分レビュー : アクセス制御/組織設定画面
    Phase 2 バックエンド接続 : Rust(Axum)スキャフォールド : Web⇔API結線 : ブランチグラフを横型に変更
    Phase 3 リッチプレビュー : Three.js 3Dビューア : 実画像プレビュー : Web Audio波形プレイヤー
    Phase 4 認証と永続化 : セッションベース認証(argon2) : 全GET読み取りをゲート : SQLiteへ永続化
    Phase 5 デスクトップアプリ着手 : LoreForge Client雛形+ログイン : ファイルツリー+ロック操作 : Server Admin雛形+権限ノードエディタ
    Phase 6 GitHubレベル機能拡張 : リポジトリ設定(rename/削除) : Client コミット履歴表示(並行作業) : Server Admin 権限設定永続化(並行作業)
    Phase 7 アーキテクチャギャップの解消 : Client Fork並み実操作(commit/branch/stage) : Server Admin実プロセス制御(Lore Server) : タブ視認性修正 : 権限グラフのApply連携 : Clientバイナリdiffビューア : Sparse Workspace Manager
    Phase 8 残課題の完全消化 : Server Adminノードエディタ動的化 : MinIO実機検証 : リフレッシュトークン基盤 : Clientテキスト/音声プレビュー : Web実チャンクストリーミング : 自動テストスイート(lorehub-api/web) : Clientテキスト/音声アップロード : デスクトップ側セッション自動延長
```

## 9. マルチエージェント並行開発

Phase 6では、3つの独立した作業(Web機能拡張・Client機能拡張・Server Admin機能拡張)をディレクトリが重ならない形で分割し、バックグラウンドサブエージェントとして並行実行した。

```mermaid
graph LR
    Orchestrator["メインセッション"]
    Orchestrator -->|"lorehub-web/ 直接実装"| WebWork["リポジトリ設定機能"]
    Orchestrator -->|"サブエージェント dispatch"| ClientAgent["loreforge-client/<br/>コミット履歴ビュー"]
    Orchestrator -->|"サブエージェント dispatch"| AdminAgent["loreforge-server-admin/<br/>権限設定の永続化"]
    ClientAgent -.->|"完了時に通知"| Orchestrator
    AdminAgent -.->|"完了時に通知"| Orchestrator
```

各エージェントには同じ「クラッシュ調査手法」(`PrintWindow`によるスクリーンショット、`MSYS_NO_PATHCONV=1`、QML singleton診断法など)を事前共有し、独立して発見した問題の再発を防いだ。

### 9.1 独立再検証パターン

Phase 7では、サブエージェントの完了報告を鵜呑みにせず、オーケストレーター(メインセッション)が毎回ゼロから独立して再検証するパターンを徹底した。

```mermaid
flowchart TD
    A["サブエージェント完了報告"] --> B["build/ を完全削除して<br/>クリーンビルド"]
    B --> C{"ビルド成功?"}
    C -->|No| D["環境要因を切り分け<br/>(例: VSインストールパス変更)"]
    C -->|Yes| E["報告された動作を<br/>自分の手で再現"]
    E --> F["実プロセス/実HTTP応答/<br/>レジストリなど一次証拠で確認"]
    F --> G["TEMP-VERIFY残留ゼロを<br/>grepで確認"]
    G --> H["git diffが報告と一致するか確認"]
    H --> I["push"]
```

この過程で、報告だけでは分からない実環境の変化(Visual Studioのインストールパスが `2022` フォルダから `18` フォルダへ自動更新されていた事実)や、検証手法そのものの欠陥(QMLの `console.log` がリダイレクトされたログファイルへ確実にフラッシュされない問題、複雑な画面遷移を伴うTimerチェーンより単機能の検証用QMLを直接ロードする方が確実、という教訓)を発見した。**「動きました」という報告と、実際に動くことは別物**という前提に立ち、毎回一次証拠(実プロセスのPID、実HTTPレスポンス、レジストリの値そのもの)を自分で取得することを徹底した。

### 9.2 自動テストスイート

Phase 8で、これまで一切存在しなかった自動テストを `lorehub-api` と `lorehub-web` に整備した(2つのQtデスクトップアプリのテスト基盤は意図的に別タスクとして切り出し中)。

- **lorehub-api**: `tower::ServiceExt::oneshot` で実際の `Router` をTCPバインド無しにin-processで駆動する統合テスト、39件。各テストが独立した `sqlite://:memory:` DBと `state::seed()` を持ち、実DBファイルには一切触れない(`cargo test`並列実行下でのテスト間汚染を防ぐ設計)。認証(ログイン/リフレッシュローテーション/失効)・リポジトリCRUD・VCS書き込みフロー・アクセス制御マージ・アップロードのリポジトリ単位分離(過去のバグクラスの回帰テスト)・HTTP Range(206/416)をカバー。
- **lorehub-web**: Vitestを新規導入(node環境、DOM操作なしのため jsdom不要)、22件。`fetchWithRefresh` の401リトライロジック(無限ループしないこと・同時失敗時のリフレッシュ共有まで含む)、`proxy.ts` のCookie合成ヘルパー、`AudioPlayer` の時刻フォーマットをカバー。

**独立再検証で確認**: `cargo test`/`npm run test` とも全件パスをクリーンビルドから再現、テストが実DBファイル(`lorehub.db`)を作成しないことも確認済み。

## 10. 現在の状態(このドキュメント作成時点)

- ✅ LoreHub Web: 8画面すべて実装、認証・永続化・リポジトリ設定(rename/削除)・**実チャンクストリーミング(HTTP Range)** まで完了、lint/build/test検証済み
- ✅ lorehub-api: 全エンドポイント認証必須化、SQLite永続化、リポジトリのCRUD完備、VCS書き込みAPI(commit/branch/stage)完備、access-control Apply対応、アクセストークン(30分)+リフレッシュトークン(7日、ローテーション付き)のデュアルトークン認証、**HTTP Range配信**、**統合テスト39件**
- ✅ LoreForge Client: 閲覧+ロック操作に加え、Fork並みの実操作(コミット/ブランチ/ステージング)、バイナリDiffビューア(画像スライダー/3Dトグル)、**テキスト/音声プレビュー**、**画像/テキスト/音声の実アセットアップロード→自動ステージング**、Sparse Workspace Manager、**セッション自動延長(25分ごとのプロアクティブrefresh)**まで完備
- ✅ LoreForge Server Admin: 実プロセスとしてのLore Server制御(起動/停止/PID/メモリ監視)、権限グラフの実サーバーへのApply、ノードエディタでのディレクトリ/ロールの動的追加・削除、タブ視認性修正、MinIOのDocker制御(実機検証済み)、**セッション自動延長**まで完備
- ✅ lorehub-web: SSR経路(`proxy.ts`)とCSR経路の両方でアクセストークン失効時の透過的リフレッシュに対応、**Vitestによる自動テスト22件**
- ⏳ 未着手: LoreForge Client/Server AdminのQt Test自動テスト基盤整備、Clientでの3Dモデルアセットアップロード(実ジオメトリローダーが存在しないため意図的に対象外)
