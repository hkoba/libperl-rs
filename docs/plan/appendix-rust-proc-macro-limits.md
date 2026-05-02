# 付録: Rust proc-macro の構文上の限界

## 動機

C の Perl API では `pTHX_` (関数定義のシグネチャに「`my_perl` 引数 +
カンマ」または「空」を挿入)、`aTHX_` (関数呼び出しの引数列に「`my_perl`
+ カンマ」または「空」を挿入) が、threaded/non-threaded を 1 ソースで
吸収するための主役である。

これに相当するものを Rust の proc-macro で作れるか? という問題。
ChatGPT は Rust Reference の以下の記述を根拠に「**できる**」と答えた:

> Function-like procedural macros may be invoked in any macro invocation
> position, which includes statements, expressions, patterns, type
> expressions, item positions, including items in extern blocks ...

これを実機で検証した結果、**ChatGPT の回答は誤り**と判明した。本付録は
その実験記録と正しい解釈をまとめる。

## 実験コード

`/tmp/proc-macro-test/` に最小ワークスペースを作成。proc-macro クレート:

```rust
// macros/src/lib.rs (proc-macro = true)
use proc_macro::TokenStream;

#[proc_macro]
pub fn pTHX_(_input: TokenStream) -> TokenStream {
    "my_perl: *mut u8 ,".parse().unwrap()  // C の pTHX_ 相当の展開
}

#[proc_macro]
pub fn pTHX_empty(_input: TokenStream) -> TokenStream {
    "".parse().unwrap()  // 空展開
}

#[proc_macro]
pub fn aTHX_(_input: TokenStream) -> TokenStream {
    "my_perl ,".parse().unwrap()  // C の aTHX_ 相当の展開
}
```

## TEST A: 単独パラメータ位置 (パラメータが他に無い)

```rust
extern "C" {
    fn perl_alone(pTHX_!());
}
```

期待: 展開後 `fn perl_alone(my_perl: *mut u8 ,);` になる。

実際:

```
error: expected one of `:` or `|`, found `)`
 --> fn perl_alone(pTHX_!());
                            ^
```

→ rustc は **`pTHX_!()` を「パラメータ位置にあるマクロ呼び出し = それ自体
が 1 個のパターン」** として食い、その後に `: 型` が来ることを期待している。
マクロの展開結果ではなく、マクロ呼び出しというパターンが 1 個ある、と
解釈されている。

## TEST B: 空展開でも同じ症状

```rust
pub fn just_empty(pTHX_empty!()) -> i32 { 0 }
```

```
error: expected one of `:` or `|`, found `)`
```

→ 展開後のトークン列で文法を再解釈しているわけではない、ということが
わかる。展開結果が空でも、パターン位置に「マクロ呼び出し」が 1 個ある
という解釈は変わらない。

## TEST C: 複数パラメータの中の 1 つ

```rust
pub fn helper(pTHX_!() x: i32) -> i32 { x + 1 }
```

```
error: expected one of `:` or `|`, found `x`
 --> pub fn helper(pTHX_!() x: i32) -> i32 {
                            ^
```

```
error[E0425]: cannot find value `x` in this scope
 --> x + 1
     ^
```

→ `pTHX_!()` をパターン 1 個として食い終わった後、`:` を期待している
ところに `x` があるのでエラー。`x` は引数として認識されていない。

## TEST D: 関数呼び出しの引数位置

```rust
pub unsafe fn caller(my_perl: *mut u8) -> i32 {
    helper(aTHX_!() 42)
}
```

```
error: macro expansion ignores `,` and any tokens following
       caused by the macro expansion here
note: the usage of `aTHX_!` is likely invalid in expression context
help: you might be missing a semicolon here
   |     helper(aTHX_!(); 42)
```

→ 引数位置のマクロは **1 個の式** として食われ、展開結果が `my_perl ,`
のように「式 + カンマ + 別トークン」だと「マクロ展開以後のトークンは
無視されます」と明示エラーになる。

## なぜ動かないのか — Rust Reference の正しい読み方

引用された "macro invocation position" は

- expression position (展開は **1 個の式**)
- pattern position (展開は **1 個のパターン**)
- type position (展開は **1 個の型**)
- statement position (展開は **1 個の文**)
- item position (展開は **1 個以上の item**)

を指す。**「位置に許される構文要素」ごとに展開結果も完結している必要が
ある**。「extern ブロックの中の item position」も `fn foo(...);` 1 個
丸ごとを吐く位置を指していて、**extern 関数のパラメータリスト内**を
意味しない:

```rust
// 動作する: extern ブロック内の item position
extern "C" {
    perl_ffi_block!();
}

// 動作しない: パラメータ位置はマクロ展開の対象だが、展開結果は完結した
// パターン 1 個でなければならない
extern "C" {
    fn foo(perl_ffi!() x: T);
}
```

C プリプロセッサのような **プレーンなトークン置換** を、Rust の
declarative/proc-macro は採用していない。これは declarative macro でも
proc-macro でも同じ。proc-macro は「より自由」ではなく「より複雑な変換が
できるだけ」。

## 何ができるか / できないか

| 想定 | 可否 | 理由 |
|------|------|------|
| `fn foo(pTHX_!(), x: T)` で引数を 1 個増やす | **不可** | パラメータ位置はパターン要求。展開結果が完結したパターンでないとエラー |
| `f(aTHX_!(), x)` 呼び出し時に引数を 1 個追加 | **不可** | 引数位置は式要求。「式 + カンマ」断片不可 |
| `#[xs_sub] fn my_xs(...)` で関数全体を書き換え | **可** | 属性 proc-macro は **item を丸ごと**書き換える |
| `extern_perl_block! { ... }` で extern ブロック全体を生成 | **可** | item position なので展開結果は item 列 |
| `perl_call!(perl, F(args...))` で呼び出し全体を包む | **可** | expression position、展開も 1 個の式 |
| `Vec<some_type!()>` 等の型位置 | **可** | type position、展開は 1 個の型 |

## 結論: API 設計への帰結

- `aTHX_` / `pTHX_` の **C 完全相当** (引数位置への断片挿入) は実現不能。
  これは Rust の構文規則の問題で、proc-macro でも回避できない。
- 代替として:
  - **`#[xs_sub]` (attribute proc-macro)** — 関数 item 全体を書き換える
    粒度に上げる。threaded なら `my_perl` 引数追加 + 本体冒頭に
    `let perl = ...;` 注入。
  - **`perl_call!(perl, F(args...))` (declarative macro)** — 呼び出し
    全体を包む粒度に上げる。
  - **通常の Rust helper** — 素直に `fn helper(perl: &Perl, ...)`。

「C の `pTHX_` の手触り」は完全には再現できないが、**書き換えの粒度を
「item 単位」「呼び出し単位」に上げれば**、Rust の構文制約下で同等の
ポータビリティ (1 ソースで両モードビルド可能) は確保できる。
