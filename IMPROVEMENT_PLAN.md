# LoreDesktopAndWebSystem 改善・リファクタリング計画書

**改善指標: 拡張性・耐障害性・スケールアウト耐性の強化**
作成日: 2026-08-11 / 調査範囲: 4コンポーネント(loreforge-client, loreforge-server-admin, lorehub-api, lorehub-web)

ワークスペース内13文書中もっともコンポーネント数が多い(4つの独立アプリ+共有バックエンド)ため、フェーズも他文書より多い。各フェーズは対象コンポーネントを明記し、無関係なコンポーネントには手を入れない。

---

## Phase 0: 現状分析(調査済み)

### コンポーネント構成

| コンポーネント | 技術スタック | 役割 |
|---|---|---|
| `lorehub-web` | Next.js 16 + React 19 + TypeScript | GitHub風リポジトリブラウザ、PR差分レビュー、アクセス制御UI |
| `lorehub-api` | Rust + Axum 0.8 + tokio + sqlx/SQLite + argon2 | Web/Desktop共有バックエンド |
| `loreforge-client` | Qt 6 / QML / C++20 | デスクトップVCSクライアント(Fork/GitKraken的) |
| `loreforge-server-admin` | Qt 6 / QML / C++20 | サーバー+Docker/MinIOストレージ管理アプリ |

インフラは`docker-compose.yml`(`lorehub-api`+`lorehub-web`の2サービスのみ。デスクトップ2アプリはCompose対象外)、CIは`.github/workflows/ci.yml`(1ワークフロー、`lorehub-api`ジョブ=cargo test/clippy/fmt と `lorehub-web`ジョブ=npm build/lint/test の2ジョブのみ。`loreforge-client`/`loreforge-server-admin`はCI未カバー)。

### `lorehub-api/src/` の分離済みモジュールと`state.rs`集中の実態

| ファイル | 役割 | 規模 |
|---|---|---|
| `state.rs` | `AppContext`(`SqlitePool`保持)+`AppState`(kv_store由来の12フィールド) | 566行 |
| `handlers.rs` | 全APIハンドラ。`ctx.read()`/`ctx.write()`直呼び出しが46箇所 | 2168行 |
| `vcs_store.rs` | `branches`/`commits`/`tree_entries`等VCSコアロジック | 1071行 |
| `repo_store.rs` | `repositories`テーブルCRUD。`AppState`非経由の自由関数群 | 226行 |
| `blob_meta_store.rs` | `file_blobs`/`staged_content`テーブルCRUD。`AppState`非経由 | 91行 |
| `blob_store.rs` | ファイルシステムBLOB本体(`blobs/{repo}/{hash[..2]}/{hash}`)。現状`&[u8]`全量バッファ方式 | 175行 |
| `lock_store.rs` | `file_locks`テーブルCRUD。`AppState`非経由 | 67行 |
| `db.rs` | マイグレーション、kv_store⇄SQL両persistenceの起点 | 331行 |

他8ファイル(`auth.rs`/`authz.rs`/`content_hash.rs`/`email.rs`/`image_assets.rs`/`main.rs`/`models.rs`/`vcs_seed.rs`)含め計16ファイル。`db.rs`のコメントが明言する通り、`repositories`/`branches`/`commits`/`file_locks`等のVCSコアデータは既に正規化されたSQLテーブルとして`repo_store.rs`/`vcs_store.rs`/`lock_store.rs`/`blob_meta_store.rs`から`AppState`のロックなしで直接読み書きされている。trait境界がなく単一構造体に集中しているのは、認証セッション・PR・アクセス制御・組織メンバー等が`kv_store`テーブルへJSON blobとして1フィールド1行で保存される`AppState`側であり、これは`db.rs`のコメントに「VCS再設計のスコープ外という意図的な判断」と明記されている(=バグではなく既知の負債)。`state.rs`の`demo_repositories()`は184GB〜1.1TB規模のリポジトリサイズラベルを保持しており、`blob_store::save_blob`/`load_blob`の全量バッファ方式が本番相当データでは非現実的であることを裏付ける。

