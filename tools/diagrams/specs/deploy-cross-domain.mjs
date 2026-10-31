// Why login worked locally and failed once deployed: where the cookie lives.
// See docs/DEPLOYMENT.md and docs/LECTURE_NOTES.md chapter 9.
export default {
  name: "deploy-cross-domain",
  title: "同じホスト名の配置と、別ドメインの配置の違い",
  desc:
    "上段は docker compose など同じホスト名での配置で、ブラウザは localhost の3000番と4000番に話し、Cookie は同一サイトとして届く。下段は Vercel と Cloudflare Tunnel を使う別ドメインの配置で、ブラウザは Vercel にしか話さず、Next.js の rewrites が /api を Tunnel 経由でローカルPCの lorehub-api へ転送する。Set-Cookie は Vercel のドメインに保存される。デスクトップクライアントは同じPCの localhost:4000 に直接つなぐ。",
  width: 1800,
  height: 1060,

  groups: [
    { id: "same", label: "SAME HOST  (docker compose · localhost)", x: 40, y: 130, w: 1720, h: 300, tone: "green" },
    { id: "diff", label: "DIFFERENT DOMAINS  (Vercel + Cloudflare Tunnel)", x: 40, y: 480, w: 1720, h: 540, tone: "blue" },
  ],

  nodes: [
    { id: "brA", x: 200, y: 250, icon: "browser", label: "ブラウザ", caption: "localhost", tone: "blue" },
    { id: "webA", x: 700, y: 250, icon: "server", label: "lorehub-web", caption: ":3000", tone: "green" },
    { id: "apiA", x: 1200, y: 250, icon: "braces", label: "lorehub-api", caption: ":4000", tone: "green" },
    { id: "noteA", kind: "box", x: 1560, y: 250, w: 340, label: "localhost の :3000 と :4000 は\n同じサイト → Cookie が届く", tone: "green" },

    { id: "brB", x: 200, y: 700, icon: "browser", label: "ブラウザ", caption: "*.vercel.app", tone: "blue" },
    { id: "vercel", x: 640, y: 700, icon: "cloud", label: "Vercel", caption: "Next.js · rewrites /api/*", tone: "blue" },
    { id: "tunnel", x: 1080, y: 700, icon: "cloud", label: "Cloudflare Tunnel", caption: "*.trycloudflare.com", tone: "orange" },
    { id: "apiB", x: 1520, y: 700, icon: "server", label: "lorehub-api", caption: "自分のPC · localhost:4000", tone: "green" },
    { id: "desk", x: 1080, y: 900, icon: "monitor", label: "LoreForge Client / Admin", caption: "同じPC · http://localhost:4000", tone: "blue" },

    { id: "note1", kind: "box", x: 640, y: 560, w: 420, label: "ブラウザが話すのは Vercel だけ\n→ Cookie は 1 つのサイトの中で完結", tone: "green" },
    { id: "note2", kind: "box", x: 1300, y: 560, w: 460, label: "Quick Tunnel の URL は再起動で変わる\n→ API_INTERNAL_URL を更新して再デプロイ", tone: "orange" },
  ],

  edges: [
    { from: "brA", to: "webA", label: "HTTP", step: 1, labelDy: -16 },
    { from: "webA", to: "apiA", label: "API_INTERNAL_URL(Docker 内)", labelDy: -16 },
    {
      from: "brA",
      to: "apiA",
      via: [[200, 178], [1200, 178]],
      tone: "gray",
      dashed: true,
      label: "NEXT_PUBLIC_API_URL(ブラウザから直接)",
      labelSeg: 1,
      labelDy: -14,
    },

    { from: "brB", to: "vercel", step: 1, label: "POST /api/auth/login", fromOffset: [0, -16], toOffset: [0, -16], labelDy: -16 },
    { from: "vercel", to: "tunnel", step: 2, label: "rewrite", fromOffset: [0, -16], toOffset: [0, -16], labelDy: -16 },
    { from: "tunnel", to: "apiB", step: 3, label: "→ localhost:4000", fromOffset: [0, -16], toOffset: [0, -16], labelDy: -16 },
    {
      from: "apiB",
      to: "brB",
      via: [[1690, 686], [1690, 820], [90, 820], [90, 700]],
      fromOffset: [0, -14],
      tone: "green",
      dashed: true,
      step: 4,
      label: "応答 + Set-Cookie(Vercel のドメインに保存)",
      labelSeg: 2,
      labelDy: -14,
    },
    {
      from: "desk",
      to: "apiB",
      via: [[1620, 900], [1620, 714]],
      toOffset: [0, 14],
      tone: "gray",
      dashed: true,
      label: "直結 · LOREHUB_INSECURE_COOKIES=true",
      labelSeg: 0,
      labelDy: -14,
    },
  ],

  labels: [
    { x: 40, y: 64, text: "Cookie はどのドメインに保存されるか", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "ローカルでは同じホスト名なので問題が出ない。別ドメインに分けた途端、ログインが通らなくなる(原因と対処は DEPLOYMENT.md)", anchor: "start", size: 13, mono: false },
  ],
};
