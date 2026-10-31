# 02. Rust の基礎(このコードを読む範囲で)

**この章で分かること**: `lorehub-api` を読むために必要な Rust だけを、実コードを題材に説明します。Rust を完全に学ぶ章ではありません。「この行は何をしているか」を説明できるようになるのが目標です。

## 2.1 値・型・関数

```rust
pub fn format_size_label(bytes: i64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    /* ... */
}
```

- `fn 名前(引数: 型) -> 戻り値の型`。型は必ず書く(推論できる所は省略可)。
- `i64` は 64 ビットの符号つき整数、`String` は文字列(所有する)。
- `format!("{bytes} B")` は文字列を作るマクロ(`!` が付くのはマクロ)。`{bytes}` は変数をその場で埋め込む。
- 最後の式に `;` を付けないと、それが**戻り値**になる(`return` は途中で抜けたいときだけ)。
- 変数は既定で**変更不可**。変えたいときは `let mut x`。

## 2.2 所有権と借用 — Rust の中心

Rust には「ガベージコレクタ」がありません。代わりに、**値には所有者が1人**というルールで、いつメモリを解放するかをコンパイラが決めます。

```rust
let a = String::from("hello");
let b = a;            // a の所有権が b に「移動」した
// println!("{a}");   // ← エラー。a はもう使えない
```

コピーしたいときは明示します。

```rust
let b = a.clone();    // 中身を複製。a も b も使える
```

関数に渡すときも同じです。所有権を渡したくないときは、**借用**(`&`)します。

```rust
fn length(s: &str) -> usize { s.len() }   // &str = 文字列を「借りて」読むだけ
let n = length(&a);                        // a の所有権は動かない
```

| 書き方 | 意味 |
|---|---|
| `T` | 所有する |
| `&T` | 読み取り専用で借りる(同時に何人でも) |
| `&mut T` | 書き込みつきで借りる(同時に**1人だけ**) |

実コードでの例: [`handlers.rs`](../../lorehub-api/src/handlers.rs) の `login` は、ロックの中の値を `.cloned()` で複製して取り出し、**ロックを手放してから**重い処理をします。

```rust
let (hash, user) = {
    let state = ctx.read().await;           // ロックを借りる
    /* ... */
    (hash, user)                            // 複製した値を返す
};                                          // ← ここで state(ロック)が解放される
```

「ロックの中の値を借りたまま長い処理をしない」ために、必要な物だけ複製して出る、が定石です。

## 2.3 `Option` と `Result` — 「無い」「失敗した」を型で表す

Rust には `null` も例外もありません。

```rust
enum Option<T> { Some(T), None }          // 値があるか、無いか
enum Result<T, E> { Ok(T), Err(E) }       // 成功か、失敗か
```

実コード([`models.rs`](../../lorehub-api/src/models.rs))の `Commit`:

```rust
pub description: Option<String>,          // 説明は付けても付けなくてもよい
```

取り出すには `match` か、短い書き方を使います。

```rust
match commit.description {
    Some(text) => println!("{text}"),
    None => println!("(説明なし)"),
}

if let Some(text) = &commit.description { /* あるときだけ */ }
```

### `let ... else` — 「無ければ早く帰る」

このコードで**最もよく見る形**です。

```rust
let Some(hash) = state.credentials.get(&body.email).cloned() else {
    return unauthorized();       // 無ければここで 401 を返して終わり
};
// ここから先、hash は「ある」ことが保証されている
```

### `?` — 失敗を呼び出し元へ渡す

[`auth.rs`](../../lorehub-api/src/auth.rs) の `extract_cookie`:

```rust
let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
```

`?` は「`None`(または `Err`)ならこの関数からそのまま返す、`Some`(`Ok`)なら中身を取り出す」です。`.ok()` は `Result` を `Option` に変えます。

### `unwrap` / `expect` — 「無いはずがない」と言い切る

`expect("メッセージ")` は、`None`/`Err` ならプログラムを**異常終了(panic)**させます。このコードにも多く出てきます(例: `.expect("query commit_files")`)。