### ドキュメント基盤とテスト資産

ルートに`ARCHITECTURE.md`(AI向け実装指示含む一次情報源)/`DESIGN.md`(§10 Cross-Platform Token Mapping: LoreHub=CSS変数/Tailwind、LoreForge Client・Server Admin=QML `Theme.qml`シングルトン。両アプリに実在: `loreforge-client/qml/Theme.qml`, `loreforge-server-admin/qml/Theme.qml`)/`LOREHUB_UI_SPEC.md`/`QUALITY_STANDARDS.md`。ECS(`archetype`/`ecs::`)・サービスコンテナ・RPC(`jsonrpc`/`grpc`)・タグ機能・静的解析サブシステムはリポジトリ全体grepで該当ゼロ(未実装確認済み)。テストは118バックエンド統合テスト(`lorehub-api/src/tests/`、`access_control.rs`〜`vcs.rs`の18ファイル)+22 Vitestテスト(`lorehub-web/src/`)+Qt Testスイート(`loreforge-client/tests/tst_repositorytreemodel.cpp`, `loreforge-server-admin/tests/tst_permissionconfigcontroller.cpp`)、総194ファイルの成熟した多サービスシステム。

### アンチパターン(全フェーズ共通)

- `ARCHITECTURE.md`のAI向け指示にある「`lore` CLIラッパー(QProcess)は初期実装、Rust FFIは将来」という段階移行方針を飛び級しない
- `DESIGN.md`をフォークしない。3コンポーネント共通の単一ソースのまま拡張する
- 静的解析サブシステム(Phase 6)は`lorehub.db`/`blobs/`に一切書き込まない
- `AppState`のkv_store方式(12フィールド)を「壊れているから即座に正規化」と早合点しない。VCSコアは既にSQL化済みであり、残るkv_store側はスコープ外という意図的判断(`db.rs`コメント)

---

## Phase 1: サービスコンテナ化(最優先・後続フェーズの安全網)

**現状:** `repo_store.rs`/`lock_store.rs`/`blob_meta_store.rs`は既に`&SqlitePool`を受け取る自由関数群として`AppState`を経由せず直接SQL CRUDしている。一方`handlers.rs`は46箇所で`ctx.read()`/`ctx.write()`により`AppState`(単一巨大構造体)へ直接アクセスしており、trait境界がない。

**実装内容:**
1. `lorehub-api/src/store_traits.rs`(新設)に既存自由関数群をtrait化:
   ```rust
   #[async_trait]
   pub trait RepoStore: Send + Sync {
       async fn list(&self) -> Vec<Repository>;
       async fn get(&self, slug: &str) -> Option<Repository>;
   }
   #[async_trait]
   pub trait LockStore: Send + Sync {
       async fn list_locks(&self, repo_slug: &str) -> HashMap<String, String>;
       async fn lock(&self, repo_slug: &str, path: &str, locked_by: &str);
       async fn unlock(&self, repo_slug: &str, path: &str);
   }
   #[async_trait]
   pub trait BlobStore: Send + Sync {
       async fn save(&self, repo_slug: &str, hash: &str, bytes: &[u8]) -> std::io::Result<()>;
       async fn load(&self, repo_slug: &str, hash: &str) -> std::io::Result<Vec<u8>>;
   }
   ```
