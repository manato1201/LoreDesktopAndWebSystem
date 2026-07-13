# LoreHub UI Specification (Web)

> LoreHub(Webサービス)の画面設計・IA(情報設計)を定義する。ビジュアルトークンは [`DESIGN.md`](./DESIGN.md)、機能要件は [`ARCHITECTURE.md`](./ARCHITECTURE.md) §4、実装時の品質基準は [`QUALITY_STANDARDS.md`](./QUALITY_STANDARDS.md) を参照。

## 1. サイトマップ / 画面一覧

| # | 画面 | 概要 | 対応するARCHITECTURE.mdの機能 |
|---|---|---|---|
| 1 | Organization / Repository一覧 | ログイン後のホーム。所属Organization横断でリポジトリをカード/リスト表示 | — |
| 2 | Repository Tree(ファイルブラウザ) | ディレクトリ・ファイルツリー、Sparse Workspace相当の閲覧、ロック状態表示 | Sparse Workspace, Visual Lock Indicator |
| 3 | File Detail / Preview | 選択ファイルのプレビュー(テキスト/画像/3Dモデル)、メタデータ、ロック操作 | Chunk-based Streaming Viewer, Web-based File Locking |
| 4 | Commit History | コミット一覧、コミット詳細(変更ファイル一覧) | — |
| 5 | Pull Request一覧 | Open/Merged/Closedのフィルタ付き一覧 | — |
| 6 | Pull Request 詳細 (Diff Review) | テキストDiff、画像Before/Afterスライダー、3Dモデル差分ビューア、コメントスレッド | Binary Diff Viewer(Web版) |
| 7 | Access Control(権限管理) | ディレクトリ単位の権限設定UI | Visual Access Control(Web版) |
| 8 | Organization設定 | メンバー管理、ストレージ使用量、監査ログ | — |

Sparse Workspace / Visual Lock Indicator / Binary Diff Viewer / Visual Access Control は元々LoreForge Client(デスクトップ)向け機能として定義されているが、LoreHubではブラウザ完結の閲覧・軽量操作版として提供する(重い編集操作はデスクトップクライアント誘導)。

## 2. レイアウト方針

`DESIGN.md` の「Dark compression(高密度・シンプル階層)」思想をダッシュボードUIへ適用する。

- **共通シェル**: 左サイドバー(Organization/Repository切り替え、ナビゲーション) + メインコンテンツ。`DESIGN.md` §4 Navigation のスタイル(`#121212`背景、active=weight 700 white、inactive=weight 400 `#b3b3b3`)を踏襲。
- **Repository Tree**: 左ペインにツリー(幅可変、リサイズ可)、右ペインにFile Detail。GitHubの「Code」タブに近いが、ツリー上のロックアイコンオーバーレイ(`DESIGN.md`のCircular要素を流用したバッジ)を常時表示する点が差分。
- **File Detail / Preview**: プレビュー領域は最大幅を確保しチャンクストリーミングのローディング状態(スケルトン)を明示。3Dプレビューは `@react-three/fiber` canvas をカード(`--radius-comfortable` 8px, `--shadow-medium`)内に配置。
- **Diff Review**: 左右2カラム(または重ね合わせスライダー)。画像diffは横スライダー(Before/After)、3Dモデルdiffは同一カメラ視点での並列表示 + オーバーレイ切替タブ。テキストdiffは行番号付き2カラム。
- **Access Control**: ディレクトリツリー + 選択ノードに対する権限マトリクス(Read/Write/Lock)をパネル表示。デスクトップ版の「ノードエディタ風UI」はWebでは簡略化し、ツリー選択+パネル編集の形に落とす(ブラウザでのノードエディタ実装は今回のスコープ外)。

## 3. コンポーネントインベントリ(初期セット)

`DESIGN.md` §4 のトークンをベースに、Tailwind componentとして最低限以下を定義する:

- Sidebar Nav Item(active/inactive)
- Repository Card
- Tree Node(ファイル/フォルダ、ロックバッジ付き)
- Lock Badge(誰がロックしているかのアバター+ツールチップ)
- Diff Slider(画像Before/After)
- Diff Viewport(3Dモデル、Three.js canvasラッパー)
- PR Status Pill(Open=Green `#1ed760`, Merged=Announcement Blue `#539df5`, Closed=Negative Red `#f3727f`)
- Permission Matrix Row
- Streaming Progress Indicator(チャンク読み込み中の状態表示)

## 4. レスポンシブ方針

`DESIGN.md` §8 のブレークポイント表を基準に、LoreHub画面ごとの折りたたみ挙動を以下に定義する。

| 画面 | Desktop(≥1024px) | Tablet(768–1024px) | Mobile(<768px) |
|---|---|---|---|
| Repository一覧 | グリッド3–4列 | グリッド2列 | 1列リスト |
| Repository Tree + File Detail | 左右2ペイン常時表示 | ツリーは折りたたみ式ドロワー | ツリー/詳細をタブ切り替え(同時表示しない) |
| Diff Review | 左右2カラム | 左右2カラム(幅縮小) | 縦積み(上下タブ切り替え) |
| Access Control | ツリー+パネル2カラム | 同上 | ツリー→パネルの2ステップ遷移 |

LoreHubは閲覧・レビューが主目的のダッシュボードのため、モバイル対応は「完全な編集操作」ではなく「閲覧・簡易確認・コメント」に機能を絞る。

## 5. 主要フロー(概要)

1. **バイナリアセットの差分レビュー**: PR一覧 → PR詳細 → 対象ファイルを選択 → Diff Viewer(画像/3Dモデルの場合はストリーミングプレビュー) → コメント投稿。
2. **ファイルロック**: Repository Tree上でファイル/ディレクトリを選択 → ロック操作ボタン → APIでロック状態を更新 → ツリー上のバッジが即時反映(楽観的UI更新 + WebSocketまたはポーリングでの他ユーザー反映)。
3. **権限設定**: Access Control画面 → ディレクトリノード選択 → 権限マトリクス編集 → 保存(バックエンドがLoreサーバー設定ファイルを生成・適用する処理は `ARCHITECTURE.md` §5.1のデータフローに従う)。
