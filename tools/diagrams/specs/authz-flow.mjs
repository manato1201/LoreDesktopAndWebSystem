// How a request is judged: 401 (who are you?) vs 403 (you may not).
// Source: handlers.rs (require_auth, is_owner_or_admin, deny_*), authz.rs.
export default {
  name: "authz-flow",
  title: "リクエストの認証・認可の判定フロー",
  desc:
    "リクエストはまず有効なアクセストークンの確認を受け、なければ401。ユーザーが特定できたら、組織の管理操作ならOwnerまたはAdminかを確認し、満たさなければ403と監査ログ。パスを扱う操作ならパスACLで許可されているかを確認し、拒否なら403と監査ログ。通ればハンドラが実行される。閲覧系の一部(ツリー、コミット一覧)はパスで絞り込まない。",
  width: 1560,
  height: 1260,

  nodes: [
    { id: "req", kind: "pill", x: 520, y: 175, label: "リクエスト(Cookie つき)", tone: "gray" },
    { id: "d1", kind: "diamond", x: 520, y: 305, label: "有効な\nアクセストークン?", tone: "blue" },
    { id: "e401", kind: "box", x: 1040, y: 305, w: 250, label: "401 Unauthorized\n(誰か分からない)", tone: "red" },
    { id: "user", kind: "box", x: 520, y: 440, w: 300, label: "ユーザー(OrgMember)を特定", tone: "green" },

    { id: "d2", kind: "diamond", x: 520, y: 580, label: "組織の管理操作?\n(削除 · ロール · 招待 · ACL · toggle)", tone: "blue", w: 340 },
    { id: "d3", kind: "diamond", x: 1040, y: 580, label: "Owner /\nAdmin?", tone: "blue" },
    { id: "e403r", kind: "box", x: 1040, y: 740, w: 290, label: "403 + 監査ログ\nrequires Owner or Admin", tone: "red" },

    { id: "d4", kind: "diamond", x: 520, y: 760, label: "パスを扱う操作?\n(取得 · upload · lock · stage · diff)", tone: "blue", w: 340 },
    { id: "d5", kind: "diamond", x: 520, y: 925, label: "パスACLで\n許可されている?", tone: "blue", w: 250 },
    { id: "e403p", kind: "box", x: 1040, y: 925, w: 290, label: "403 + 監査ログ\ndenied {read|write|lock}", tone: "red" },

    { id: "ok", kind: "pill", x: 520, y: 1085, label: "ハンドラを実行", tone: "green" },
    { id: "warn", kind: "box", x: 1080, y: 1180, w: 560, label: "注意: ツリー・コミット一覧・コミット詳細は\nパスで絞り込まない(ファイル名は見える)", tone: "orange" },
  ],

  edges: [
    { from: "req", to: "d1" },
    { from: "d1", to: "e401", label: "いいえ", tone: "red", dashed: true, labelDy: -14 },
    { from: "d1", to: "user", label: "はい", labelDx: 22 },
    { from: "user", to: "d2" },

    { from: "d2", to: "d3", label: "はい", labelDy: -14 },
    { from: "d3", to: "e403r", label: "いいえ", tone: "red", dashed: true, labelDx: 28 },
    { from: "d3", to: "ok", label: "はい", via: [[1400, 580], [1400, 1085]], tone: "green", labelSeg: 0, labelDy: -14 },

    { from: "d2", to: "d4", label: "いいえ", labelDx: 28 },
    { from: "d4", to: "d5", label: "はい", labelDx: 22 },
    { from: "d4", to: "ok", label: "いいえ(認証だけで通る)", via: [[250, 760], [250, 1085]], labelSeg: 0, labelDy: -14, tone: "gray", dashed: true },
    { from: "d5", to: "e403p", label: "いいえ", tone: "red", dashed: true, labelDy: -14 },
    { from: "d5", to: "ok", label: "はい", labelDx: 22 },
  ],

  labels: [
    { x: 40, y: 64, text: "リクエストの判定: 401 と 403 の使い分け", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "401 = 認証されていない(誰か分からない) / 403 = 認証はされたが許可がない。どちらの拒否も、拒否そのものを監査ログに残す(401 を除く)", anchor: "start", size: 13, mono: false },
  ],
};
