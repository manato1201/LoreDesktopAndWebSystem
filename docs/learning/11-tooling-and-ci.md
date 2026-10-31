# 11. スクリプト・テスト・CI

**この章で分かること**: 「手で直すと壊れる」ものを**生成**する仕組み(トークンと図)を、スクリプトを読みながら理解します。5種類のテスト、CI が何を守るか、コミットの前に何を確かめるか。

前提: [00](00-start-here.md)(Node.js が使える)。JavaScript の `import`・関数・オブジェクトが読めれば十分です。

![デザイントークンの生成パイプライン](../diagrams/token-pipeline.svg)

## 11.1 なぜ「生成」するのか

同じ値を**2か所に手で書く**と、いつか食い違います。デザインの色は、Web の CSS と、2つの Qt アプリの `Theme.qml` の**3か所**にありました。人の注意力に頼らず、**元を1つに**して、他は**元から作る**(生成する)ようにしました。

| 元(編集する) | 生成物(編集しない) | 生成する道具 |
|---|---|---|
| `design-tokens/tokens.json` | `lorehub-web/src/app/tokens.generated.css`、両アプリの `qml/Theme.qml` | `node tools/gen-tokens.mjs` |
| `tools/diagrams/specs/*.mjs` | `docs/diagrams/*.svg`、`docs/lore-ecosystem-docs.html` の中の図 | `node tools/diagrams/build.mjs` |

**3つの約束**:

1. **生成物は手で直さない。** 直すのは元。
2. **生成物もコミットする。** 読む人・ビルドする人が、生成を意識しなくてよいように。
3. **CI が「元と生成物の食い違い」を検出する**(`--check`)。食い違い=誰かが元を直して再生成を忘れた、または生成物を手で触った。

## 11.2 `gen-tokens.mjs` を読む

[`tools/gen-tokens.mjs`](../../tools/gen-tokens.mjs) は100行足らずです。構造が大切で、**3つの層**に分かれています。

```
データ    design-tokens/tokens.json              ← 値だけ。ロジックは無い
純粋関数  renderCss(tokens) / renderQml(tokens)   ← 文字列を入れたら文字列が出る。読み書きしない
入出力    main()                                  ← ファイルを読んで、書く(または --check で比べる)
```

**純粋関数**(同じ入力から、いつも同じ出力。外の世界に触らない)にしておくと、**テストしやすく、動作が読みやすい**。入出力は、端に追いやります。

```js
export function renderCss(tokens) {
  const lines = [];
  for (const c of tokens.colors) lines.push(`  ${c.css}: ${c.value};`);
  /* ... radii, spacing, shadows ... */
  return `/* ${HEADER} */\n:root {\n${lines.join("\n")}\n}\n`;
}
```

### `--check` の仕組み

```js
for (const [path, content] of outputs(tokens)) {
  const current = existsSync(path)
    ? readFileSync(path, "utf8").replaceAll("\r\n", "\n")   // ← Windows の改行(CRLF)を揃えて比べる
    : null;
  if (current === content) continue;                          // 同じ → 何もしない
  if (check) { console.error(`stale: ${path}`); stale += 1; } // --check: 食い違いを報告するだけ
  else       { writeFileSync(path, content); }                // 通常: 書き出す
}
if (check && stale > 0) process.exit(1);                      // 終了コード 1 = 失敗(CI が落ちる)
```

- **冪等**(何回実行しても同じ結果): 同じなら書き換えない。だから、実行しても Git の差分が増えない。
- **改行の揃え**: Windows の Git は、取り出し時に改行を CRLF に変えることがあります。中身は同じなので、比べる前に LF に揃えます(これを入れる前は、Windows で `--check` が誤って失敗する危険がありました。実際に、CRLF に変えたファイルで通ることを確かめています)。
- **終了コード**: コマンドは終わるとき、数字を返します。**0=成功、それ以外=失敗**。CI は、これだけを見ます。

### 「直接実行されたときだけ動く」書き方

```js
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
```

`node tools/gen-tokens.mjs` で実行したときは `main()` が走り、**他のファイルから `import` されたときは走りません**(関数だけを使える)。テストが `renderCss` だけを呼べるのは、このおかげです。

### 試す

```bash
node tools/gen-tokens.mjs --check        # 食い違いが無ければ、何も出ず終了コード 0
# tokens.json の --color-accent を #1ed761 に変えてから:
node tools/gen-tokens.mjs --check        # "stale: ..." と出て、終了コード 1
node tools/gen-tokens.mjs                # 3つの生成物を書き直す(ここで初めて Theme.qml も変わる)
git diff --stat                          # 変わったのは、生成物と tokens.json だけ
```

## 11.3 図を生成する(`tools/diagrams`)