> **保守の目線**: DB の失敗を `expect` で受けると、その要求は 500 になるのではなく**タスクが落ちます**。「起きるはずがない」と言える所(起動時の設定など)なら良いですが、実行中の DB エラーは `Result` で返して 500 にするほうが堅牢です。直すのは時間のかかる作業なので、今は「そういう設計だ」と知っておいてください。

## 2.4 構造体・列挙型・`match`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
    pub size_delta_label: String,
}
```

- `struct` = 名前つきの項目の束。
- `#[derive(...)]` = 決まった機能を自動で付ける(`Debug`=表示、`Clone`=複製、`Serialize`/`Deserialize`=JSON との相互変換)。
- `#[serde(rename_all = "camelCase")]` = Rust 側は `size_delta_label`、**JSON では `sizeDeltaLabel`** にする。TypeScript(Web)の型と名前を揃えるため。

```rust
pub enum FileChangeType { Added, Modified, Deleted }

match change.change_type {
    FileChangeType::Added => "追加",
    FileChangeType::Modified => "変更",
    FileChangeType::Deleted => "削除",
}
```

`match` は**全部の場合を書かないとコンパイルできません**。後から `Renamed` を足すと、`match` を書いた全箇所が「足りない」とエラーになり、直し忘れが起きません(これがとても強力です)。

`matches!` は「この形か?」だけを真偽で返す短縮形です。

```rust
fn is_owner_or_admin(user: &OrgMember) -> bool {
    matches!(user.role, MemberRole::Owner | MemberRole::Admin)
}
```

## 2.5 `async` / `await`

Web サーバーは、DB やファイルの待ち時間に**他の要求を処理**したいので、待つ処理を `async` にします。

```rust
pub async fn health() -> Json<serde_json::Value> { /* ... */ }

let state = ctx.read().await;       // ロックが取れるまで「待つ」(他の仕事は進む)
```

- `async fn` は呼ぶと「やることの予定(Future)」を返し、`.await` したときに実際に進む。
- `.await` できるのは `async` の中だけ。
- 実行するのは `tokio`(ランタイム)。`#[tokio::main]` や `#[tokio::test]` が、その準備をする。

> **落とし穴**: `.await` していない間は、他の要求が同じスレッドで進みます。**重い計算(CPU を使い続ける処理)**を `async` の中で直接やると、そのスレッドの他の仕事が止まります。Argon2 のような重い処理は `tokio::task::spawn_blocking` に出します(`login` がそうしている)。

## 2.6 共有される状態: `Arc` と `RwLock`

複数の要求が同じデータ(`AppState`)を読み書きします。

```rust
pub type SharedState = Arc<AppContext>;      // Arc = 複数の持ち主で共有する(参照カウント)

pub struct AppContext {
    state: RwLock<AppState>,                  // RwLock = 読む人は何人でも、書く人は1人だけ
    pub db: SqlitePool,
    /* ... */
}
```

- `Arc<T>` は、`clone()` しても中身は複製せず、**持ち主の数を数えるだけ**。最後の持ち主が消えたら解放。
- `RwLock` は「読み取りロック(複数同時 OK)」と「書き込みロック(1人だけ、読む人も待つ)」。**書き込みロックを持っている間は、読む要求も全部止まる**。だから短く持つ。

```rust
let mut state = ctx.write().await;   // 書き込みロック
state.sessions.remove(token);
let sessions = state.sessions.clone();
drop(state);                          // ← 明示的に手放す(以後は他の要求も進める)
crate::db::save_blob(&ctx.db, "sessions", &sessions).await;   // 遅いDB書き込みはロックの外で
```

この `clone → drop → 保存` の形は、このコードに何度も出ます。理由は「遅い処理(DB)の間、ロックを持たない」ためです。

## 2.6.1 ライフタイム(`'_`)を見たら

```rust
pub async fn create_commit(pool: &SqlitePool, new: NewCommit<'_>) -> Result<Commit, CommitError>
```

`NewCommit<'_>` の `'_` は「`NewCommit` は**借りた文字列**を持っているので、借りもとより長く生きてはいけない」という印です。読むときは「借りた値を持つ構造体なんだな」と思えば十分です。

## 2.7 トレイトと `impl`

`trait` は「できること」の約束です(他の言語のインターフェース)。

