# 08. Next.js のフロントエンド

**この章で分かること**: LoreHub Web が、どうやって画面を作るか。サーバーコンポーネントとクライアントコンポーネントの違い、Cookie の扱い、期限切れトークンの透過的な更新、そして**環境変数が「いつ」決まるか**(ビルド時か、起動時か)。

前提: [01](01-web-basics.md)(Cookie)、[07](07-auth-and-authz.md)(セッションの更新)。React の基本(コンポーネント、`props`、`useState`)が分かると読みやすいです。

> **重要な注意(このリポジトリの約束)**: `lorehub-web/AGENTS.md` に「**これはあなたの知っている Next.js ではない**。破壊的変更がある。コードを書く前に `node_modules/next/dist/docs/` の該当ガイドを読め」とあります。この章を読んでコードを書くときも、**必ず手元の版のドキュメントで確かめて**ください(例: `middleware` が `proxy` になっている、`params` が Promise になっている、など)。

![リポジトリ画面が表示されるまで](../diagrams/request-flow.svg)

## 8.1 App Router: フォルダが URL になる

`src/app/` の下の**フォルダ構造**がそのまま URL になります。

```
src/app/
├── layout.tsx                 全ページ共通の枠(<html> と フォント)
├── not-found.tsx              404 の画面
├── login/page.tsx             /login
└── (app)/                     ← ( ) のフォルダは URL に出ない「グループ」。ログイン必須の領域
    ├── layout.tsx             ログイン確認 + サイドバー + トースト(ここが関門)
    ├── page.tsx               /
    ├── pulls/page.tsx         /pulls
    └── repositories/[slug]/   ← [slug] は変わる部分(例: /repositories/audio-master)
        ├── layout.tsx         リポジトリのヘッダーとタブ
        ├── page.tsx           /repositories/audio-master      (Code)
        └── commits/
            ├── page.tsx       …/commits
            └── [hash]/page.tsx  …/commits/{ハッシュ}
```

- `page.tsx` = その URL の中身。`layout.tsx` = 子をくるむ枠(ネストする)。
- `(app)` グループの `layout.tsx` が、ログインしていなければ `/login` へ送ります。**この1か所が、ログイン必須の領域全体の関門**です。

```tsx
// (app)/layout.tsx(抜粋)
const cookie = await getSessionCookieHeader();
const user = cookie ? await getCurrentUser(cookie) : null;
if (!user) redirect("/login");
```

## 8.2 サーバーコンポーネントとクライアントコンポーネント

| | サーバーコンポーネント(既定) | クライアントコンポーネント(`"use client"`) |
|---|---|---|
| 動く場所 | **サーバー**(HTML を作る) | **ブラウザ**(対話する) |
| できること | DB・API を直接呼ぶ、秘密を扱う、`async/await` | `useState`、クリック、`localStorage` |
| できないこと | `useState`・クリック | サーバーだけの秘密に触る |
| 例 | `repositories/[slug]/page.tsx` | `RepositoryExplorer.tsx`、`Toast.tsx`、`CopyableHash.tsx` |

**使い分けの原則**: まずサーバーコンポーネントにして、**対話が要る所だけ**クライアントにします。リポジトリ画面は、サーバー側が6本の取得を済ませて HTML を作り、**クリックして選ぶ部分(`RepositoryExplorer`)だけ**がクライアントです。

```tsx
// repositories/[slug]/page.tsx(抜粋) — サーバーコンポーネント
const [tree, commits, branches, openPrs, mergedPrs, closedPrs] = await Promise.all([
  getTree(slug, cookie), getCommits(slug, cookie), getBranches(slug, cookie),
  getPullRequests("open", cookie), getPullRequests("merged", cookie), getPullRequests("closed", cookie),
]);
if (!tree) notFound();                                // 404 の画面へ
return <RepositoryExplorer repoSlug={slug} tree={tree} summary={<RepositorySummary ... />} />;
```

- `Promise.all` は、6本の取得を**同時に**走らせ、全部が済むのを待ちます(順に待つより速い)。
- `notFound()` は、`app/not-found.tsx`(アスキーの木)を出します。
- **サーバーコンポーネントを、クライアントコンポーネントの `props` として渡せます**(`summary={...}`)。右サマリはサーバーで作り、枠の中に差し込んでいます。

## 8.3 Cookie をサーバーから API へ渡す