仕様(JavaScript のオブジェクト)から SVG を作る、小さな描画器です。作った理由と使い方は [講義資料 Chapter 12](../LECTURE_NOTES.md) にあります。ここでは、中身の**考え方**を3つ読みます。

### ① 矢印を、ノードの「縁」で止める — 二分探索

矢印は、ノードの**中心**を狙って引きますが、線を**縁**で止めたい。四角でも、ひし形でも、同じコードで。そこで、**「この点は図形の内側か」を返す関数**だけを図形ごとに持ち、中心から外へ向かって**二分探索**で縁を探します。

```js
function exitPoint(shape, ox, oy, dx, dy, gap) {
  let lo = 0, hi = 2000;                       // 内側にいる距離 〜 十分に外
  for (let i = 0; i < 40; i++) {
    const mid = (lo + hi) / 2;
    if (shape.inside(ox + ux * mid, oy + uy * mid)) lo = mid;   // まだ内側 → もっと外へ
    else hi = mid;                                              // 外 → 戻る
  }
  /* lo が、ほぼ縁の位置 */
}
```

図形の種類を増やすときは、`inside` を1つ書くだけ。**形ごとの縁の計算式を書かない**ことで、コードが短く・間違いにくくなります。

### ② 角を丸める

折れ線の角で、手前と先へ少し戻った点を作り、その間を**二次ベジェ曲線**(`Q`)でつなぎます。

### ③ 文字の幅を「推定」する

描画器は、実際のフォントを持たないので、文字の幅を**推定**します(日本語は1文字=1em、英数字は約0.56em)。正確ではないので、**画像にして目で確かめる**習慣が要ります。

### 検証(`validateSpec`)と、確認

描く前に、仕様の間違いを**描かずに止めます**(ノードの重なり、キャンバスからのはみ出し、存在しないノードへの矢印、未知のアイコン・色)。テストは `node --test tools/diagrams/render.test.mjs` の12件です。ただし、**検証は最低限**で、見た目の良し悪し(矢印が文字を横切る、ラベルが隠れる)は、**画像にして目で見る**ことでしか見つかりません。

## 11.4 5種類のテスト

| 対象 | コマンド | 守るもの | 所要 |
|---|---|---|---|
| `lorehub-api` | `cd lorehub-api && cargo test` | 認可・VCS・認証・スキーマなど(統合テスト)。**各テストが専用のメモリ DB** | 数秒〜十数秒 |
| `lorehub-web` | `cd lorehub-web && npm test` | `lib/` の純粋ロジック(整形・集計・API エラー・図形) | 1秒前後 |
| 図 | `node --test tools/diagrams/render.test.mjs` | 描画器の検証と決定性 | 1秒未満 |
| Qt Client | `ctest --test-dir build`(ビルドの後) | モデルのロジック(`RepositoryTreeModel`) | 1秒未満 |
| Qt Server Admin | 同上 | 権限グラフの変換(`PermissionConfigController`) | 1秒未満 |

加えて、**生成物の同期**(`gen-tokens --check`、`diagrams/build --check`)と、`cargo clippy`・`cargo fmt --check`・`npm run lint` が、CI で走ります。

**「テストがある」ことの限界**: テストが通っても、**画面の見た目**や、**実際に動くサーバー**までは保証しません。この資料の随所で「起動して確かめた」と書いているのは、そのためです。特に Rust は、`cargo test` が**通常のバイナリを作り直さない**ため、「テストは通ったが、起動する古いファイルは修正前」ということが実際に起きました(07章)。

## 11.5 CI: 壊れたものを入れない

[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) は、プッシュとプルリクエストで次の3つのジョブを走らせます。

```
lorehub-api        cargo test  /  cargo clippy -- -D warnings  /  cargo fmt --check
generated files    gen-tokens --check  /  diagrams/build --check  /  図のテスト
lorehub-web        npm ci  /  npm run build  /  npm run lint  /  npm test
```

- `clippy -D warnings`: Rust の「怪しい書き方」の警告を、**エラーとして扱う**。
- `cargo fmt --check`: 整形(インデント等)が規約どおりか。**自分でテストを足したとき、整形し忘れると CI が落ちます**(実際に自分で踏みました。`cargo fmt` で直る)。
- どれかが失敗すると、プルリクエストに赤が付く。**CI を緑にしてから**取り込む。

### CI と同じものを、手元で(コミット前の型)

```bash
# 生成物
node tools/gen-tokens.mjs --check && node tools/diagrams/build.mjs --check
node --test tools/diagrams/render.test.mjs

# API
cd lorehub-api && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

# Web
cd lorehub-web && npx tsc --noEmit && npm run lint && npm test && npm run build
```

