// What happens, in order, when a signed-in person opens a repository page.
export default {
  name: "request-flow",
  title: "リポジトリ画面が表示されるまでの流れ",
  desc:
    "ブラウザが Next.js サーバーにリポジトリ画面を要求すると、Next.js はログイン確認と6本のAPI取得を lorehub-api に並列で依頼し、APIは SQLite を読んで JSON を返す。Next.js はサーバーコンポーネントで HTML を組み立ててブラウザに返し、ブラウザが表示する。手順は1から9の番号で示す。",
  width: 1740,
  height: 650,

  groups: [{ id: "server", label: "SERVER SIDE", x: 450, y: 150, w: 1250, h: 300, tone: "green" }],

  nodes: [
    { id: "browser", x: 190, y: 310, icon: "browser", label: "ブラウザ", caption: "利用者", tone: "blue" },
    { id: "next", x: 640, y: 310, icon: "server", label: "Next.js サーバー", caption: "proxy.ts · Server Component", tone: "green" },
    { id: "api", x: 1090, y: 310, icon: "braces", label: "lorehub-api", caption: "Axum · require_auth", tone: "green" },
    { id: "db", x: 1520, y: 310, icon: "database", label: "SQLite", caption: "tree · commits · branches", tone: "orange" },

    { id: "n7", kind: "box", x: 640, y: 530, w: 300, label: "Server Component が\nHTML を組み立てる", badge: 7, tone: "green" },
    { id: "n23", kind: "box", x: 1090, y: 530, w: 330, label: "Cookie のトークンを sessions で引き、\n通ったユーザーでハンドラを実行", badge: "2-3", tone: "green" },
    { id: "n9", kind: "box", x: 190, y: 530, w: 290, label: "表示。コピーやトーストは\nClient Component が動く", badge: 9, tone: "blue" },
  ],

  edges: [
    { from: "browser", to: "next", step: 1, label: "GET /repositories/…", fromOffset: [0, -18], toOffset: [0, -18], labelDy: -16 },
    { from: "next", to: "browser", step: 8, label: "HTML", dashed: true, tone: "green", fromOffset: [0, 18], toOffset: [0, 18], labelDy: 18 },

    { from: "next", to: "api", step: 2, label: "GET /api/auth/me", fromOffset: [0, -34], toOffset: [0, -34], labelDy: -16 },
    { from: "next", to: "api", step: 3, label: "tree · commits · branches · PR×3", fromOffset: [0, -6], toOffset: [0, -6], labelDy: -16 },
    { from: "api", to: "next", step: 6, label: "JSON", dashed: true, tone: "green", fromOffset: [0, 22], toOffset: [0, 22], labelDy: 18 },

    { from: "api", to: "db", step: 4, label: "SELECT (sqlx)", fromOffset: [0, -14], toOffset: [0, -14], labelDy: -16 },
    { from: "db", to: "api", step: 5, label: "行", dashed: true, tone: "orange", fromOffset: [0, 14], toOffset: [0, 14], labelDy: 18 },
  ],

  labels: [
    { x: 40, y: 64, text: "リポジトリ画面が表示されるまで", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "番号は処理の順(簡略化: レイアウトのリポジトリ取得は省略)。実線は要求、点線は応答。ページの6本の取得は同時に走る(Promise.all)", anchor: "start", size: 13, mono: false },
  ],
};