2. `SqliteRepoStore`/`SqliteLockStore`/`FsBlobStore`は既存の`repo_store.rs`/`lock_store.rs`/`blob_store.rs`の関数をそのまま呼ぶ薄いラッパーとして実装する(ロジック変更なし)
3. `state.rs`の`AppContext`に`Arc<dyn RepoStore>`/`Arc<dyn LockStore>`/`Arc<dyn BlobStore>`フィールドを追加し、既存の`db: SqlitePool`と併存させる。handlersは段階的にtrait経由へ移行し、一括置換はしない
4. Qt/C++側(`loreforge-client`/`loreforge-server-admin`): 既存`ApiClient::networkManager()`(プロセス唯一の静的`QNetworkAccessManager`)と同じ「プロセス唯一のstaticアクセサ」パターンを一般化した`ServiceRegistry`シングルトンを追加し、`Main.qml`へは`Theme.qml`同様のcontext propertyとして公開する
5. Phase 8で差し替える対象を今回は登録しない。SQLite実装のみ登録し、実装バリエーションは増やさない

**検証チェックリスト:**
- [ ] `lorehub-api/src/tests/`の118テストがtrait抽出後も無変更でパス
- [ ] `SqliteRepoStore`等が既存関数を呼ぶだけで独自ロジックを持たないことをレビューで確認
- [ ] `ServiceRegistry`経由で`DockerController`/`LoreServerController`がQML側から呼び出せる
- [ ] `cargo clippy --all-targets -- -D warnings`(既存CIゲート)が引き続きパス

**アンチパターン:** trait抽出を「実装差し替えの実演」まで広げない。モックやTiDB版実装は本フェーズで作らない。境界を作ることのみがゴール。

---

## Phase 2: Archetype ECS導入(アセットプレビュー・Diffパイプライン限定)

**現状:** `loreforge-client/src/RepositoryTreeModel.h`の`TreeItem`は既に`kind`(`"directory"|"text"|"image"|"model3d"|"audio"|"binary"`)と`lockedBy`を保持しており、Workspace Managerツリーの実データそのもの。Binary Diff Viewer(画像スライダー/3Dモデル差分)は`ARCHITECTURE.md` §2.3で設計確定済みだが、プレビュー種別ごとのバッチ処理層は未実装。

**実装内容:**
1. 対象を明確に限定する: `Main.qml`/`LoginScreen.qml`/`RepositoryListScreen.qml`等の「ウィンドウ/ダイアログ」層は既存OOP(`QAbstractListModel`派生)のまま touch しない。ECS化するのは(a) Workspace Managerツリー内のプレビュー対象アセット群、(b) Binary Diff Viewerの描画対象、の2つのみ
2. `TreeItem::kind`の値をそのままアーキタイプの分割軸に転用する:
   ```cpp
   // loreforge-client/src/preview/PreviewArchetypes.h (新設)
   struct Model3DPreviewArchetype {
       QVector<QString> paths, contentHashes;
       QVector<GpuMeshHandle> cachedMeshes;   // SoA: バッチロード・バッチdiff用
   };
   struct Image2DPreviewArchetype {
       QVector<QString> paths;
       QVector<QImage> beforeAfterPairs;
   };
   struct LockedAssetArchetype {
       QVector<QString> paths, lockedBy;      // TreeItem::lockedByを転写
   };
   ```
3. 既存`QOpenGLWidget`/Qt 3D実装はアーカイブせず、`PreviewArchetypeSystem::renderBatch()`が同一アーカイブ内エンティティをまとめて既存レンダラへ描画命令として発行する形にラップする(レンダラ差し替えではなく呼び出し単位の変更)
4. 自動並列化の安全性: 同一アーカイブ内エンティティはPhase 3のジョブシステムでバッチ並列処理してよいが、アーカイブを跨ぐGPUリソース共有書き込みがないことを設計時点で保証する

**検証チェックリスト:**
- [ ] `TreeItem::kind`の値とアーキタイプ種別が1:1対応
- [ ] 既存`tst_repositorytreemodel.cpp`が無変更でパス(ツリーモデル自体は非ECSのまま)
- [ ] ウィンドウ/ダイアログ層コードにECS関連の新規依存が増えていないことをgrepで確認
- [ ] 100アセット規模のダミーツリーでバッチdiff所要時間を計測・記録(Phase 3並列化のベースライン)

