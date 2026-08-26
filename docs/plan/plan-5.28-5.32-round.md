# 引継: perl 5.28〜5.32 対応ラウンド(macrogen 0.1.11 反映 + require-codegen 拡充)

作成: 2026-08-26(perl-LibPerlRs-PartialEval セッションからの依頼指示書)。
下流 = perl-LibPerlRs-PartialEval(GH-16 の部分評価枠組み)の対応 perl
拡大ラウンド第 2 段。第 1 段(5.34〜5.44)は macrogen 0.1.10 反映の
PR #22(ac2bb4b)で完了済み — 本タスクはその 5.28〜5.32 版の再演。

## 0. 前提(依存タスク)

libperl-macrogen 側の skip_codegen 解除タスクが先行:
`~/db/github/libperl-macrogen/doc/plan/unskip-refcnt-padlist-5.28-5.30.md`
(perl 5.28/5.30 で `Perl_SvREFCNT_dec` / `PadlistARRAY` / `PadlistMAX` /
`PAD_SET_CUR` が生成されるようにして **0.1.11 リリース**)。

なお 5.32 は下流 engine 側の修正(`Perl_av_count` → `AvFILLp+1` 置換)
だけで green になる見込みなので、本タスクの主眼は 5.28/5.30。

## 1. 作業内容

1. **macrogen bump**: libperl-sys の libperl-macrogen 依存を 0.1.11 へ
   (PR #22 = ac2bb4b「sys: bump libperl-macrogen to 0.1.10, require
   CopLINE generation」が完全な前例)。
2. **require-codegen.txt 拡充**: `libperl-sys/require-codegen.txt` に
   以下を追加(生成漏れを rustc の E0425 でなく build.rs 段階で理由付き
   fail にする。CopLINE 追加時と同じ):
   ```
   Perl_SvREFCNT_dec
   PadlistARRAY
   PadlistMAX
   PAD_SET_CUR
   ```
   ※ macrogen 側 samples/require-partial-eval.txt と同一セット維持の
   注記がファイル冒頭にあるので、macrogen 側にも同期を依頼(または
   同 PR で言及)。
3. **CI matrix に 5.28 追加**: 現在の matrix は 5.30〜5(latest)。
   5.28 を追加し、5.28/5.30 の threaded が green になることを確認。
   (workflow 内の「特定行番号の関数を表示する」デバッグステップは
   行番号が版でずれるので、5.28 で意味を成さなくても気にしない)
