// The repository screen as a wireframe: what sits where, and what shrinks.
export default {
  name: "repo-screen-layout",
  title: "リポジトリ画面のレイアウト",
  desc:
    "上から、パンくず、ページヘッダー、タブ(Code・Commits・Settings)。その下は3列で、左がファイルツリー(スクロールしても見える)、中央が選択したファイルかREADMEか空リポジトリの案内、右がサマリ(コミット・ブランチ・PRの数と、種類別のサイズ)。狭い画面では右のサマリが内容の下に回る。",
  width: 1500,
  height: 940,

  groups: [{ id: "page", label: "BROWSER  (幅 1280px 以上)", x: 40, y: 130, w: 1420, h: 720, tone: "gray" }],

  nodes: [
    { id: "crumb", kind: "box", x: 330, y: 200, w: 560, h: 40, label: "Repositories › audio-master", tone: "gray" },
    { id: "head", kind: "box", x: 750, y: 262, w: 1340, h: 56, label: "audio-master · nebula · 673 B · Updated 2h ago", tone: "gray" },
    { id: "tabs", kind: "box", x: 750, y: 328, w: 1340, h: 44, label: "[Code] [Commits] [Settings]", tone: "gray" },

    { id: "tree", kind: "box", x: 240, y: 620, w: 340, h: 360, label: "ファイルツリー\n(スクロールしても追従する)", tone: "blue" },
    { id: "main", kind: "box", x: 760, y: 620, w: 580, h: 360, label: "選択したファイル / README\n\n空のリポジトリなら\n「最初のコミットを待っています」\n+ 手順 + 木のアスキーアート", tone: "green" },
    { id: "sum", kind: "box", x: 1260, y: 620, w: 300, h: 360, label: "サマリ\nコミット · ブランチ · PR の数\n\nサイズ(種類別の積み上げ)\n音声 · 3D · 画像 · その他", tone: "orange" },
  ],

  edges: [],

  labels: [
    { x: 240, y: 840, text: "lg 以上で左に固定", size: 12, mono: false, tone: "blue" },
    { x: 760, y: 840, text: "常に主役", size: 12, mono: false, tone: "green" },
    { x: 1260, y: 840, text: "xl 以上で右 / それ未満は下", size: 12, mono: false, tone: "orange" },
    { x: 40, y: 64, text: "リポジトリ画面のレイアウト", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "「何が変わったか」を最短で見せる3列。ファイル行・容量の表示ルールは docs/learning/09 を参照", anchor: "start", size: 13, mono: false },
  ],
};
