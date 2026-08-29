# 引継: `xs_boot!` の多 package 化 (1 つの boot から複数の Perl 名前空間へ登録)

作成: 2026-08-29 (perl-LibPerlRs-PartialEval の EvalCapture 着手セッションからの依頼指示書)。
下流 = `perl-LibPerlRs-PartialEval` の `LibPerlRs::EvalCapture` 実装
(`~/db/github/perl-LibPerlRs-PartialEval/docs/plan/evalcapture-2026-08.md` = 文書 A) の
前提作業。

## Status: 実装完了 (2026-08-29)

本文書の内容は libperl-rs セッションで実装済み。実績:

- `libperl-macros/src/xs_boot.rs`: `module` キーワード + `package`/`subs` の
  繰り返しに対応。パーサと boot 名決定を単体テスト 11 本で検査
  (`cargo test -p libperl-macros`)
- `xs-demo-mytest2`: 2 つ目の package `Mytest2::Extra` を追加 (`module` 明示形)。
  `t/10_multi_package.t` が両名前空間への登録と相互不干渉を検査
- 後方互換: `xs-demo-mytest` は無改修で PASS (17 tests)
- 下流: `LibPerlRs::Inspect` 72 tests PASS / `LibPerlRs::PartialEval` 170 tests PASS
  (どちらも再ビルド後)

以下は当初の仕様書。

## 0. 動機

下流は `LibPerlRs::EvalCapture` を `LibPerlRs::PartialEval` と**同じ cdylib**に同居させたい
(捕捉レジストリを Rust レベルで部分評価器と共有するため)。ところが:

- XSLoader が呼ぶ boot 関数は 1 つだけ (`XSLoader::load('LibPerlRs::PartialEval')` →
  `boot_LibPerlRs__PartialEval`)
- 現行の `xs_boot!` は `package` 1 つしか受けず、全 sub を `<package>::<sub>` で登録する

そのため今のままでは `LibPerlRs::EvalCapture::*` という名前の XSUB を作れない。
1 つの boot 関数から複数の Perl 名前空間へ登録できるようにする。

これは .xs が昔からやっていること (`MODULE = Foo  PACKAGE = Foo::Bar` で名前空間を
切り替えられる) の Rust 側への翻訳であり、libperl-rs 固有の発明ではない。

## 1. 構文 (決定済み)

```rust
xs_boot! {
    module  = "LibPerlRs::PartialEval";   // -> boot_LibPerlRs__PartialEval
    package = "LibPerlRs::PartialEval";
    subs    = [engine_version, _compile, _partial_eval];
    package = "LibPerlRs::EvalCapture";
    subs    = [_start, _stop, _captured, _release];
}
```

規則:

- `module` は **boot 関数名**を決める (`::` → `__`、`boot_` 前置)。**省略時は最初の
  `package`** — したがって既存の呼び出しは無改修で動く
- `package` / `subs` の対を任意個繰り返せる。登録名は各対の `<package>::<sub>`
- `Perl_xs_boot_epilog` に渡す個数は全グループの合計

**後方互換の対象** (無改修で動き続けること):

| 呼び出し元 | 形 |
|---|---|
| `xs-demo-mytest/src/lib.rs:32` | `package = "Mytest"; subs = [is_even, round];` |
| `xs-demo-mytest2/src/lib.rs:283` | `package = "Mytest2"; subs = [... 24 個 ...];` |
| `~/db/github/perl-LibPerlRs-Inspect/crates/inspect-xs/src/lib.rs` | 単一 package |
| `~/db/github/perl-LibPerlRs-PartialEval/crates/partial-eval-xs/src/lib.rs:584` | 単一 package (9 個) |

## 2. 変更箇所 — `libperl-macros/src/xs_boot.rs`

現行の構造 (参考):

