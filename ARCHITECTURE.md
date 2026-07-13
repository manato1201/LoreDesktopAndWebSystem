# System Design Document: Lore Ecosystem (GUI Clients & Web Service)

> Status: v1 — 設計確定フェーズ。実装前の一次情報源(single source of truth)。
> 関連ドキュメント: ビジュアルデザインシステムは [`DESIGN.md`](./DESIGN.md)、LoreHubのIA/画面設計は [`LOREHUB_UI_SPEC.md`](./LOREHUB_UI_SPEC.md)、品質基準は [`QUALITY_STANDARDS.md`](./QUALITY_STANDARDS.md) を参照。

## 1. System Overview (システム全体像)

本プロジェクトは、Epic Gamesが開発した巨大バイナリ対応の次世代VCS「Lore」を利用し、直感的なGUI操作を可能にするエコシステムを構築する。
以下の3つの独立した、かつ相互連携するコンポーネントで構成される。

- **LoreForge Client (Desktop Client App)**: Gitクライアント「Fork」のような、Loreリポジトリの操作、ファイルロック管理、アセットの3D/2Dプレビューを行うクライアントGUI。
- **LoreForge Server Admin (Desktop Server App)**: ローカルネットワーク等にLoreサーバーおよびバックエンドストレージ（S3互換）をGUI操作でワンクリック構築・管理するアプリ。
- **LoreHub (Web Service)**: GitHub/GitLabのように、WebブラウザからLoreリポジトリの閲覧、権限管理、プルリクエスト（差分レビュー）を行うWebアプリケーション。

## 2. Component 1: LoreForge Client (Desktop Client)

### 2.1. Role & Objective

開発者やアーティストがコマンドライン(CLI)を使わずに、Loreの強力な機能（Sparse Workspace, Chunking, File Locking）を直感的に利用できるようにする。

### 2.2. Tech Stack

- **UI Layer**: Qt 6, QML (モダンで滑らかなUIコンポーネントの構築)
- **Logic Layer**: C++20
- **VCS Integration**: Rust FFI (Foreign Function Interface) または QProcess。
  - AIへの指示: 初期フェーズでは `lore` CLIコマンドを非同期で叩くラッパー（QProcess）として実装し、JSON出力をパースする。パフォーマンスが必要なコア部分は将来的にRustライブラリとのFFIに移行する設計とすること。
- **Asset Preview**: QOpenGLWidget または Qt 3D (FBX/OBJ等の3Dモデル差分プレビュー用)

### 2.3. Key Features & Implementation Notes

- **Sparse Workspace Manager**: ツリービューから必要なディレクトリのみを選択し、オンデマンドで取得するUI。
- **Visual Lock Indicator**: 誰がどのバイナリアセットをロック（排他制御）しているかを、アセットアイコン上にオーバーレイ表示する。
- **Binary Diff Viewer**: テキストのDiffではなく、画像（Before/Afterのスライダー）や3Dモデルの視覚的なDiff機能。

## 3. Component 2: LoreForge Server Admin (Desktop Server App)

### 3.1. Role & Objective

コマンドラインやインフラの知識がないユーザーでも、社内LAN等にLoreサーバー環境をGUIから即座に構築・監視できるようにする。

### 3.2. Tech Stack

- **UI Layer**: Qt 6, QML
- **Logic Layer**: C++20
- **Infrastructure Control**: Docker SDK API (またはC++プロセス管理)
- **Database (Settings)**: SQLite

### 3.3. Key Features & Implementation Notes

- **1-Click Environment Setup**: バックエンドとして必要なS3互換ストレージ (MinIO等) と Lore Server プロセスを、Dockerコンテナまたはローカルプロセスとしてボタン一つで起動・停止・ステータス監視(CPU/RAM)する。
- **Visual Access Control**: Loreの特徴である「ディレクトリ単位の権限管理」を、ノードエディタ風のUIで設定し、裏側でLoreのサーバー設定ファイル（JSON等）を生成・適用する。

## 4. Component 3: LoreHub (Web Service)

### 4.1. Role & Objective

GitHubのように、Webブラウザ上でリポジトリのツリー構造、コミット履歴、バイナリアセットのプレビューを可能にするサービス。

### 4.2. Tech Stack

- **Frontend**: Next.js (React), TypeScript, Tailwind CSS
- **Backend**: Rust (Axum または Actix-Web)
  - AIへの指示: LoreのコアライブラリがRustであるため、バックエンドはRustを採用し、Loreのクレート（ライブラリ）を直接呼び出して最速でメタデータや履歴を取得するアーキテクチャとする。
- **Storage Interface**: S3 API (MinIO連携用)
- **3D Viewer (Web)**: Three.js (`@react-three/fiber`)

### 4.3. Key Features & Implementation Notes

- **Chunk-based Streaming Viewer**: 巨大なバイナリファイル（数十GB）をブラウザでダウンロードせずにプレビューするため、バックエンド(Rust)でチャンクを結合し、必要な部分だけをフロントエンドにストリーミングする設計。
- **Web-based File Locking**: Web画面上から特定のファイルやディレクトリにロックをかけるAPIエンドポイントの実装。

画面構成・IA(サイトマップ)の詳細は [`LOREHUB_UI_SPEC.md`](./LOREHUB_UI_SPEC.md) を参照。

## 5. Architectural Data Flow & Guidelines for AI

### 5.1. Data Flow (Web ↔ Server ↔ Client)

1. **Storage**: 全ての実データ（チャンク化されたバイナリ）はMinIO（S3互換ストレージ）に不変データとして保存される。
2. **State & Metadata**: ロック情報やコミットハッシュはLore Server (Rust) またはバックエンドのDBで管理される。
3. **Communication**: Client(C++) および Web Backend(Rust) は、Loreの標準プロトコルまたはRPC経由でLore Serverと通信する。

### 5.2. AI Coding Guidelines

- プロンプトを受け取った際は、必ず「アーキテクチャのどの部分（Client, Server Admin, Web）の実装か」を確認し、文脈を維持すること。
- C++のコードを生成する際は、Qt 6のモダンな機能（スマートポインタ、シグナル＆スロット、非同期処理のQFutureなど）を積極的に用いること。
- UIを生成する際は、可能な限りロジック（C++）とビュー（QML / React）を分離する設計パターン（MVVMアーキテクチャ等）を厳守すること。
- ビジュアルトークン（色・タイポグラフィ・角丸・余白・シャドウ）は3コンポーネント共通で [`DESIGN.md`](./DESIGN.md) を単一のソース・オブ・トゥルースとする。Web実装はCSS変数/Tailwind theme、Qt/QML実装は同ファイルのCross-Platform Token Mappingを参照すること。
