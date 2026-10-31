# 03. Axum の API: ルーター・ハンドラ・ミドルウェア・テスト

**この章で分かること**: `lorehub-api` が「要求を受けて、応答を返す」仕組みを、1本の要求が通る道順で理解します。読み終えると、新しいエンドポイントを足せます(手順は [13章](13-extend-recipes.md))。

前提: [01](01-web-basics.md)(HTTP と Cookie)、[02](02-rust-basics.md)(`async`、`Option`/`Result`)。

## 3.1 1本の要求が通る道

![リクエストの判定フロー](../diagrams/authz-flow.svg)

要求は、外側の「層(layer)」から順に通り、最後に**ハンドラ**に届きます。応答は逆順に戻ります。

```
 ブラウザ/Qt
   │  ▼ 要求
   ├─ RequestBodyLimitLayer   本文が大きすぎれば 413 で打ち切り
   ├─ CorsLayer               許可したオリジン以外の越境要求を制限
   ├─ PrometheusLayer         件数・所要時間を数える(/metrics)
   ├─ TraceLayer              アクセスログ
   ├─ (login と forgot-password だけ) GovernorLayer   送信元 IP 単位のレート制限(429)
   ├─ require_auth            Cookie → sessions → ユーザー特定(401)
   └─ ハンドラ                 認可チェック(403)→ 本処理 → 応答
```

この順は [`main.rs`](../../lorehub-api/src/main.rs) の `build_router` に、そのまま書いてあります。**`.layer(...)` は、後から足したものほど外側**になります(最後に足した `RequestBodyLimitLayer` が最外)。

## 3.2 ハンドラ: 関数がそのままエンドポイントになる

```rust
pub async fn get_tree(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    /* ... */
}
```

引数の `State(ctx)`、`Path(slug)` を **抽出子(extractor)** と呼びます。「要求のどこから何を取り出すか」を、型で宣言します。

| 抽出子 | 取り出すもの | 例 |
|---|---|---|
| `State<SharedState>` | アプリ全体で共有している状態(`AppContext`) | DB、`AppState` |
| `Path<String>` | URL のパラメータ | `/repositories/{slug}/tree` の `slug` |
| `Json<T>` | 本文の JSON(`T` に変換) | `Json(body): Json<CreateCommitRequest>` |
| `Extension<T>` | 前の層が要求に**付け足した**値 | `Extension(user): Extension<OrgMember>`(`require_auth` が付ける) |
| `HeaderMap` | ヘッダー全部 | Cookie を自分で読むとき |

> **約束**: 本文を読む抽出子(`Json`)は、**必ず最後の引数**。本文は1回しか読めないため。

戻り値は「応答に変換できるもの」(`IntoResponse`)なら何でもよく、実際には次の形を使い分けています。

```rust
Json(entries).into_response()                      // 200 + JSON
StatusCode::NO_CONTENT.into_response()             // 204、本文なし
(headers, Json(json!({ "user": user }))).into_response()   // ヘッダー(Set-Cookie)つき
not_found("repository not found")                  // 自作の小さな関数(404 + {"error": "..."})
```

エラー応答の小さな関数(`not_found`/`unauthorized`/`bad_request`/`forbidden`)は `handlers.rs` の先頭にあり、**応答の形を1か所に揃える**ためのものです(`{ "error": "メッセージ" }`)。

## 3.3 ルーター: URL とハンドラを結ぶ

```rust
Router::new()
    .route("/api/repositories", get(handlers::list_repositories).post(handlers::create_repository))
    .route("/api/repositories/{slug}", get(handlers::get_repository)
                                        .patch(handlers::update_repository)
                                        .delete(handlers::delete_repository))
    .route("/api/repositories/{slug}/content/{*path}", get(handlers::get_file_content))
```

- `{slug}` は1区切りのパラメータ、`{*path}` は**残り全部**(スラッシュを含むファイルパス用)。
- 同じ URL にメソッドを並べると、メソッドごとに別のハンドラになる。

### 公開ルートと保護ルート

`build_router` は、ルーターを2つに分けています。

```rust
let public_routes = login_route.route("/api/health", ...).route("/api/auth/refresh", ...) /* ... */;

let protected_routes = Router::new()
    .route("/api/auth/me", ...)
    /* ... 残り全部 ... */
    .layer(middleware::from_fn_with_state(shared_state.clone(), handlers::require_auth));

public_routes.merge(protected_routes) /* ... + 外側の層 */
```

`.layer(require_auth)` は、**その時点までに足したルートにだけ**かかります。だから「公開にしたいもの」は `public_routes` に、「それ以外」は `protected_routes` に置く。**新しいエンドポイントは、既定で `protected_routes` に足す**のが安全側です(足し忘れても、保護されない事故にならない)。

## 3.4 ミドルウェア: `require_auth`

[`handlers.rs`](../../lorehub-api/src/handlers.rs) の `require_auth` は、要求がハンドラに届く前に毎回走る関数です。