サーバー(Node.js)には、ブラウザのような「Cookie の保管庫」がありません。だから、サーバーコンポーネントから API を呼ぶときは、**Cookie を自分で付けます**([`auth-server.ts`](../../lorehub-web/src/lib/auth-server.ts))。

```ts
import "server-only";                                  // ← クライアントから import するとビルドエラーにする
export async function getSessionCookieHeader() {
  const token = (await cookies()).get("lorehub_token")?.value;
  return token ? `lorehub_token=${token}` : undefined;
}
```

`import "server-only"` は、**うっかりクライアントのコードに混ぜたとき、黙って壊れるのでなく、ビルドが失敗する**ようにする保険です。

## 8.4 期限切れトークンを、気づかせずに更新する

アクセストークンは30分で切れます([07](07-auth-and-authz.md))。切れるたびに「ログインし直し」では不便なので、リフレッシュトークンで**こっそり更新**します。ただし、**経路によってやり方が違います**。

| 経路 | 誰が更新するか | なぜ |
|---|---|---|
| ブラウザ(クライアントコンポーネントの `fetch`) | [`fetchWithRefresh`](../../lorehub-web/src/lib/api.ts) | 401 を受けたら、**1回だけ**更新して、同じ要求をやり直す |
| サーバー(ページの描画) | [`proxy.ts`](../../lorehub-web/src/proxy.ts) | サーバーコンポーネントは**描画中に Cookie を設定できない**ので、描画の**前**に走るプロキシが更新する |

**`fetchWithRefresh` の工夫**: ページが同時に何本も API を呼び、全部が 401 になることがあります。そのとき、更新を何回も走らせると、リフレッシュトークンが使い捨てなので**2回目以降が失敗**します。そこで、**進行中の更新を1つだけ共有**します。

```ts
let refreshInFlight: Promise<boolean> | null = null;

function refreshAccessToken() {
  if (!refreshInFlight) {
    refreshInFlight = fetch(`${API_BASE}/api/auth/refresh`, { method: "POST", credentials: "include" })
      .then((res) => res.ok).catch(() => false)
      .finally(() => { refreshInFlight = null; });
  }
  return refreshInFlight;        // 2本目以降は、同じ Promise を待つだけ
}
```

**`proxy.ts` の条件**: アクセストークンの Cookie が**まるごと無く**、リフレッシュトークンがある場合だけ動きます(アクセストークンは乱数で、期限が中に入っていないため、「Cookie が消えた」が確かな合図)。更新できたら、①この描画に新しい Cookie を渡し、②ブラウザへ `Set-Cookie` を返します。対象は、ログイン必須の URL(`matcher`)だけです。

## 8.5 環境変数は「いつ」決まるか — いちばんの落とし穴

| 変数 | 決まる時 | 見える場所 |
|---|---|---|
| `NEXT_PUBLIC_*`(例: `NEXT_PUBLIC_API_URL`、`NEXT_PUBLIC_USE_API_PROXY`) | **ビルド時**に、ブラウザ向け JS へ**文字列として埋め込まれる** | ブラウザとサーバー |
| `API_INTERNAL_URL` を**ランタイムのコード**で読む(`api.ts`、`proxy.ts`) | **起動時**に読まれる | サーバーだけ |
| `next.config.ts` の **`rewrites()`** が読む `API_INTERNAL_URL` | **ビルド時** | — |

同じ名前の変数が、**使う場所によって「いつ決まるか」が違う**のが罠です。

- 変数を後から変えても、ブラウザ向け JS に**すでに埋め込まれた値は変わりません**(再ビルドが要る)。
- `rewrites()` はビルド時に評価されるので、`API_INTERNAL_URL` は **`npm run build` の時点で**設定されていなければ、転送のルールが作られません(起動時に設定しても遅い)。
- 実デプロイで実際に起きたこと: 環境変数を設定したつもりが、ビルドに反映されていなかった。**ダッシュボードの表示を信じず、配信された JS の中身を直接読んで**確かめた([講義資料 Chapter 9](../LECTURE_NOTES.md))。

### API の呼び先を決める `API_BASE`

```ts
const API_BASE =
  typeof window === "undefined"                       // サーバーで動いているか?
    ? (process.env.API_INTERNAL_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000")
    : USE_API_PROXY                                   // ブラウザ: プロキシを使う設定か?
      ? ""                                            //   → 相対パス(/api/...)。同じオリジンへ
      : (process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000");
```

