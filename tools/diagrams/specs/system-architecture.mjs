// System architecture: who talks to whom, and where the data lives.
export default {
  name: "system-architecture",
  title: "Lore Ecosystem のシステム構成図",
  desc:
    "ブラウザ、LoreForge Client、Server Admin の3つのクライアントが、同じ lorehub-api(Rust / Axum)を使う。Webは Next.js サーバー経由、デスクトップは直接接続。API は認証ミドルウェア、ハンドラ、認可、VCSストアで構成され、SQLite と blobs/ ディレクトリに保存する。下段に開発・配布の流れ(トークン生成、CI、Docker、インストーラー)を示す。",
  width: 1800,
  height: 1120,

  groups: [
    { id: "clients", label: "CLIENTS", x: 40, y: 130, w: 290, h: 660, tone: "blue" },
    { id: "api", label: "LOREHUB-API  (RUST / AXUM)", x: 780, y: 130, w: 620, h: 660, tone: "green" },
    { id: "store", label: "PERSISTENCE  (/data VOLUME)", x: 1500, y: 330, w: 260, h: 430, tone: "orange" },
    { id: "dev", label: "BUILD & SHIP", x: 40, y: 880, w: 1720, h: 210, tone: "gray", dashed: true },
  ],

  nodes: [
    { id: "browser", x: 185, y: 230, icon: "browser", label: "ブラウザ", caption: "利用者", tone: "blue" },
    { id: "client", x: 185, y: 450, icon: "monitor", label: "LoreForge Client", caption: "Qt6 · QML · C++20", tone: "blue" },
    { id: "admin", x: 185, y: 670, icon: "gear", label: "Server Admin", caption: "Qt6 · QML · C++20", tone: "blue" },

    { id: "next", x: 500, y: 230, icon: "server", label: "LoreHub Web", caption: "Next.js · RSC · /api プロキシ", tone: "blue" },

    { id: "mw", x: 900, y: 230, icon: "shield", label: "認証ミドルウェア", caption: "require_auth · セッション", tone: "green" },
    { id: "app", x: 1220, y: 230, icon: "database", label: "AppState", caption: "RwLock · メンバー/ACL/PR", tone: "green" },
    { id: "handlers", x: 900, y: 450, icon: "braces", label: "ハンドラ", caption: "handlers.rs", tone: "green" },
    { id: "vcs", x: 1220, y: 450, icon: "branch", label: "VCS ストア", caption: "vcs_store.rs", tone: "green" },
    { id: "authz", x: 900, y: 670, icon: "lock", label: "認可", caption: "ロール + パスACL", tone: "green" },

    { id: "sqlite", x: 1640, y: 450, icon: "database", label: "SQLite", caption: "lorehub.db", tone: "orange" },
    { id: "files", x: 1640, y: 650, icon: "cube", label: "blobs/", caption: "{repo}/{hash2}/{hash}", tone: "orange" },

    { id: "tokens", x: 200, y: 975, icon: "swatches", label: "design-tokens", caption: "tokens.json", tone: "gray" },
    { id: "gen", x: 540, y: 975, icon: "terminal", label: "gen-tokens.mjs", caption: "CSS + Theme.qml を生成", tone: "gray" },
    { id: "ci", x: 880, y: 975, icon: "workflow", label: "GitHub Actions", caption: "test · lint · --check", tone: "gray" },
    { id: "docker", x: 1220, y: 975, icon: "cube", label: "Docker Compose", caption: "api + web", tone: "gray" },
    { id: "pkg", x: 1560, y: 975, icon: "doc", label: "インストーラー", caption: "NSIS · Inno Setup", tone: "gray" },
  ],

  edges: [
    { from: "browser", to: "next", label: "HTTPS", labelDy: -14 },
    { from: "next", to: "mw", label: "fetch + Cookie", fromOffset: [0, -22], toOffset: [0, -22], labelDy: -14 },
    { from: "client", to: "mw", label: "QNetworkAccessManager", via: [[640, 450], [640, 230]] },
    { from: "admin", to: "mw", label: "PUT /api/access-control/entries", via: [[700, 670], [700, 252]], toOffset: [0, 22], labelAt: 0.55 },

    { from: "mw", to: "handlers", label: "認証済み", labelDx: 0 },
    { from: "handlers", to: "authz", label: "権限を確認 (403)", dashed: true, tone: "gray" },
    { from: "handlers", to: "vcs", label: "VCS 操作", fromOffset: [0, 14], toOffset: [0, 14], labelDy: 18 },
    { from: "handlers", to: "app", via: [[1060, 436], [1060, 230]], fromOffset: [0, -14], dashed: true, tone: "gray", label: "メンバー · セッション", labelSeg: 1, labelDx: 0 },

    { from: "vcs", to: "sqlite", label: "SQL (sqlx)" },
    { from: "vcs", to: "files", via: [[1220, 650]], label: "内容の実体", labelSeg: 1, labelDy: -14 },
    { from: "app", to: "sqlite", via: [[1640, 230]], dashed: true, tone: "gray", label: "kv_store (JSON)", labelSeg: 0, labelAt: 0.72, labelDy: -14 },

    { from: "tokens", to: "gen", label: "読み込み", tone: "gray" },
    { from: "gen", to: "ci", label: "--check で差分検知", tone: "gray", dashed: true },
    { from: "ci", to: "docker", label: "ビルド", tone: "gray" },
    { from: "docker", to: "pkg", label: "配布物", tone: "gray", dashed: true },
  ],

  labels: [
    { x: 40, y: 64, text: "Lore Ecosystem — システム構成図", anchor: "start", size: 26, mono: false, weight: 700, ink: true },
    { x: 40, y: 92, text: "3つのクライアントが、1つの Rust バックエンドと1つのデザイントークンを共有する", anchor: "start", size: 13, mono: false },
    { x: 1760, y: 1108, text: "実線 = リクエストの流れ / 点線 = 補助的な参照・検知", anchor: "end", size: 12 },
  ],
};
