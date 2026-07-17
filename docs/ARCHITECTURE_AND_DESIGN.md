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

**現状の実装範囲の注記**: `ServerAdmin → ServerProc`(実プロセス起動/停止/PID・メモリ監視)と `ServerAdmin → Handlers`(権限グラフのApply)はどちらも実データで動作する実装。`ServerAdmin → Docker` のMinIO制御も実際の `docker` CLIを呼び出す実装だが、この開発環境にDockerが未インストールのため実機検証はできていない(コードレビューベース)。LoreHub WebとLoreForge Clientはどちらも同一の `lorehub-api` に接続しており、片方で作った変更がもう片方にリアルタイムで反映されることを確認済み。

## 3. コンポーネント詳細

### 3.1 LoreHub (Web)

- **フロントエンド**: Next.js 16 (App Router, Server Components), TypeScript, Tailwind CSS v4
- **バックエンド**: Rust (Axum), tokio, sqlx(SQLite), argon2, tower-http CORS
- **主要画面**: リポジトリ一覧 / ツリー閲覧 / ファイル詳細(画像・音声・3Dプレビュー) / コミット履歴(ブランチグラフ) / PR差分レビュー / アクセス制御 / 組織設定 / **リポジトリ設定(rename・削除)**
- **認証**: HttpOnlyセッションCookie。Server Componentは `next/headers` の `cookies()` からCookieを読み取りAPIへ転送(`auth-server.ts`)。Client Componentは `credentials: "include"` でブラウザが自動送信。

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
- **今後**: 実バイナリアセットのアップロード/差分生成(現状はサーバー側デモデータを参照)

### 3.3 LoreForge Server Admin (Desktop)

- **UI**: Qt6 / QML、ノードエディタ風の権限設定UI
- **ロジック**: C++20、`QProcess` によるDocker制御(`DockerController`)とローカルプロセス制御(`LoreServerController`)
- **実装済み**:
  - 環境ステータスパネル(MinIO/Lore Serverの2カード)
  - **Lore Server実制御**: `lorehub-api.exe` をローカルプロセスとして起動/停止、PID・メモリ使用量(`tasklist` 経由)を監視。アプリ終了時も子プロセスを確実に回収(orphan防止)
  - MinIODocker制御(`docker run`/`stop`/`ps`/`stats`、CPU/RAM表示)— コード実装済みだがこの開発環境にDocker未インストールのため実機未検証
  - ディレクトリ×ロールのノードエディタUI、権限設定のローカル永続化(JSON)
  - **権限グラフのApply**: ログインしてlorehub-apiへ `PUT /api/access-control/entries` を送信し、ノードエディタの権限グラフを実サーバーへ反映(パスごとのマージ、対象外パスは無傷)
- **今後**: ノードエディタでのディレクトリ/ロール追加・削除UI(現状は既定の5+3ノード構成が前提)

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

## 10. 現在の状態(このドキュメント作成時点)

- ✅ LoreHub Web: 8画面すべて実装、認証・永続化・リポジトリ設定(rename/削除)まで完了、lint/build検証済み
- ✅ lorehub-api: 全エンドポイント認証必須化、SQLite永続化、リポジトリのCRUD完備、VCS書き込みAPI(commit/branch/stage)完備、access-control Apply対応
- ✅ LoreForge Client: 閲覧+ロック操作に加え、Fork並みの実操作(コミット/ブランチ/ステージング)、バイナリDiffビューア、Sparse Workspace Managerまで完備
- ✅ LoreForge Server Admin: 実プロセスとしてのLore Server制御(起動/停止/PID/メモリ監視)、権限グラフの実サーバーへのApply、タブ視認性修正まで完備。MinIOのDocker制御はコード実装済みだが実機(Docker)未検証
- ⏳ 未着手: LoreForge Clientでの実バイナリアセットアップロード/差分生成、Server Adminのノードエディタでのディレクトリ/ロール追加・削除UI、MinIO Docker制御の実機検証