ブラウザが `""`(相対パス)で呼ぶと、要求は**自分のドメイン**に来て、`next.config.ts` の `rewrites()` が API へ転送します。ブラウザは API のドメインを**一切見ない**ので、Cookie は Web アプリのドメインのものになります([01](01-web-basics.md) の図)。

> **「空文字」で切り替える設計をやめた理由**: 以前は `NEXT_PUBLIC_API_URL` を**空**にして「プロキシを使う」の合図にしていましたが、ホスティングのダッシュボードによっては、空文字を保存できません(古い値が残る)。**専用の明示的なフラグ** `NEXT_PUBLIC_USE_API_PROXY=true` に置き換えました。「何も無い」で意味を持たせるより、「ある」ことで意味を持たせるほうが、設定ミスに強い。

## 8.6 ビルドと配布(Docker)

`next.config.ts` の `output: "standalone"` は、**実行に必要なファイルだけ**を `.next/standalone` に集めます。Docker の最終イメージが軽くなります(`lorehub-web/Dockerfile`)。

```
npm run build の出力:
  ○ (Static)   /login ...           事前に作れる(ビルド時に HTML 化)
  ƒ (Dynamic)  /repositories/[slug] ... 要求のたびにサーバーで作る(Cookie に依存するため)
```

ログインに依存するページは、すべて `ƒ`(動的)になります。

## 8.7 テストの限界を知る

`lorehub-web` のテストは Vitest で、環境は **node**(ブラウザではない)です。

- 純粋なロジック(`format.ts`、`graph-layout.ts`、`api-errors.ts`)は、普通に単体テストできる。
- **`async` なサーバーコンポーネントは、Vitest で描画できない**(`vitest.config.mts` のコメント)。だから、**ロジックを `lib/` の関数に出して、そこをテストする**設計にしている。
- 画面そのものは、**起動して見る**(または E2E テストを足す)。

これが、リポジトリ画面の容量集計を `summarizeStorage`(`lib/format.ts`)に出している理由です。

## 8.8 まとめ

- フォルダ構造が URL。`layout.tsx` が枠、`(app)` グループが「ログイン必須」の関門。
- サーバーコンポーネントを既定に、対話が要る所だけ `"use client"`。
- サーバーから API を呼ぶときは Cookie を**自分で付ける**。
- 期限切れの更新は、ブラウザは `fetchWithRefresh`(進行中の更新を共有)、サーバーは `proxy.ts`(描画の前)。
- **環境変数は「いつ決まるか」を確かめる**。`NEXT_PUBLIC_*` と `rewrites()` はビルド時。
- テストしにくい所は、**ロジックを関数に出す**。

## 8.9 演習

1. `npm run build` を実行し、出力のルート表で、どのページが `○` で、どのページが `ƒ` か確認する。ログイン必須のページがすべて `ƒ` になる理由を、`cookies()` との関係で説明する。
2. `lib/api.ts` の `fetchWithRefresh` のテスト(`api.test.ts`)を読み、「同時の401で、更新が1回だけ走る」ことを確かめているテストを探す。なぜ `refreshInFlight` を `finally` で `null` に戻すのか?(戻さないと、次の期限切れで何が起きる?)
3. `/repositories/audio-master` の URL を、存在しない `/repositories/nope` に変えて開き、`notFound()` → `not-found.tsx` が出ることを確かめる。`curl -i -b jar.txt` でステータスが 404 であることも見る(手元では、存在するリポジトリが 200、存在しないものが 404、**Cookie なしの `/` は 307 で `/login` へ**でした)。
4. **環境変数の実験**: `NEXT_PUBLIC_API_URL=http://localhost:9999 npm run build` でビルドし、`.next/static` の JS を検索して、`9999` が埋め込まれていることを確かめる。次に、起動時だけ別の値にしても、ブラウザ向け JS が変わらないことを確かめる。
5. **設計**: ページ内の `getPullRequests` が3回(open/merged/closed)で、リポジトリごとの PR 件数を数えている。API に `?repo=` の絞り込みを足すと、どう良くなるか。どこを変える必要があるか?(API・`api.ts`・`page.tsx`)

前: [07. 認証と認可](07-auth-and-authz.md) / 次: [09. UI の決めごと](09-ui-rules.md)
