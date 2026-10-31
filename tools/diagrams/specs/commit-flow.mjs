// The three calls that turn a local file into a commit, as flowcharts.
// Every box maps to code: handlers.rs (upload_file, stage_change) and
// vcs_store.rs (create_commit, compute_commit_hash).
export default {
  name: "commit-flow",
  title: "ファイルがコミットになるまでの流れ",
  desc:
    "3つのAPI呼び出しの順に進む。1. アップロード: 書き込み権限を確認し、内容の SHA-256 を計算して blobs/ に保存し、file_blobs と staged_content を更新する。2. ステージ: その時点の content_hash を pending_changes に確定して記録する。3. コミット: 1つのトランザクションの中で、ステージ済みの有無と内容の保存済みを検証し、親のツリーに変更を適用し、コミットハッシュを計算して各テーブルに書き込み、ブランチの先頭を進める。",
  width: 1900,
  height: 1100,

  groups: [
    { id: "g1", label: "1  POST /upload", x: 40, y: 140, w: 580, h: 790, tone: "blue" },
    { id: "g2", label: "2  POST /tree/stage", x: 680, y: 140, w: 480, h: 790, tone: "blue" },
    { id: "g3", label: "3  POST /commits  (1 TRANSACTION)", x: 1220, y: 140, w: 640, h: 910, tone: "green" },
  ],

  nodes: [
    // ---- 1. upload
    { id: "u0", kind: "pill", x: 260, y: 215, label: "ファイルを送る", tone: "gray" },
    { id: "u1", kind: "diamond", x: 260, y: 335, label: "Write 権限が\nある?", tone: "blue" },
    { id: "u1x", kind: "box", x: 500, y: 335, w: 170, label: "403 + 監査ログ", tone: "red" },
    { id: "u2", kind: "box", x: 260, y: 470, w: 280, label: "内容の SHA-256 を計算", tone: "green" },
    { id: "u3", kind: "box", x: 260, y: 600, w: 330, label: "blobs/ に保存\n同じ内容は書き直さない\n(一時ファイル → rename)", tone: "green" },
    { id: "u4", kind: "box", x: 260, y: 740, w: 360, label: "file_blobs と staged_content\nを更新", tone: "green" },
    { id: "u5", kind: "pill", x: 260, y: 860, label: "201 Created", tone: "gray" },

    // ---- 2. stage
    { id: "s0", kind: "pill", x: 920, y: 215, label: "変更を宣言\n(added / modified / deleted)", tone: "gray", w: 280 },
    { id: "s1", kind: "diamond", x: 920, y: 335, label: "Write 権限が\nある?", tone: "blue" },
    { id: "s1x", kind: "box", x: 1112, y: 335, w: 80, label: "403", tone: "red" },
    { id: "s2", kind: "box", x: 920, y: 480, w: 330, label: "staged_content から\ncontent_hash と size を取り出す", tone: "green" },
    { id: "s3", kind: "box", x: 920, y: 620, w: 330, label: "pending_changes に記録\nhash はこの時点で確定", tone: "green" },
    { id: "s4", kind: "pill", x: 920, y: 730, label: "200 OK", tone: "gray" },

    // ---- 3. commit
    { id: "c0", kind: "pill", x: 1470, y: 215, label: "コミットする(メッセージ)", tone: "gray" },
    { id: "c1", kind: "diamond", x: 1470, y: 335, label: "ステージ済みが\nある?", tone: "blue" },
    { id: "c1x", kind: "box", x: 1730, y: 335, w: 200, label: "400  NothingStaged", tone: "red" },
    { id: "c2", kind: "diamond", x: 1470, y: 475, label: "追加 / 変更の内容は\nすべて保存済み?", tone: "blue", w: 230 },
    { id: "c2x", kind: "box", x: 1730, y: 475, w: 200, label: "400  パス一覧つき", tone: "red" },
    { id: "c3", kind: "box", x: 1470, y: 615, w: 380, label: "親のツリーをコピーし、変更を適用\n(新しいスナップショット)", tone: "green" },
    { id: "c4", kind: "box", x: 1470, y: 745, w: 420, label: "commit_hash = SHA-256(\nrepo · 親 · ブランチ · 作者 · メッセージ\n· 変更(パス順) · 時刻 )", tone: "green" },
    { id: "c5", kind: "box", x: 1470, y: 890, w: 420, label: "commits / commit_files / tree_entries へ INSERT\nブランチ先頭を更新、pending / staged を削除", tone: "green" },
    { id: "c6", kind: "pill", x: 1470, y: 1000, label: "COMMIT → 201 Created", tone: "gray", w: 240 },
  ],

  edges: [
    { from: "u0", to: "u1" },
    { from: "u1", to: "u1x", label: "いいえ", tone: "red", dashed: true },
    { from: "u1", to: "u2", label: "はい", labelDx: 22 },
    { from: "u2", to: "u3" },
    { from: "u3", to: "u4" },
    { from: "u4", to: "u5" },

    { from: "s0", to: "s1" },
    { from: "s1", to: "s1x", label: "いいえ", tone: "red", dashed: true, labelDy: -14 },
    { from: "s1", to: "s2", label: "はい", labelDx: 22 },
    { from: "s2", to: "s3" },
    { from: "s3", to: "s4" },

    { from: "c0", to: "c1" },
    { from: "c1", to: "c1x", label: "いいえ", tone: "red", dashed: true, labelDy: -14 },
    { from: "c1", to: "c2", label: "はい", labelDx: 22 },
    { from: "c2", to: "c2x", label: "いいえ", tone: "red", dashed: true, labelDy: -14 },
    { from: "c2", to: "c3", label: "はい", labelDx: 22 },
    { from: "c3", to: "c4" },
    { from: "c4", to: "c5" },
    { from: "c5", to: "c6" },
  ],

  labels: [
    { x: 40, y: 64, text: "ファイルがコミットになるまで", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "クライアントは 1 → 2 → 3 の順に呼ぶ。内容(blob)は1で保存され、コミットは「どの内容か」の hash だけを持つ", anchor: "start", size: 13, mono: false },
    { x: 920, y: 830, text: "ステージ後に同じパスを再アップロードしても、\n次のコミットが取り込む内容は変わらない\n(git の index と同じ)", size: 12, mono: false, tone: "blue" },
  ],
};