**アンチパターン:** ECSの対象をWorkspace Manager全体やアプリ全体に広げない。ウィンドウ/ダイアログ層のOOP設計(ユーザー要望の「併用」)を崩さない。

---

## Phase 3: ジョブシステム(並列化基盤・依存グラフ可視化)

**現状:** `blob_store::save_blob`/`load_blob`は`&[u8]`全量バッファのread/write。`demo_repositories()`が示す184GB〜1.1TB規模では非現実的。

**実装内容:**
1. Rust側(`lorehub-api`): `blob_store.rs`に`save_blob_chunked`/`load_blob_chunked`を追加(既存の全量版は互換維持のため残す)。並列度は`rayon`か自作ジョブキューかを選定する(既存依存に`rayon`がなければ依存追加を避け自作の軽量キューを優先)
2. C++側(`loreforge-client`/`loreforge-server-admin`): `QThreadPool`ベースの依存グラフ`JobGraph`を新設:
   ```cpp
   class JobGraph {
   public:
       JobId addJob(std::function<void()> work, QVector<JobId> dependsOn = {});
       void run(QThreadPool* pool);
       QVariantList toVisualizationModel() const;  // ノード/エッジをQML/Webへそのまま渡す
   };
   ```
3. 対象ジョブ: chunkベースblobアップロード/ダウンロード、Phase 2アーキタイプ単位のDiffバッチ計算、Phase 6静的解析スキャン。GPU排他を要する処理(Phase 2の描画バッチ)はジョブキューの無制限並列に委ねず、CPUバウンド処理と明確に分離する
4. 依存グラフ可視化: `JobGraph::toVisualizationModel()`の出力をQML側とWeb側(lorehub-web)双方でそのまま描画する。可視化専用の別ロジックは新規に作らない
5. 失敗/リトライ方針: Phase 1の`ServiceRegistry`/trait経由で各ストア呼び出しを注入しているため、リトライは同一trait実装への再呼び出しで完結し、フェイク実装でのリトライテストが書ける

**検証チェックリスト:**
- [ ] 1GB相当のダミーバイナリでchunked upload/downloadが全量版と同一`content_hash`を生成
- [ ] `JobGraph`が循環依存を検出しエラーとする(DAG違反の防止)
- [ ] Phase 2のバッチdiff計算がジョブ化後もPhase 2ベースライン以下の所要時間で完走
- [ ] `toVisualizationModel()`の出力をQML側で表示し依存エッジが視覚的に確認できる

**アンチパターン:** GPUリースを要する処理をジョブキューの無制限並列に委ねない。

---

## Phase 4: プロセス分離(リアルタイム編集のクラッシュ耐性)

**要件:** 編集中のクラッシュがアプリ全体を道連れにしない、かつ編集中データが生存すること。現状`loreforge-client`は単一プロセスで、`RepositoryTreeModel`のステージング状態(`m_pendingChanges`)はプロセスメモリ上にのみ存在する。

**実装内容:**
1. `loreforge-client`をUIシェルプロセス(既存`main.cpp`/QML/Modelスタック)とアセットプレビュー/プラグインのワーカープロセス(新設`loreforge-worker`)に分割。Phase 2の`PreviewArchetypeSystem`とPhase 3の`JobGraph`はワーカー側へ移設し、GPU描画・重い解析処理をシェルから隔離する
2. IPC: `QLocalSocket`(同一マシン内)でシェル-ワーカー間を接続
3. ワーカーはコミット前のステージング内容をWAL的ジャーナルへ逐次書き込む:
   ```
   %LOCALAPPDATA%/LoreForge/journal/{repo_slug}/{session_id}.wal
     各行: {"seq": 1, "path": "Assets/hero.fbx", "changeType": "modified", "ts": "..."}
   ```
