# Quality Standards (LoreHub Web)

> `web-production-skill`(ローカル参照資料、リポジトリには含めない/`.gitignore`対象)に含まれる実務基準のうち、LoreHub(ログイン後のSaaSダッシュボード)へ転用可能なものだけを抽出し明文化したもの。LP/受託案件特有の基準は意図的に除外している(理由は§0参照)。

## 0. 適用範囲と除外理由

LoreHubはマーケティング用ランディングページではなく、認証後に使う開発者向けダッシュボード(GitHub/GitLab相当)である。そのため `web-production-skill` の以下の内容は**非適用**として除外した:

- コンバージョン設計(FV3要素・CTA設計・問い合わせ導線) — 成果指標が「問い合わせ数」ではないため
- フォーム送信手段の選定(`forms.md`) — 該当する公開フォームがない
- SEO/GA4/GTM(`seo-analytics.md`) — 非公開・認証必須のアプリのため検索エンジン最適化は対象外
- WordPress化(`wordpress-theme.md`) — 静的CMSサイトではない
- 8方向のデザイン方向性・ヒーロー型レイアウト(`design-directions.md`, `layout-patterns.md`) — マーケティングサイトのビジュアル戦略であり、`DESIGN.md`(Spotify風トークン)が既にLoreHubのビジュアル方針を規定済み
- ショーケースモードのWebGL演出・スクロール一体型モーション(`webgl-showcase.md`, `motion-advanced.md`) — 装飾目的の演出であり、ダッシュボードでは可読性・操作性を優先する

一方、以下は普遍的なWeb品質基準としてそのまま転用する。

## 1. コーディング規約

- **CSSカスタムプロパティでトークン管理**: 色・余白・角丸をハードコードせず、`DESIGN.md`のCross-Platform Token Mappingで定義したCSS変数(`--color-*`, `--radius-*`, `--spacing-unit`等)を通す。Tailwind theme拡張もこの変数を参照する形にする。
- **BEM風命名**: Tailwindユーティリティで賄えないカスタムコンポーネント(Diffビューア、3Dプレビューシェル等)は `block__element--modifier` 命名を用いる。
- **z-indexの段階管理**: 10刻みで管理する(例: header=100, dropdown=200, modal=1000)。アドホックな`9999`指定を禁止。
- **画像**: `<img>` には常に `width` / `height` / `alt` を指定し、CLS(レイアウトシフト)を防ぐ。

## 2. アクセシビリティ(a11y)フロア

- キーボード操作のみで全機能に到達できること。`:focus-visible` のスタイルを絶対に消さない。
- インタラクティブ要素には適切な `aria-label` / `aria-expanded` 等を付与する。
- 本文テキストのコントラスト比は **4.5:1** を目安とする(WCAG AA相当)。

## 3. パフォーマンス目標

- Lighthouse(モバイル): **Performance 80+ / Accessibility 90+**(SEOスコアは非公開アプリのため対象外)
- **LCP 2.5秒以内 / CLS 0.1以内**
- アニメーションは `transform` / `opacity` のみで行い、`top`/`left`/`width`/`height`/`margin` を直接アニメーションしない。`will-change` は本当に重い要素1〜2個に限定する。

## 4. モーション方針

- `prefers-reduced-motion` には**自動対応**する(OS設定を尊重し、機能に支障のない範囲でモーションを縮小/無効化)。`web-production-skill`のショーケースモードにある「利用者が明示的に止めるボタン」方式はLoreHubでは採用しない(アクセシビリティ優先の業務アプリのため)。
- ホバー・パネル遷移・トースト等のマイクロインタラクションは、easingをトークン化(`--duration-fast`, `--ease-out-expo`等)して一貫した体感速度を保つ。
- IntersectionObserverによるスクロール演出・マーケティング的なリビールアニメーション・マーキー等は使用しない(ダッシュボードは即座に情報を提示するべきで、演出目的の遅延を作らない)。

## 5. セルフレビュー(「テンプレ感/AI感」回避)

`web-production-skill`の`self-review.md`にある一般的な「AI感」の兆候を、SaaSダッシュボードの文脈に言い換えて適用する:

- 紫〜青のグラデーションを多用しない(DESIGN.mdはSpotify Green以外は achromatic が原則)
- 全カード均等シャドウ・均等余白の「3カードレイアウト」を機械的に繰り返さない — 情報の重要度に応じて密度・強調を変える
- 全セクションが同一リズム(同じ余白・同じ見出しサイズ)にならないようにする — Repository一覧・Diff Review・Access Controlはそれぞれ異なる情報密度を持つべき
- ダミーテキスト・placeholder文言を残したまま実装完了としない

## 6. QA(公開前チェックの普遍項目)

- **ブレークポイント**: 375px / 768px / 1024px / 1440px でレイアウト崩れがないことを確認(モバイルファースト、`DESIGN.md`§8のブレークポイント表と整合させる)
- **コンソールエラー0**: DevTools上でエラー・警告が出ていないこと
- **ブラウザ確認**: Chrome / Safari(モバイル閲覧を許可する画面がある場合はiOS Safariも)
- **リンク/ナビゲーション**: 全てのナビゲーション項目・ボタンが正しい遷移先を持つこと
