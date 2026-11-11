# DESIGN.md — LoreHub(Web)の見た目の決めごと

新しい画面や部品を足す(AI に頼む場合も)ときは、この文書を渡す。**色・角丸・余白のトークンは `../design-tokens/tokens.json` が唯一の情報源**で、`node tools/gen-tokens.mjs` が `src/app/tokens.generated.css`(Web)と QML(Qt クライアント)を生成する。**生成されたファイルを手で直さない**。仕様は [../LOREHUB_UI_SPEC.md](../LOREHUB_UI_SPEC.md)・[../QUALITY_STANDARDS.md](../QUALITY_STANDARDS.md)。このリポジトリでは、Next.js の規約が通常と違うことがある(`AGENTS.md`)。

背景: [QUALITY_UPLIFT_2026-11-09.md](../../QUALITY_UPLIFT_2026-11-09.md)(QL1〜QL3)。共通の考え方は [Web/Portfolio/DESIGN.md](../../Web/Portfolio/DESIGN.md)。LoreHub は**運用のツール**(層 G0〜G1 + 見せ場 1 つまで)。**管理・設定の画面に重い演出を置かない**。見せ場は 3D モデルの差分(`ModelViewer`)。

## 1. 方針

- **ダーク 1 テーマ**(濃いグレーの地・緑の差し色)。ライトテーマは持たない。
- 緑(`--color-accent`)は**主な操作と、正常・追加**だけに使う。**緑の塗りの上の文字は暗いインク**(`text-bg-base`。白にしない)。
- 色だけで区別しない(変更の種類は文字のバッジを併記)。

## 2. 色

| トークン                                                        | 値                                | 用途                          |
| --------------------------------------------------------------- | --------------------------------- | ----------------------------- |
| `--color-bg-base`                                               | `#121212`                         | 地                            |
| `--color-surface` / `-interactive` / `-elevated`                | `#181818` / `#1f1f1f` / `#252525` | 面(段が上がるほど明るい)      |
| `--color-text-primary`                                          | `#ffffff`                         | 本文                          |
| `--color-text-secondary`                                        | `#b3b3b3`                         | 補助の文字                    |
| `--color-accent` / `-border`                                    | `#1ed760` / `#1db954`             | 差し色(塗り・文字・枠)        |
| `--color-negative` / `--color-warning` / `--color-announcement` | `#f3727f` / `#ffa42b` / `#539df5` | 不正・注意・お知らせ          |
| `--color-border` / `--color-border-light`                       | `#4d4d4d` / `#7c7c7c`             | 罫線 / **操作できる部品の枠** |

### 2.1 コントラスト(2026-11-11)

基準: 通常の文字 4.5:1 以上、UI 部品の枠・図形 3:1 以上(WCAG 2.x AA)。**トークンの全組み合わせを計算**した(文字 × 4 つの面)。

| 組み合わせ                                         | 最悪の面(`elevated`)での値                       |
| -------------------------------------------------- | ------------------------------------------------ |
| `text-primary`                                     | 15.33:1                                          |
| `text-secondary`                                   | 7.31:1                                           |
| `accent` / `negative` / `warning` / `announcement` | 7.99 / 5.49 / 7.74 / 5.48:1                      |
| `border-light`(操作できる部品の枠)                 | 3.67:1(基準 3:1 以上)                            |
| 緑の塗りの上の暗い文字(`text-bg-base` on `accent`) | 9.76:1(白だと 1.92:1 で**不可**)                 |
| `border`(装飾の区切り)                             | 1.8〜2.2:1(**装飾だけ**。操作の境界には使わない) |

**実ページの測定**: 公開ページ(`/login`・`/forgot-password`)の全テキストを測った。**基準未満は 0 件**(最小 8.47:1)。**認証が要る画面(リポジトリ・差分・設定・権限)は、バックエンドが要るため測っていない**。トークンの保証と、`bg-accent` の上の文字がすべて `text-bg-base` / `text-black` であることの確認で補った。

## 3. 動き

1. 動きは**不透明度の遷移**(`transition-opacity`)と、読み込み中の `animate-pulse` だけ。**新しい動きは、`transform` と `opacity` だけ**で、100〜300ms。**無限ループは足さない**(読み込み中を除く)。
2. **動きを減らす**: `animate-pulse` には `motion-reduce:animate-none` を付ける(**2026-11-11 に追加**。新しい読み込み表示にも付ける)。ASCII ツリーは、もともと動かない表示(`AsciiTree.tsx`)。
3. 見せ場(`ModelViewer`)は、three.js の 3D。管理・設定の画面には置かない。

## 4. アイコン

- **マーク**: 暗い角丸の正方形に、太さ 2.6 の「L」と、緑の点(コミットの印)。**1 か所で定義**: `src/lib/mark.tsx`(`MarkSvg`)。`src/app/icon.svg` は同じ図形で、ブラウザが暗いときは明暗が反転する。
- **一式**: `icon.svg`・`favicon.ico`(16 / 32 / 48px。**既定の Next のものは置き換えた**)・`apple-icon`(180px)。
- **作らないもの**: **OGP と PWA のマニフェスト**は、認証が要る内部ツールなので作らない(共有されず、ホーム画面に追加するものでもない)。
- **`public/`**: テンプレートの SVG は削除した。**`public/.gitkeep` は消さない**(`Dockerfile` が `COPY ... /app/public` するため、ディレクトリが無いとイメージのビルドが失敗する)。

## 5. 検査

| 何を                     | どう                                                                                               |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| 型・lint・テスト・ビルド | `npx tsc --noEmit` / `npx eslint src` / `npm test`(44 件)/ `npx next build`                        |
| コントラスト             | トークンの全組み合わせの計算(上記)+ 公開ページの実測(測定中は `transition` / `animation` を止める) |
| アイコン                 | `/icon.svg`・`/favicon.ico`・`/apple-icon` が 200 で画像が正しいこと。`<head>` のリンク            |
| **人の目**               | **「格好良いか」は人が決める**                                                                     |

**未実施**: 認証が要る画面の実測・機械的な UI の点検ツール(impeccable の `detect`)・実機のタブとホーム画面でのアイコン確認。
