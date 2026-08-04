# LoreDesktopAndWebSystem

LoreのGUI操作ができるアプリケーション及びwebサービスの開発を目的としたリポジトリ

架空の「巨大バイナリ対応の次世代VCS」**Lore** を中心に、3つの独立コンポーネントで構成される。

| コンポーネント | 役割 | 立ち位置の例え |
|---|---|---|
| [`lorehub-web`](lorehub-web/) | ブラウザからリポジトリ閲覧・PR・権限管理 | GitHub / GitLab |
| [`lorehub-api`](lorehub-api/) | 上記2クライアントが共有するRust(Axum)バックエンド | — |
| [`loreforge-client`](loreforge-client/) | デスクトップのVCSクライアント(Qt6/QML/C++20) | Fork / GitKraken |
| [`loreforge-server-admin`](loreforge-server-admin/) | Loreサーバー環境をGUIで構築・管理(Qt6/QML/C++20) | Docker Desktop + 権限エディタ |

## 主な機能

- **実コンテンツアドレス方式のVCS**: `lorehub-api` はSHA-256による実ハッシュ化・重複排除・全バージョン履歴保持・ブランチ分岐・行レベルdiffを備えた正規化SQLバックエンドで動作する(以前はコミットハッシュがランダム値、ツリーが全リポジトリ共有の静的スナップショットという「シミュレーション」だった。詳細は `docs/ARCHITECTURE_AND_DESIGN.md` §5)。
- **セキュリティ**: パスベースACL(ディレクトリ単位、祖先探索方式)の実強制、組織操作(リポジトリ削除・ロール変更・招待・ACL適用)のRBACゲート、セキュアCookie・CORSオリジン制限・リクエストボディ上限・ログインのIPレート制限。
- **アカウントライフサイクル**: 自己サービスのパスワード変更、メンバー招待(承認制・自動ログイン付き)、パスワード忘れ/リセット(メール送信、SMTP未設定時はログ出力にフォールバック)。
- **デプロイ**: `docker-compose.yml` によるRust/Next.jsの2サービス構成、GitHub Actions CI、`/api/health` ヘルスチェック、Vercel等クロスドメイン配置向けのAPIプロキシ(`NEXT_PUBLIC_USE_API_PROXY`)。詳細は `docs/DEPLOYMENT.md`。
- **可観測性**: `/metrics`(Prometheus形式)とJSON構造化ログ(`LOREHUB_LOG_FORMAT=json`)。
- **デスクトップ配布**: LoreForge Client/Server AdminともWindowsインストーラー(.exe、NSIS/CPack)を生成可能。詳細は `docs/DESKTOP_PACKAGING.md`。
- **自動テスト**: `lorehub-api`(統合テスト118件)・`lorehub-web`(Vitest 22件)・LoreForge Client/Server Admin(Qt Test)の4系統。

## ドキュメント

- [`docs/ARCHITECTURE_AND_DESIGN.md`](docs/ARCHITECTURE_AND_DESIGN.md) — アーキテクチャ・データフロー・設計判断(Mermaid図)
- [`docs/TECHNICAL_REFERENCE.md`](docs/TECHNICAL_REFERENCE.md) — セットアップ・APIリファレンス・実装詳細
- [`docs/LECTURE_NOTES.md`](docs/LECTURE_NOTES.md) — 設計判断の「なぜ」を学ぶ講義資料
- [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) — Docker Composeでのデプロイ手順
- [`docs/DESKTOP_PACKAGING.md`](docs/DESKTOP_PACKAGING.md) — デスクトップアプリのインストーラー生成
- [`docs/lore-ecosystem-docs.html`](docs/lore-ecosystem-docs.html) — 上記を1ページに集約したスタンドアロンHTML資料