4. シェル再起動後、journalファイルの存在を検知し、最終commit済みseqより後のエントリのみをリプレイして`m_pendingChanges`を復元する(commit済みseqより前は二重適用防止のため再生しない)
5. 別途執筆される「SoundMiddleware設計書」も同一理由(リアルタイム編集中のクラッシュ耐性)でプロセス分離を要求しており、本フェーズのシェル/ワーカー分割+WALジャーナル設計はそちらでも直接引用されるモデルである

**検証チェックリスト:**
- [ ] ワーカープロセスを強制終了してもシェルプロセスが生存し続ける
- [ ] 強制終了直前のステージング変更が、シェル再起動後のjournalリプレイで復元される
- [ ] 正常系のcommit完了後、journalファイルが該当seqまで切り詰められる(無限成長しない)
- [ ] `QLocalSocket`切断を検知し、シェル側がワーカー再起動を自動試行する

**アンチパターン:** ワーカークラッシュのたびにjournal全体を再生しない。

---

## Phase 5: エクスプローラー/アセットブラウザのフィルタ機能(小規模・Phase 7の前提整備)

**現状:** `RepositoryTreeModel`は`kind`/`lockedBy`/`included`を既にデータとして保持しているが、QML側のフィルタUIは未実装。

**実装内容:**
1. `RepositoryTreeModel`に`Q_PROPERTY`ベースのフィルタ条件(種別/ロック状態/タグ)を追加し、`rebuildFlatRows()`にフィルタ適用ステップを挿入する
2. フィルタUIを`RepositoryWorkspaceScreen.qml`にツールバーとして追加(種別チェックボックス群、ロック中のみ表示トグル)
3. Phase 7のタグ機能を前提にしておく: `FlatRow`に`QStringList tags`を先行追加し(Phase 5時点では常に空)、Phase 7でタグAPIが追加された時点でフィルタ条件に組み込むだけで済む形にする(後戻りしない)

**検証チェックリスト:**
- [ ] 種別フィルタ(image/model3d/audio等)でツリー表示が正しく絞り込まれる
- [ ] ロック中のみ表示トグルが`lockedBy`の非空判定と一致する
- [ ] `tags`フィールド追加後も既存`tst_repositorytreemodel.cpp`が無変更でパス

---

## Phase 6: 静的解析サブシステム(独自拡張子+別ツール+CI)

**制約:** (a) 製品データ非干渉 — `.lqa`(Lore Quality Analysis)サイドカーとして解析結果を保存し、`lorehub.db`/`blobs/`には一切書き込まない。(b) 経過の可視化 — `.lqa`はコミット/変更のたびに追記されるバージョン付きレコードとし、時系列はLore自体のバージョン履歴に相乗りさせる。(c) 拡張容易性 — `checks: { [checkId]: result }`形式の可変マップとし、固定structにしない。

**実装内容:**
1. サイドカー配置: `blobs/`と兄弟の新規ディレクトリ`{base_dir}/lqa/{repo_slug}/{path}.lqa.json`(パス構造をミラーし、`blob_store.rs`のfan-out規約とは物理的に独立させる)
2. スキーマ例(データサイズ・コメント情報・各種設定・変換エラー・ツールバージョンを記録):
   ```json
   {
     "lqa_version": "1.0",
     "asset_path": "Assets/Characters/hero_rig.fbx",
     "records": [
       {
         "commit_ref": "c_8f21a0",
         "generated_at": "2026-08-11T09:15:00Z",
         "generated_by": { "tool": "loreqa-cli", "tool_version": "0.3.1" },
         "data_size_bytes": 184320550,
         "settings_snapshot": { "mc_resolution": 256, "import_scale": 1.0 },
         "conversion_errors": [],
         "checks": {
           "polycount": { "status": "pass", "value": 48213, "threshold": 60000 },
           "texture_budget_mb": { "status": "warn", "value": 210, "threshold": 200 },
           "comment_density": { "status": "info", "comment_lines": 12, "total_lines": 340 },
           "naming_convention": { "status": "pass" }
         }
       }
     ]
   }
   ```