- `mod kw { custom_keyword!(package); custom_keyword!(subs); }`
- `struct XsBootInput { package: LitStr, subs: Vec<Ident> }`
- `Parse`: `while !input.is_empty()` で `package` / `subs` を 1 回ずつ受ける
- `xs_boot()`: `boot_ident` を package から作り、`registrations` を
  `subs.iter().map(|sub| format!("{pkg}::{sub}"))` で組み、`Perl_newXS_deffile` を並べ、
  最後に `Perl_xs_boot_epilog` を呼ぶ。threaded / 非 threaded で
  `boot_params` / `null_check` / `epilog_call` を切り替えている

変更内容:

1. `mod kw` に `custom_keyword!(module)` を追加
2. 入力構造体を
   ```rust
   struct XsBootInput {
       module: Option<LitStr>,
       groups: Vec<(LitStr, Vec<Ident>)>,
   }
   ```
   に変更
3. `Parse` を「`module` は 0〜1 回、`package` の直後に `subs` が来る対を 0 回以上」に。
   `package` を読んだら次が `subs` であることを要求し、対にして `groups` へ push する
4. `boot_ident` = `module` があればそれ、無ければ `groups[0].0`
5. `registrations` を `groups` の flat_map に (`format!("{pkg}::{sub}")` を
   グループごとの `pkg` で適用)。threaded / 非 threaded の分岐は現行のまま
6. `n_subs` = 全グループの sub 数の合計
7. モジュール先頭の doc コメント (`//! Syntax:` のブロック) を新構文の例に更新。
   `module` 省略時の既定も明記する

`quote!` / `syn` の使い方は現行のまま。proc-macro の制約については
`docs/plan/appendix-rust-proc-macro-limits.md` を参照。

## 3. エラー設計

`syn::Error` で位置付きの診断にすること (メッセージは英語で、既存の
`"missing \`package = \"...\";\`"` の調子に合わせる):

- `package` の直後が `subs` でない
- `subs` が先行する `package` なしに現れた
- グループが 0 個 (`subs` も `package` も無い)
- `module` が 2 回指定された

## 4. テスト

1. **多 package の実動作**: `xs-demo-mytest2` に 2 つ目の package
   (例 `Mytest2::Extra`) を足し、既存の Perl 側テストで両名前空間の XSUB が
   引けることを確認する。`module` を明示した形と省略した形の両方を通す
2. **後方互換の回帰**: `xs-demo-mytest` は単一 package 形のまま触らない —
   これがそのまま回帰テストになる
3. **下流ビルド確認** (path 依存なので sibling checkout をそのままビルドできる):
   ```console
   $ cd ~/db/github/perl-LibPerlRs-Inspect && perl Makefile.PL && make debug && prove -b t/
   $ cd ~/db/github/perl-LibPerlRs-PartialEval && perl Makefile.PL && make debug && prove -b t/
   ```
4. `cargo test --all --examples` / `cargo clippy --all-targets` / `cargo fmt`

## 5. リリース方針

下流 (`perl-LibPerlRs-Inspect` / `perl-LibPerlRs-PartialEval`) は libperl-rs を
**path 依存**で参照しているので、開発中は crates.io リリース不要。
下流 dist を CPAN へ公開する段になって初めて libperl-macros / libperl-rs の
リリースが要る。

なお下流 (文書 A §8) は本タスクの完了を待たずに進行できる設計になっている
(暫定的に単一 package 形で `LibPerlRs::PartialEval::_capture_*` として登録し、
本タスク完了後に多 package 形へ書き換える)。したがって本タスクは
**下流をブロックしていない** — 急がなくてよい。

## 6. 参照

- `libperl-macros/src/xs_boot.rs` (変更対象)
- `docs/plan/README.md` §(XS 章、569 行目付近 —
  「`Perl_newXS_deffile` での登録 + `Perl_xs_boot_epilog` で締める」)
- `docs/plan/appendix-rust-proc-macro-limits.md`
- `docs/plan/roadmap-next-projects-2026-08.md` (ファミリー全体のロードマップ。
  本タスクは第 2 弾 `LibPerlRs::EvalCapture` の前提作業)
- 下流: `~/db/github/perl-LibPerlRs-PartialEval/docs/plan/evalcapture-2026-08.md` (文書 A)
