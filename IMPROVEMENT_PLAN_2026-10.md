# LoreDesktopAndWebSystem 改善書(2026-10 X調査反映)

作成日: 2026-10-07 / 対象: 架空の巨大バイナリ対応VCS「Lore」のGUI(lorehub-web / lorehub-api(Rust・Axum)/ loreforge-client / loreforge-server-admin(Qt6/QML))
関連: [X_POSTS_BY_GENRE_2026-10.md](../X_POSTS_BY_GENRE_2026-10.md)、既存の [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md)、[ARCHITECTURE.md](ARCHITECTURE.md)

**前提**: Lore は巨大なバイナリ(Houdini / Unreal / Unity のアセット)を扱う想定のVCS。DCCとエンジン間の受け渡しの問題と、直接つながる題材。

## 改善項目一覧

| ID | ジャンル | 改善内容 | 出典 | 効果 | 工数 |
|---|---|---|---|---|---|
| L1 | G3 | LoreHub の PR 画面に「構造への影響」タブ: 変更した範囲の構造図の差分を表示 | CodeBoarding のPR連携 | レビューが速い | 大 |
| L2 | G3 | データモデルを「画面・概念図・ER図」で切り替えるデザイン / 仕様ビュー | mayugelion | 仕様の整合が取りやすい | 中 |
| L3 | G9 | バイナリアセット(FBX / USD / .hip)の追加時に、書き出し前診断を走らせるフック | 前回資料 | 壊れたアセットのコミット防止 | 中 |
| L4 | G5 | server-admin の画面に「等角投影のサーバールーム」ビュー(サービスの稼働状況) | kensuu | 状態が直感で分かる | 中 |
| L5 | G5 | 404 / 空リポジトリの画面に遊び(点群の魚)、コミット時のタップ波紋 | yoshifuji / yui540 | 愛着 | 小 |
| L6 | G2 | 自前ホストの隔離方針(docker-compose の各サービスの権限) | mania_3 | 事故の被害限定 | 小 |
| L7 | G3 | `lorehub-api`(Rust)の構造図を生成して `ARCHITECTURE.md` に取り込む | shiba_program(Rust対応) | ドキュメント更新の手間削減 | 小 |

## Phase 1: L7 / L6
- CodeBoarding は Rust に対応。`lorehub-api` に実行し、トップ階層の図だけを `ARCHITECTURE.md` に貼る。
- `docker-compose.yml` の各サービスに、不要な権限(特権・ホストマウント)がないか棚卸しする。

## Phase 2: L2 / L5
- 仕様の元データ(YAML か JSON)を1つにして、画面一覧・概念図・ER図を出し分けるビューを作る。LOREHUB_UI_SPEC.md の画面一覧と対応させる。

## Phase 3: L3
- Lore のコミットフック: アセット拡張子ごとに診断コマンドを呼び、警告を出す(ブロックはせず警告のみ。最初は様子見)。

## Phase 4: L1 / L4
- L1 は大きい。まず「変更ファイル → 所属コンポーネント」の対応表を出すだけの版から。
- L4 は Qt/QML の描画負荷を見て、静的なSVGを背景にする案から。

## Final Phase
- QUALITY_STANDARDS.md の基準に沿って、追加画面の品質チェックを実施する。

**検証チェックリスト:**
- [ ] 構造図の自動生成が、ビルドを壊さない(別ステップで実行)
- [ ] 仕様ビューの3つの切替が、同じ元データから出ている
- [ ] 診断フックが、正常なアセットで誤警告を出さない
- [ ] 演出が低スペック環境でも重くない

## 注意・未確認
- Lore は架空のVCSという位置づけ。実運用の話ではなく、設計の練習・検証として扱う。


---

## UI/デザイン改善(2026-10 追補)

参照: [UI_DESIGN_PRINCIPLES_2026-10.md](../UI_DESIGN_PRINCIPLES_2026-10.md)(原則 P1〜P10)/ 設計: [IMPROVEMENT_DESIGN_2026-10.md](IMPROVEMENT_DESIGN_2026-10.md)
追加の参考: 東京終電図・ONE ASIA ATLAS・文字画像APNGメーカー・中台の小さな家・re-presentation.jp と、デザインに関する2件の記事。
**方針**: 骨格(配色・書体・レイアウト)は維持し、「決めていないこと」を無くす方向で直す。装飾は足さない。

| ID | 原則 | 改善内容 | 工数 |
|---|---|---|---|
| U1 | P6 | リポジトリ画面を「左=ファイルツリー、中央=タブ(ファイル/コミット/PR/アクセス)、右=サマリ」に | 中 |
| U2 | P1/P2 | 配色トークンを、このプロジェクト自身のものへ段階的に置き換える(現状は緑アクセントの暗いテーマ) | 中 |
| U3 | P8 | 404・空リポジトリに遊び | 小 |
| U4 | P1 | Qt側(loreforge-client / server-admin)とWebでトークンを共有 | 中 |


### UIパーツ・Motion 追補(2026-10-11)

詳細は [IMPROVEMENT_DESIGN_2026-10.md](IMPROVEMENT_DESIGN_2026-10.md) の同名の節。
- **パーツ**: パンくず(18)、ドロワー(02)、モーダル(01)、ツールチップ(06)・トースト(08)、ページネーション(16)、エンプティステート(24)
- **動き**: スティッキー(17)
