// The course in docs/learning/: which chapter to read before which.
export default {
  name: "learning-map",
  title: "学習資料の全体地図",
  desc:
    "学習資料は00から13の章でできている。00から始め、バックエンドの道(02 Rust、03 Axum、04 SQL、05 内容アドレス、06 diff)と、Webの道(01 Webの基礎、08 Next.js、09 UIの決めごと、12 Qt)に分かれる。07 認証と認可は03の後、10 アルゴリズム集は05の後、11 スクリプト・テスト・CIはその後、最後に13 拡張レシピ集で全体を使う。",
  width: 1900,
  height: 820,

  groups: [
    { id: "be", label: "BACKEND  (Rust)", x: 330, y: 130, w: 1500, h: 190, tone: "green" },
    { id: "fe", label: "FRONTEND  (Web / Desktop)", x: 330, y: 440, w: 1200, h: 190, tone: "orange" },
  ],

  nodes: [
    { id: "c00", kind: "box", x: 150, y: 380, w: 190, label: "地図と読み方", caption: "環境構築 · 学習ルート", badge: "00", tone: "blue" },

    { id: "c02", kind: "box", x: 450, y: 225, w: 200, label: "Rust の基礎", caption: "所有権 · Result · async", badge: "02", tone: "green" },
    { id: "c03", kind: "box", x: 750, y: 225, w: 200, label: "Axum の API", caption: "ルーター · 抽出子 · 層", badge: "03", tone: "green" },
    { id: "c04", kind: "box", x: 1050, y: 225, w: 200, label: "SQL と永続化", caption: "sqlx · 正規化 · Tx", badge: "04", tone: "green" },
    { id: "c05", kind: "box", x: 1350, y: 225, w: 230, label: "内容アドレス方式", caption: "SHA-256 · 重複排除", badge: "05", tone: "green" },
    { id: "c06", kind: "box", x: 1670, y: 225, w: 200, label: "差分アルゴリズム", caption: "LCS · Myers", badge: "06", tone: "green" },

    { id: "c01", kind: "box", x: 450, y: 535, w: 200, label: "Web の基礎", caption: "HTTP · Cookie · セッション", badge: "01", tone: "orange" },
    { id: "c08", kind: "box", x: 750, y: 535, w: 200, label: "Next.js", caption: "RSC · proxy · 環境変数", badge: "08", tone: "orange" },
    { id: "c09", kind: "box", x: 1050, y: 535, w: 200, label: "UI の決めごと", caption: "a11y · トークン · 部品", badge: "09", tone: "orange" },
    { id: "c12", kind: "box", x: 1350, y: 535, w: 200, label: "Qt / QML", caption: "Theme · Model/View", badge: "12", tone: "orange" },

    { id: "c07", kind: "box", x: 1050, y: 380, w: 200, label: "認証と認可", caption: "401/403 · ACL · 制限", badge: "07", tone: "blue" },
    { id: "c10", kind: "box", x: 1350, y: 380, w: 200, label: "アルゴリズム集", caption: "木 · グラフ · 再帰", badge: "10", tone: "blue" },
    { id: "c11", kind: "box", x: 1650, y: 535, w: 200, label: "スクリプト・テスト・CI", caption: "gen-tokens · vitest · cargo", badge: "11", tone: "gray" },
    { id: "c13", kind: "box", x: 1650, y: 700, w: 200, label: "拡張レシピ集", caption: "足し方 · 直し方", badge: "13", tone: "gray" },
  ],

  edges: [
    { from: "c00", to: "c02", via: [[150, 225]], tone: "gray" },
    { from: "c00", to: "c01", via: [[150, 535]], tone: "gray" },

    { from: "c02", to: "c03", tone: "green" },
    { from: "c03", to: "c04", tone: "green" },
    { from: "c04", to: "c05", tone: "green" },
    { from: "c05", to: "c06", tone: "green" },

    { from: "c01", to: "c08", tone: "orange" },
    { from: "c08", to: "c09", tone: "orange" },
    { from: "c09", to: "c12", tone: "orange" },

    { from: "c03", to: "c07", via: [[750, 380]], tone: "blue", dashed: true },
    { from: "c05", to: "c10", tone: "blue", dashed: true },
    { from: "c12", to: "c11", via: [[1500, 535]], tone: "gray", dashed: true, fromOffset: [0, 0] },
    { from: "c10", to: "c11", via: [[1650, 380]], tone: "gray", dashed: true },
    { from: "c11", to: "c13", tone: "gray" },
  ],

  labels: [
    { x: 40, y: 64, text: "学習資料の地図(docs/learning/)", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "00 から始める。実線は前の章を読んでから、点線は読むと理解が深まる関係。章末に演習がある", anchor: "start", size: 13, mono: false },
  ],
};