3. 別ツール: `loreqa-cli`(新規Rustバイナリクレート。既存4コンポーネントに次ぐ5本目の独立ツールとして、`lorehub-api`等のディレクトリには混在させない)。`content_hash.rs`と同じ`sha2`ベースの実装スタイルを踏襲し、新規チェックはプラグイン関数の追加のみで拡張できるようにする
4. CI: 既存`.github/workflows/ci.yml`(現状`lorehub-api`/`lorehub-web`の2ジョブのみ)には手を入れず、新規`.github/workflows/lqa-analysis.yml`を新設して`loreqa-cli`のビルド→変更ファイル解析→`.lqa`生成→スキーマ検証を実行する
5. 本フェーズの`.lqa`出力形式は、別途執筆される「AssetDataInsightSuite設計書」の入力契約の一次リファレンス実装である

**検証チェックリスト:**
- [ ] `loreqa-cli`実行前後で`lorehub.db`/`blobs/`に`git diff`相当の差分がゼロ
- [ ] 同一`asset_path`への2回目の実行で`records`に新規エントリが追記され、既存レコードは上書きされない
- [ ] 未知の`checkId`を追加してもスキーマ検証がエラーにならない(可変マップの実証)
- [ ] `lqa-analysis.yml`がPRごとにグリーン実行される

**アンチパターン:** `checks`の値を固定`struct`+`#[serde(deny_unknown_fields)]`のような閉じたスキーマにしない。新規チェック追加のたびにスキーマバージョンを上げる運用にしない。

---

## Phase 7: RAGアシスタント+タグ機能+システムレイヤー管理+外部エディタ連携(既存資産の再利用優先)

**実装内容:**
1. **RAGアシスタント**: `DevelopmentRAGEnvironment/scripts/rag_local_bridge.py`の既存HTTPブリッジ契約(`:8766`、`X-API-Key`ヘッダ認証、`/health`・`/admin`・`/ui`以外の全エンドポイントで認証必須)をそのまま再利用する。`lorehub-api`/`loreforge-client`双方から同一ブリッジへHTTPで問い合わせ、新規RAG基盤(別ベクトルDB・別ポート)は作らない
2. **タグ機能**: `lorehub-api`に新規`asset_tags`テーブル(`repo_slug`, `path`, `tag`)を追加し、`repo_store.rs`/`lock_store.rs`と同じ「`AppState`非経由の自由関数モジュール」パターンで`tag_store.rs`を新設。Phase 1の`TagStore` traitとして最初から抽出する。lorehub-web(タグ編集UI)・loreforge-client(Phase 5フィルタへ接続)双方に反映
3. **システムレイヤー管理**: 既存の`Visual Access Control`(`ARCHITECTURE.md` §3.3、`loreforge-server-admin/src/PermissionConfigController.h`が実体)を発展させ、複数のACLレイヤーを上から順に合成し最終的な実効権限を導出する、Perforce風ワークスペースビューに近いレイヤー合成モデルを定義する
4. **外部エディタ連携**: `QProcess`起動(選択アセットを既定/指定エディタで開く)+`QFileSystemWatcher`によるファイル変更監視(保存を検知しstageChange相当を提案)という標準パターンを`loreforge-client`に追加

**検証チェックリスト:**
- [ ] RAGアシスタントへのリクエストが`X-API-Key`ヘッダなしで401になる(既存ブリッジの認証仕様と一致)
- [ ] `asset_tags`追加後も`TagStore` traitがPhase 1の境界パターンに沿っている
- [ ] レイヤー合成モデルで下位レイヤーの権限が上位レイヤーに明示的に上書きされるケースをテストで確認
- [ ] 外部エディタでの保存後、`QFileSystemWatcher`のシグナルからPhase 5フィルタ済みツリーが再描画される

---

## Phase 8: TiDB Cloud/TiKV移行の設計(段階的・非破壊)