4. **(fallback、必要時のみ)compat shim**: macrogen 側で
   `S_SvREFCNT_dec` 系の解除が難航した場合の代替として、
   `libperl-sys/src/perl_core.rs` に Stack_off_t(PR #19 / 23cb70e)の
   前例に倣った手書き shim を置ける:
   ```rust
   // 5.30 以前: SvREFCNT_dec マクロ相当 (inline.h S_SvREFCNT_dec のミラー)
   #[cfg(not(perlapi_ver32))]
   pub unsafe fn Perl_SvREFCNT_dec(my_perl: *mut PerlInterpreter, sv: *mut SV) {
       if !sv.is_null() {
           let rc = (*sv).sv_refcnt;
           if rc > 1 { (*sv).sv_refcnt = rc - 1; }
           else { Perl_sv_free2(my_perl, sv, rc); }  // bindings.rs に全版存在
       }
   }
   ```
   (Padlist 系はマクロ形状が単純な構造体アクセスなので、こちらも
   最悪 perl_core.rs 直書きで足せる。ただし原則は macrogen 生成で。)

## 2. 検証

- 版ごとに `cargo build` / `cargo test`(threaded)。
- ローカルで perl を切り替えるときは issue #21 の stale bindings 問題に
  注意: `rm -rf target/*/build/libperl-sys-*` してから。
- 生成物確認: `find target -name macro_bindings.rs` の中に上記 4 シンボル
  が `pub unsafe fn` として現れること。

## 3. 下流での最終確認

PartialEval 側 CI(`~/db/github/perl-LibPerlRs-PartialEval`)は
workflow_dispatch input `libperl-rs-ref` でこのリポジトリのブランチを
指して事前検証できる(merge 前に 5.28/5.30/5.32 ithreads probe の
green を確認してから merge するのが第 1 段で確立した手順)。
下流の総括指示書: PartialEval の `docs/TASK-perl-5.28-5.32-2026-08.md`。

## 4. スコープ外

- **non-ithreads**(全版): issue #20 の MULTIPLICITY 吸収層 — 別タスク。
- **5.20〜5.26**: libperl-sys の macro_bindings 生成不良
  (5.26: 1 件、5.24: hv_func MURMUR 型不整合 44 件、5.20:
  unused_parens×3 が -D warnings で error 化)— 次段。
- `Perl_SvREFCNT_inc`: 下流は sv_refcnt フィールド加算で回避済み、不要。

## 5. 実施結果 (2026-08-26 追記、branch macrogen-0.1.11)

§1 の 4 項目を、macrogen 側解決サマリ
(`~/db/github/libperl-macrogen/doc/plan/unskip-refcnt-padlist-5.28-5.30.md`
§0' の「下流タスクへの注意」)に合わせて次の形で実施:

1. **macrogen 0.1.11 bump** — crates.io 公開 (apidoc data 1.14) を確認の上
   Cargo.toml / Cargo.lock を更新。
2. **require-codegen**: §1-2 の 4 つのうち無条件 require できるのは
   `PadlistARRAY` / `PadlistMAX` のみと判明:
   - `PAD_SET_CUR` は non-threaded perl では生成されない
     (PAD_SET_CUR_NOSAVE が unresolved) ため、新設の
     **require-codegen-threaded.txt** に置き、build.rs が useithreads
     検出時のみ `with_require_codegen_list` を追加適用 (multi-call 可)。
     CI の non-threaded セルを壊さないための分割。
   - `Perl_SvREFCNT_dec` はこの名前自体が 5.31.x の S_→Perl_ 改名由来で
     <=5.30 の macro_bindings.rs に現れないため require 不可。
3. **shim**: §1-4 は fallback ではなく必須と確定 (macrogen は Perl_ 名の
   alias 生成機能を見送り)。S_SvREFCNT_dec が 0.1.11 で生成されるように
   なったため、手書きミラーではなく alias 形を採用:
   `#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
   pub use self::S_SvREFCNT_dec as Perl_SvREFCNT_dec;` (perl_core.rs、
   Stack_off_t 前例の隣)。perlapi_ver28 ガードは S_SvREFCNT_dec 未生成の
   5.26 以前 (次段) を除外するため。
4. **CI**: matrix に '5.28' 追加 (threaded/non-threaded 両方、計 18 セル)。
5. **付随修正**: xs-demo-mytest{,2}/tests/perl_smoke.rs に Test2::V0
   欠如時の skip ガードを追加 (prove 欠如時と同型)。Test2::Suite は
   perl 5.40 で core 入りしたため 5.38 以前の素のコンテナ perl に無く、
   multi-perl の --downstream-test が全滅していた。CI は cpanm で導入する
   ので従来どおり実行される。

検証:
- ローカル perl 5.42 (threaded): `-D warnings` で workspace + examples
  テスト全 green。macro_bindings.rs に 4 シンボル生成確認。
- multi-perl `--downstream --downstream-test` (macrogen 作業コピー =
  0.1.11 release commit): 5.28/5.30/5.32-threaded +
  5.28/5.30-non-threaded の 5 leg 全て smoke=OK downstream=OK、
  build ログ warning/error 0、各 leg 17 テストターゲット green。
  生成確認: PadlistARRAY/PadlistMAX/PAD_SET_CUR は全 leg、
  S_SvREFCNT_dec は 5.28/5.30 (alias もコンパイル済み)、
  Perl_SvREFCNT_dec は 5.32。

実走で得た知見:
- **stale apidoc-cache の罠を実地確認**: 手元の旧 5.30-threaded 成果物で
  PAD_SET_CUR が CASCADE_UNAVAILABLE (SAVECOMPPAD suppressed) だったのは
  リリース前の実験状態の cache 残留が原因。リリース 0.1.11 同梱データに
  SAVECOMPPAD 抑制は無く、cache 掃除後の再走で解消。macrogen 側
  解決サマリの「ハーネスの罠」どおり。
- downstream 生成では **non-threaded でも PAD_SET_CUR が生成された**
  (5.28/5.30 で確認)。macrogen expect の「non-threaded は不生成」は
  ハーネス単体生成での観察で、include 構成の差と思われる。require の
  threaded 分割は macrogen の enforce 区分に合わせて維持した。

macrogen 側 samples/require-partial-eval.txt との同期: PadlistARRAY/
PadlistMAX は同期済み (macrogen 0.1.11 で先行追加済み)。PAD_SET_CUR /
Perl_SvREFCNT_dec の扱いも同ファイルのコメントに記載済みのため、
追加の同期依頼は不要。

## 6. 最終確認結果 (2026-08-26)

- libperl-rs CI (run 32963180261, branch macrogen-0.1.11 = PR #23):
  **全 18 セル green** (threaded/non-threaded × 5.28〜5.42 + '5')。
- PartialEval CI workflow_dispatch (run 32963825520,
  libperl-rs-ref=macrogen-0.1.11): **run 全体 success。
  5.28/5.30/5.32 ithreads probe を含む ithreads 全セル
  (5.28〜'5') green** — §3 の merge 前提条件を達成。
  no-ithreads 全滅と 5.20〜5.26 probe 失敗は §4 のスコープ外どおり。
- 前提修正: PartialEval の Cargo.lock が macrogen 0.1.10 を pin して
  いたため --locked の dispatch が全セル失敗 → 同リポジトリ main に
  lock bump (14ba1ef) を push して解消 (master libperl-rs の ^0.1.10
  要求も 0.1.11 を満たすため先行 bump は無害。push CI も green)。

残タスク: PR #23 の merge (green 確認済みのため merge 可能な状態)。
