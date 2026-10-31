// One source of design values, generated into every platform's own format.
export default {
  name: "token-pipeline",
  title: "デザイントークンの生成パイプライン",
  desc:
    "開発者は design-tokens/tokens.json だけを編集する。tools/gen-tokens.mjs がそこから、Web用の tokens.generated.css と、LoreForge Client・Server Admin それぞれの Theme.qml を生成する。CI は同じスクリプトを --check で実行し、生成物がコミット済みのものと食い違えば失敗させる。",
  width: 1700,
  height: 900,

  groups: [{ id: "out", label: "GENERATED  (編集しない)", x: 1000, y: 140, w: 660, h: 520, tone: "orange" }],

  nodes: [
    { id: "dev", x: 140, y: 400, icon: "user", label: "開発者", caption: "値を変えたいとき", tone: "blue" },
    { id: "tokens", x: 460, y: 400, icon: "swatches", label: "tokens.json", caption: "design-tokens/", tone: "green" },
    { id: "gen", x: 780, y: 400, icon: "terminal", label: "gen-tokens.mjs", caption: "node tools/gen-tokens.mjs", tone: "green" },

    { id: "css", x: 1180, y: 260, icon: "file", label: "tokens.generated.css", caption: "lorehub-web", tone: "orange" },
    { id: "qml1", x: 1180, y: 400, icon: "file", label: "Theme.qml", caption: "loreforge-client", tone: "orange" },
    { id: "qml2", x: 1180, y: 540, icon: "file", label: "Theme.qml", caption: "loreforge-server-admin", tone: "orange" },

    { id: "web", x: 1520, y: 260, icon: "browser", label: "LoreHub Web", caption: "Tailwind の @theme が参照", tone: "blue" },
    { id: "client", x: 1520, y: 400, icon: "monitor", label: "LoreForge Client", caption: "QML シングルトン", tone: "blue" },
    { id: "admin", x: 1520, y: 540, icon: "gear", label: "Server Admin", caption: "QML シングルトン", tone: "blue" },

    { id: "ci", x: 780, y: 760, icon: "workflow", label: "GitHub Actions", caption: "gen-tokens.mjs --check", tone: "gray" },
    { id: "fail", kind: "box", x: 1180, y: 760, w: 300, label: "コミット済みと食い違えば\nCI が失敗する", tone: "red" },
  ],

  edges: [
    { from: "dev", to: "tokens", label: "編集", step: 1 },
    { from: "tokens", to: "gen", label: "読み込む", step: 2 },
    { from: "gen", to: "css", via: [[780, 260]], step: 3, label: "書き出し", labelSeg: 1, labelDy: -14 },
    { from: "gen", to: "qml1", labelSeg: 0 },
    { from: "gen", to: "qml2", via: [[780, 540]], labelSeg: 1 },

    { from: "css", to: "web", tone: "gray", dashed: true },
    { from: "qml1", to: "client", tone: "gray", dashed: true },
    { from: "qml2", to: "admin", tone: "gray", dashed: true },

    { from: "gen", to: "ci", tone: "gray", dashed: true, label: "CI でも同じ生成を実行", labelDx: 80 },
    { from: "ci", to: "fail", tone: "red", dashed: true, label: "差分あり" },
  ],

  labels: [
    { x: 40, y: 64, text: "デザイントークンは1か所で決め、各環境へ生成する", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "値を直すのは tokens.json だけ。Web と Qt の見た目が食い違わない", anchor: "start", size: 13, mono: false },
  ],
};