> **`cargo fmt` と `cargo fmt --check` の違い**: `fmt` は**書き換える**、`--check` は**確認だけ**(CI はこちら)。手元では先に `cargo fmt` で直す。

## 11.6 Docker

`docker-compose.yml` は、2つのサービス(`lorehub-api`、`lorehub-web`)を立ち上げます。読むときの要点:

- **マルチステージビルド**: 1つ目の段で**ビルド**(コンパイラ等が要る)、最後の段に**成果物だけ**をコピー。最終イメージが小さく、攻撃面も小さい(`lorehub-api/Dockerfile`)。
- **ビルド時 vs 起動時**: `NEXT_PUBLIC_API_URL` は `build.args`(ビルド時)、`API_INTERNAL_URL` は `environment`(起動時)。[08章](08-nextjs-frontend.md) の表と同じ区別。
- **隔離**: `cap_drop: [ALL]` と `no-new-privileges`。非 root で動き、書き込み先は名前付きボリュームだけ([DEPLOYMENT.md](../DEPLOYMENT.md))。
- **ヘルスチェック**: `/api/health`(API)、`/login`(Web)。`depends_on: condition: service_healthy` で、API が健全になってから Web を起動する。

## 11.7 Git の作法

- **何をコミットしたか、必ず見る。** `git add -A` の後に `git status` で**意図しないファイル**(秘密、ビルド成果物、ログ)が無いか確認する。
- **秘密は入れない。** `.env` などは、そもそも読まない・コミットしない(`.gitignore`)。
- **コミットメッセージ**は、そのリポジトリの `git log` の流儀に合わせる(このリポジトリは、動詞で始まる英語の説明文)。「何を」だけでなく「**なぜ**」を書く。
- **生成物と元は、同じコミットに**入れる(食い違うコミットを作らない)。
- 改行の警告(`LF will be replaced by CRLF`)は、Windows の `autocrlf` の通知で、**エラーではない**。

## 11.8 Windows での作業の癖

- ターミナルは **Git Bash**(POSIX 風)と PowerShell で、書き方が違う。このリポジトリのコマンド例は、基本 Git Bash/POSIX 風。
- **バックスラッシュ(`\`)を含むコードを、シェルの「ヒアドキュメント」でファイルに書く**と、解釈されて壊れることがある(この資料を作る間に何度も起きた)。長いスクリプトや、`\n` を含む文字列は、**エディタ(ファイル作成ツール)で直接ファイルに書く**のが確実。
- Qt のビルドは、**MSVC の環境を設定してから**(`vcvarsall.bat x64`)行う([12章](12-qt-qml.md))。

## 11.9 まとめ

- 同じ値を複数に書かない。**元を1つ、生成物はコミット、CI で `--check`**。
- スクリプトは「データ・純粋関数・入出力」に分ける。`--check` と終了コードで、CI が使える形にする。
- 図の描画は、「形ごとの `inside`」+二分探索で、形に依存しないコードにできる。
- テストは5系統。**通っても、起動して確かめる**ことは別にある。
- コミット前に、CI と同じコマンドを手元で。`git status` で中身を見る。

## 11.10 演習

1. 11.2 の「試す」を実際にやる。`--check` の**終了コード**を、`echo $?`(PowerShell なら `$LASTEXITCODE`)で確かめる。
2. `gen-tokens.mjs` に、**新しいトークン**を足す手順を、`tokens.json` から順に実行して試す(例: `--color-info-subtle`)。足した後に `--check` が通ること、`Theme.qml` と CSS の両方に出ることを確かめる。(手順は [13章 レシピ E](13-extend-recipes.md))
3. `tools/diagrams/specs/` に新しい図を足す(2つのノードと1本の矢印だけでよい)。わざとノードを重ねて、`build.mjs` が**描かずにエラーを出す**ことを確かめる。直して、画像で見る。(手順は 13章 レシピ G)
4. CI の `generated-files` ジョブを手元で再現する。`tokens.json` を直して**再生成せずに**コミットしようとしたら、どのコマンドが、どう気づかせてくれるか。
5. **設計**: `docs/lore-ecosystem-docs.html` は、人が書いた部分と、図の生成部分が**同じファイル**にある。生成が、人の書いた部分を壊さないために、`build.mjs` はどんな工夫をしているか(`<!-- diagram:名前 -->` の目印)。もし目印が無いと、どうなるか。
6. **発展**: `gen-tokens.mjs` に、`--check` と同様に、**生成物が1つも無い**(初回)場合を、エラーにするか、作るか。どちらが親切か、理由つきで答える。

前: [10. アルゴリズム集](10-algorithms.md) / 次: [12. Qt / QML](12-qt-qml.md)