**実装内容:**
1. 本計画では実移行しない。Phase 1で確立した`RepoStore`/`LockStore`/`BlobStore`のtrait境界を使い、`db.rs`をSQLite実装のまま据え置きつつ将来差し替え可能にする設計のみ確定する
2. 使い分け基準:

   | データ種別 | 移行先 | 理由 |
   |---|---|---|
   | `repositories`/`commits`/`branches`/`pull_requests`等(メタデータ検索系) | TiDB Cloud(MySQL互換) | 複雑なJOIN・検索クエリ、スケールアウト前提 |
   | `blob_meta_store.rs`(`file_blobs`/`staged_content`)、kv_store由来のセッション/認証系 | TiKV(生KV) | 高頻度・単純key-valueアクセスが支配的、レイテンシ最優先 |
3. 移行手順: dual-write shadow mode(SQLiteとTiDB/TiKV双方に書き込み、読み取りはSQLiteのまま)で整合性検証 → 読み取りをカナリア的に段階移行 → 完全カットオーバー。ビッグバン移行はしない
4. kv_store方式は「VCS正規化のスコープ外」という`db.rs`の既存の意図的判断を踏まえ、TiKV移行はkv_store由来フィールドの正規化とセットで検討する(正規化なしでJSON blobをそのままTiKVへ移すだけでも一次的な効果はある)

**検証チェックリスト:**
- [ ] `RepoStore`/`LockStore`/`BlobStore`のtrait定義がTiDB/TiKV実装追加時にシグネチャ変更を要しないこと(設計レビューのみ、実装はしない)
- [ ] dual-write shadow modeの整合性検証手順(ハッシュ比較等)が具体的に文書化されている
- [ ] 移行対象テーブルのTiDB/TiKV振り分け表が`ARCHITECTURE.md`に追記されている

---

## Final Phase: 統合検証

- [ ] 4コンポーネント全てがビルド成功(`lorehub-api`/`lorehub-web`は`docker-compose up`、`loreforge-client`/`loreforge-server-admin`はCMakeビルド)
- [ ] Phase 1の`ServiceRegistry`/traitに対しフェイク実装(`FakeRepoStore`等)を差し替えたテストが通る
- [ ] Phase 4のクラッシュインジェクション(ワーカープロセス強制終了)後、シェルプロセスが生存しjournalリプレイでデータ復旧すること
- [ ] `loreqa-cli`実行後も`lorehub.db`/`blobs/`に`git diff`で差分がないこと
- [ ] `ARCHITECTURE.md`/`DESIGN.md`へ本計画で確定した設計(trait境界、ECS適用範囲、TiDB/TiKV振り分け)を追記済み

---

## 相互参照ドキュメント

- Phase 6の`.lqa`形式(独自拡張子サイドカー、`checks`可変マップ)は、別文書「AssetDataInsightSuite設計書」における入力契約の一次リファレンス実装であること
- Phase 4のプロセス分離(UIシェル+ワーカー、WALジャーナルによるクラッシュ耐性)は、別文書「SoundMiddleware設計書」が同一理由で要求するプロセス分離の直接のモデルであること
- Phase 7のRAGアシスタントは`DevelopmentRAGEnvironment/scripts/rag_local_bridge.py`の`:8766`ブリッジ(`X-API-Key`ヘッダ認証)をそのまま再利用し、新規RAG基盤は作らないこと

**優先度注記:** 本計画は13文書中最大スコープだが、`ARCHITECTURE.md`/`DESIGN.md`/`LOREHUB_UI_SPEC.md`/`QUALITY_STANDARDS.md`という最も厚いドキュメント基盤と194ファイル・118+22件のテスト資産の上に成り立つため、土台自体は最も整っている。ただしPhase 6の`.lqa`出力契約は「AssetDataInsightSuite設計書」側が前提として直接引用するため、実質的にワークスペース全体の進行を左右するボトルネックフェーズとして扱うこと。