```rust
pub async fn require_auth(State(ctx): State<SharedState>, mut request: Request, next: Next) -> Response {
    let token = auth::extract_session_token(request.headers());   // 1. Cookie からトークン
    /* sessions の表を引く。無い・期限切れなら 401 */                // 2. 表を引く
    let Some(user) = /* org_members から該当メンバー */ else { return unauthorized(); };
    request.extensions_mut().insert(user);                         // 3. 「この要求はこのユーザー」と付け足す
    next.run(request).await                                         // 4. 次の層/ハンドラへ
}
```

- `next.run(request)` を呼ぶと**先に進む**。呼ばずに `Response` を返せば、そこで打ち切り(401)。
- ハンドラは `Extension(user)` で受け取るだけ。**ハンドラの中でセッションを調べ直さない**。

## 3.5 共有状態: `AppContext`

```rust
pub struct AppContext {
    state: RwLock<AppState>,     // メンバー・セッション・ACL・PR など(メモリ上)
    pub db: SqlitePool,          // SQLite への接続プール
    pub blob_base_dir: PathBuf,  // ファイル実体の置き場
    /* ... */
}
```

ハンドラは `ctx.read().await` / `ctx.write().await` で `AppState` に触り、`ctx.db` で SQL を発行します。VCS のデータ(リポジトリ・コミット・ツリー…)は `AppState` に**無く**、`repo_store` / `vcs_store` が SQL を直接叩きます([04章](04-sql-and-storage.md))。

## 3.6 テスト: 本物のサーバーを立てずに、ルーターを直接呼ぶ

API のテストは、`src/tests/` にある**統合テスト**です。TCP ポートを開かず、`Router` に要求を直接渡します(`tower::ServiceExt::oneshot`)。

```rust
#[tokio::test]
async fn commit_with_no_pending_changes_is_400() {
    let app = test_app().await;                                  // 1. テスト専用のアプリ(メモリ上のDB)
    let cookie = login_cookie(&app, DEMO_EMAIL).await;           // 2. ログインして Cookie を得る

    let commit_req = json_request(                               // 3. 要求を作る
        "POST",
        &format!("/api/repositories/{SEEDED_SLUG}/commits"),
        Some(&cookie),
        serde_json::json!({ "message": "nothing staged" }),
    );
    let resp = send(&app, commit_req).await;                     // 4. 送る
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);          // 5. 確かめる
}
```

ヘルパーは [`tests/mod.rs`](../../lorehub-api/src/tests/mod.rs) にあります。

| ヘルパー | 役割 |
|---|---|
| `test_app()` | 本番と**同じ** `build_router` を、`:memory:` の DB と専用の一時ディレクトリで作る |
| `login_cookie(&app, email)` | ログインして `Cookie` ヘッダー用の文字列を返す |
| `json_request(method, uri, cookie, body)` / `plain_request(...)` | 要求を作る |
| `send(&app, req)` | `oneshot` で送る |
| `body_json(resp)` | 応答本文を JSON に |

**なぜ各テストが独立か**: `test_app()` が毎回、**専用のメモリ DB と専用の一時ディレクトリ**を作るので、テストを並列に走らせても互いに干渉しません。この性質が、`cargo test` を速く・安定させています。

## 3.7 動かして確かめる

```bash
cd lorehub-api
cargo test                        # 全部(約10秒)
cargo test authz                  # 名前に "authz" を含むテストだけ
cargo test -- --nocapture         # テスト中の println! も表示
```

## 3.8 まとめ

- ハンドラは `async fn`。引数の**型**で「要求のどこから何を取るか」を宣言し、戻り値は `IntoResponse`。
- ルーターが URL とハンドラを結ぶ。`{slug}` と `{*path}`。
- `.layer` は後に足したものほど外側。`require_auth` は `protected_routes` だけにかかる。**新しい道は既定で保護側に**。
- 要求に付け足した値は `Extension` で受け取る。
- テストは `Router` を直接呼ぶ。各テストが専用の DB とディレクトリを持つ。

## 3.9 演習

1. `cargo test authz -- --nocapture` を実行し、どんなテスト名が走るか眺める。1つ選んで、上の5ステップ(作る→ログイン→要求→送る→確かめる)に当てはめる。
2. `GET /api/health` を `curl -i` で呼び、`/metrics` も呼ぶ。`/metrics` に `axum_http_requests_total{method="GET",status="200",endpoint="/api/health"}` のような行があることを確かめ、`/api/health` を数回呼んだ後で数が増えることを確認する。
3. **読解**: `main.rs` の `build_router` で、`login_route` と `forgot_password_route` に**別々の** `GovernorConfigBuilder` が使われている。1つの設定を共有したら、どんな不都合があるか?(ヒント: 片方の試行回数が、もう片方の枠を食う。)
4. **発展**: `GET /api/ping` を `{ "pong": true }` で返す新しいエンドポイントを、`protected_routes` に足し、テストを書く。(手順は [13章のレシピ A](13-extend-recipes.md))

前: [02. Rust の基礎](02-rust-basics.md) / 次: [04. SQL と永続化](04-sql-and-storage.md)