```rust
Json(serde_json::json!({ "error": message }))
    .into_response()                // IntoResponse というトレイトの機能
```

ハンドラが `Response` や `Json<...>` や `StatusCode` を返せるのは、それらが `IntoResponse` を実装しているからです(03章)。

## 2.8 クロージャとイテレータ

```rust
let user = state
    .org_members
    .iter()                                     // 要素を順に見る
    .find(|m| m.email == body.email)            // 条件に合う最初の1つ(|m| ... はクロージャ=無名関数)
    .cloned();                                  // 借りた値を複製して Option<OrgMember> に
```

```rust
let missing: Vec<String> = pending
    .iter()
    .filter(|p| /* 条件 */)                      // 絞り込み
    .map(|p| p.path.clone())                     // 変換
    .collect();                                  // Vec にまとめる
```

`for` ループで書けることを、`filter`/`map`/`collect` で**宣言的に**書けます。

## 2.9 モジュールとテスト

- `mod foo;` = `foo.rs` を取り込む。ファイル=モジュール。
- `pub` が付いたものだけ、他のモジュールから見える。
- テストは `#[test]`(同期)か `#[tokio::test]`(async)。`cargo test` で全部走る。

```rust
#[cfg(test)]
mod size_label_tests {
    use super::format_size_label;

    #[test]
    fn rounding_up_to_1024_moves_to_the_next_unit() {
        assert_eq!(format_size_label(1024 * 1024 - 1), "1.0 MB");
    }
}
```

`#[cfg(test)]` は「テストのときだけコンパイルする」。このテストは、実際にあった不具合(1,048,575 バイトが「1024.0 KB」と出る)の再現でした。

## 2.10 エラーメッセージの読み方

Rust のコンパイルエラーは親切です。**1行目と、`^^^` の指す所**を読みます。

```
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:4:16
  |
2 |     let a = String::from("hello");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
3 |     let b = a;
  |             - value moved here
4 |     println!("{a}");
  |                ^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
```

「`a` は 3 行目で移動したのに 4 行目で使った」と、場所つきで教えてくれます。`help:` の提案に従うと直ることが多いです。

## 2.11 まとめ

- 値の所有者は1人。移動・複製(`clone`)・借用(`&`)を使い分ける。
- 「無い」は `Option`、「失敗」は `Result`。`let ... else` と `?` で早く抜ける。
- `match` は全部の場合を書く。型が変更の見落としを防ぐ。
- 待つ処理は `async`、重い計算は `spawn_blocking`。
- 共有状態は `Arc<RwLock<...>>`。**書き込みロックは短く**、遅い処理はロックの外で。

## 2.12 演習

1. `lorehub-api/src/vcs_store.rs` の `size_label_tests` に、`format_size_label(0)` と `format_size_label(1023)` を確かめるテストが既にある。`cargo test size_label_tests` を実行して、出力の読み方を確認する。
2. 次の関数を書く。`fn initials(name: &str) -> String` は「Aiko Tanaka」→「AT」を返す。(ヒント: `split_whitespace()`、`chars().next()`、`flat_map`/`collect`。空の名前では空文字を返す。)テストも `#[cfg(test)]` に書く。
3. `handlers.rs` で `unwrap()` が使われている行を `grep -n "unwrap()" src/handlers.rs` で探す。それぞれ「本当に起きるはずがないか」を理由つきで判断する。(例: `auth::session_cookie(&access_token).parse().unwrap()` は、自分で組み立てた Cookie 文字列なので失敗しない。)
4. **発展**: `login` のロックを、わざと元の形(書き込みロックを握ったまま検証)に戻すと、どんな問題が理屈の上で起きるか説明する。

<details>
<summary>演習2の解答例</summary>

```rust
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::initials;
    #[test]
    fn takes_the_first_letter_of_each_word() {
        assert_eq!(initials("Aiko Tanaka"), "AT");
        assert_eq!(initials("  aiko   tanaka "), "AT");
        assert_eq!(initials(""), "");
    }
}
```

</details>

前: [01. Web の基礎](01-web-basics.md) / 次: [03. Axum の API](03-axum-api.md)
