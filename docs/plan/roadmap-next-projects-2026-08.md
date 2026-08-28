# 次期プロジェクト提案: libperl-rs ファミリーによる Perl プログラミング近代化

作成: 2026-08-26。`docs/plan/plan-2026-08-26.md`(ChatGPT 相談記録)を出発点に、
現有 3 リポジトリ(libperl-rs / perl-LibPerlRs-PartialEval / perl-optree-analyzer)の
資産棚卸しを行った上での候補評価と推奨ロードマップ。

ChatGPT 案の優先順位は「①Semantic Inspector → ②OP tracer/debugger →
③LSP semantic backend → ④値認識 fuzz/PBT → ⑤実行時支援型推論 →
⑥refactoring engine → ⑦time-travel debugger」だった。本文書はこれを
**現有資産の実態で更新**し、実行可能な形に落とす。

## 0. 結論(先に要点)

- **第 1 弾 = `LibPerlRs::Inspect`**(Semantic Inspector)。ただし新規開発ではなく、
  **perl-optree-analyzer の改名・改組・拡張**として。同リポジトリは M0〜M6 完了・
  未公開・「名前は仮称」と自己申告しており、ChatGPT 第 1 位の中身が約 6 割
  完成した状態で眠っている。
- 並走する **foundation track = libperl-rs Step 2 の完了**(docs/plan/README.md §4)。
  ただし先行作り込みではなく、下流 3 か所に重複する raw 層からの「抽出」で行う。
- 第 2 弾 = **`LibPerlRs::EvalCapture`**(PartialEval DESIGN.md §3 に設計完備・未着手)。
- 第 3 弾 = **LSP backend モード**(`perl-inspect --serve`)。新 LSP は書かず
  既存 LSP (Perl Navigator / PLS) への供給に徹する。
- tracer/debugger と fuzz/PBT は後続。Inspect の ID モデルなしの tracer は
  「デモ」にしかならないため、順序を入れ替えない。

## 1. 調査で判明した事実 — ChatGPT の前提を更新する 3 点

### 1.1 「Semantic Inspector」は約 6 割完成している

`~/db/github/perl-optree-analyzer`(**ローカルのみ・git remote 未設定・未公開**):

- `OpTree::Analyzer::analyze($coderef)` → `{args, returns, logic, types, lints}`
  - argspec: signature / `my (...) = @_` / shift/pop / `$_[n]` の 4 段検出、
    min/max arity、パラメータ名、デフォルト式
  - retspec: return 収集(行番号・値形状)+ die/croak/confess + wantarray
  - logic: 文レベルガード条件の真理値表(≤8 原子)
  - types: フロー非依存 facet 収集 + 矛盾検出(hashref を arrayref として使用、
    行番号付き)。multideref デコード込み
  - lints: `my $x = EXPR if COND` 等
- crate 構成: analyzer-core(純 Rust: IR + passes + ミニ deparser)/
  analyzer-capture(FFI: 2 パスキャプチャ + raw.rs)/ analyzer-xs(cdylib)
- M0〜M6 完了(M6 = EUMM インストール対応 2026-07)。テスト 72 全 green、
  valgrind 0 エラー。upstream 還元も一巡済み(libperl-rs 0.4.1 / macrogen 0.1.6)
- `docs/status-2026-07-05.md` 自身が「パッケージ名は仮称」「バックログ #3 =
  capture 層の汎用 optree クレート化 → libperl-rs へ upstream」と明記
- 欠けているのは**ファイル/パッケージ単位の面**(stash walk、CLI、
  compile-and-capture)— いずれも動くコードが他所にある(§1.2, §1.3)

### 1.2 PartialEval に再利用インフラ + 未着手姉妹モジュールの完全設計

`~/db/github/perl-LibPerlRs-PartialEval`:

- 再利用可能な engine 層: `raw.rs`(op walker / COP 行マッパ / GV/CV 名前解決)、
  `compile.rs`(string→CV capture Handle + **BEGIN 帰属 = use 文抽出**)、
  `frame.rs`(pad/@_ frame 構築、savestack ベース teardown = longjmp 安全)、
  `callback.rs`(Rust→Perl 呼び出し橋)、`runloop.rs`(mini runloop。
  `_run_ops` XSUB = 実行 op 名列 = tracer の原型が既にある)
- `docs/DESIGN.md` §3 に **LibPerlRs::EvalCapture の完全設計(未着手)**:
  走行中アプリの string-eval CV を `PL_savebegin` + `PL_ppaddr[OP_ENTEREVAL]`
  差し替えで全捕獲、BEGIN 帰属・caller メタデータ付き
- `docs/API-REVIEW-2026-08.md` §5-8: use 抽出(`compile_info`)は Perl 界に
  類例がなく evaluator より遥かに安定 → **単体で見せるのが採用の入口**、が既定指針
- 家名憲章(DESIGN.md §8): Perl 名前空間は `LibPerlRs::*`、
  dist は EUMM + cargo ハイブリッド(XSLoader cdylib、postamble で cargo)
- ファミリー開発原則: 「先回り実装をしない。実 fixture 駆動で増分する」

### 1.3 libperl-rs 本体: Step 2 未完 + 消費者ゼロの生成済み FFI

- 本体 crate の introspection 安全層は `Cv` のみ。`Op` newtype / op-tree iterator /
  stash walker / COP アクセサ / `SvKind` は未移植(docs/plan/README.md §4 Step 2、
  north-star 例 `walker_stash.rs` / `walker_subs.rs` も未着手)。
  豊富な実装は凍結済み libperl-proto0(examples 102-110, eg/*)に存在
- 同等の raw 層が **3 か所に重複**: proto0 / analyzer-capture/raw.rs /
  partial-eval-engine/raw.rs(後者は前者の子孫)。
  → upstream の機は熟している。投機ではなく「実消費者 2+ の実績」ベース
- **生成済みだが消費者ゼロの FFI**(libperl-sys):
  `PL_runops` 差し替え(runops_proc_t / PL_runops_std/dbg)、
  indexable `static mut PL_ppaddr[MAXO]` / `PL_check`、PERLDBf_* / PL_DB* 一式、
  `Perl_eval_pv/sv`、`Perl_op_dump/sv_dump`、CopLINE/CopFILE/CopSTASH 族、
  Pad* 全族、CvGV/CvNAME_HEK/CvSTASH/CvOUTSIDE、GvLINE/GvFILE、HvNAME、
  そして `sigdb.rs`(全 1839 perl API 関数の署名 DB — LSP/codegen 素材)

## 2. 候補評価

| 候補 | 近代化価値 | CPAN 比新規性 | 資産再利用 | リスク |
|---|---|---|---|---|
| A. Semantic Inspector(optree-analyzer 発展) | 基盤: 全候補の前提 | 中〜高(optree 深度 + 機械可読 API) | **最大**(~6 割済み) | 低〜中 |
| B. OP tracer → data-history debugger | 高(体感価値大) | 高(生きた類例なし) | 中(runloop/フック原型) | **初手には高** |
| C. LibPerlRs::EvalCapture | ニッチだが唯一無二 | 高(BEGIN 帰属は新規) | 極高(設計完・技法実装済) | 低(ただし脇役) |
| D. LSP backend daemon | 天井最大 | 中(Navigator も compiler+B 使用) | A に全面依存 | 初手には高(採用) |
| E. 値認識 fuzzer / coverage-guided PBT | XS/シリアライザ作者向け | 高(差別化形態のみ) | 中〜高 | 中〜高 + 市場 |

補足:

- **B を先にしない理由**: A なしの B は op/sub/位置の安定 ID を持たない
  「デモ」になる。逆に A は単体で LSP/refactoring に即効性がある。
  B の難所はフックではなく頑健性(XS / tie/magic / goto / die-JMPENV 耐性と
  記録量 — DESIGN.md §9 は狭い PartialEval の場合ですら die 制御を未解決と記録)。
- **D を先にしない理由**: 採用は「明らかに優れた semantic payload の実演」が
  前提。payload = A の出力そのもの。
- **E の適時**: argspec/types パスがそのまま「使用法からの generator 推論」
  (ChatGPT §10)の入力になり、runloop は per-path op coverage を既に測っている。
  coverage-guided PBT の部品は A/B の後に自然に揃う。

## 3. 推奨ロードマップ: 2 トラック並走 + 順次リリース

### 3.1 Foundation track — libperl-rs 0.5 = Step 2 を「抽出」で完了

docs/plan/README.md §4 Step 2 + §3.3 の残件を、下流からの抽出で埋める:

- `Op` newtype + `OpNextIter` / `OpSiblingIter`(analyzer-capture/raw.rs と
  partial-eval-engine/raw.rs から。`#[cfg(perlapi_ver26)]` の sibling 分岐込み)
- COP file/line アクセサ(CopLINE は require-codegen 済み・未消費)
- GV/CV 名前解決(CvGV / GvFILE / GvLINE / HvNAME)
- stash walker(proto0 examples 105-107 の `StashWalker` から)
- `SvKind` enum(proto0 eg/sv0.rs から)、pad 名列挙(eg/pad0.rs + argspec から)
- `Perl::parse_only`(compile-not-run。§3.3 の LSP 向け残件)
- north-star 例 `walker_stash.rs` / `walker_subs.rs` を examples/ に

原則: **実消費者が 2 つ以上あるものだけ**を上げる(fixtures-first の維持)。
これは optree-analyzer が libperl-rs 0.4.1 還元(Cv newtype 等)で一度実証した
ワークフローの反復。

### 3.2 Product track 第 1 弾 — `LibPerlRs::Inspect`

perl-optree-analyzer を家名憲章下に改名・改組して拡張する。
仮称のまま未公開なので互換コストはゼロ。

- dist `perl-LibPerlRs-Inspect` / モジュール `LibPerlRs::Inspect`。
  EUMM + cargo ハイブリッド(install ガード・offline vendoring 含め
  optree-analyzer の機構をそのまま継承)
- crates: `inspect-core`(純 Rust: IR + passes)/ `inspect-capture`
  (Step 2 進行に伴い薄化)/ `inspect-xs`(`analyze($coderef)` API 継承)/
  **`inspect-cli`**(自前インタプリタ内蔵の `perl-inspect` バイナリ、EXE_FILES)
- CLI は「1 起動 = 使い捨てインタプリタ/プロセス」なので、
  **MULTIPLICITY(issue #20)は MVP の経路外**(in-process spawn が要るのは
  daemon 段 — DESIGN.md §2 の spawn 型)

**MVP**(`perl-inspect lib/Foo.pm` → JSON、ChatGPT §24 の 5 項目を改訂):

1. ファイルを compile-not-run(`parse_only`。BEGIN は走る — §5 の信頼モデル参照)
2. stash walk → パッケージ/sub 一覧、各 sub に**由来タグ**
   (この file で定義 / import / XS — CvFILE/GvFILE 判定。既存 LSP が誤る点)
3. CV ごと: file、行範囲(先頭/末尾 COP)、prototype/signature、pad lexicals
4. 既存 argspec パスの出力(= LSP signature help / completion の素材。
   retspec/logic/types は `--deep` の裏に温存し、既定 schema を小さく保つ)
5. `schema_version` 付き JSON(API-REVIEW G6 の教訓: schema は無料のうちに版付け)

**M2**: compile.rs の BEGIN 帰属技法をファイル compile に移植し、
**import 引数付き `use` 一覧**(`compile_info` 相当)を JSON に載せる。
CPAN に再構成できるツールがない、採用の看板機能。

### 3.3 第 2 弾(小さく速く)— `LibPerlRs::EvalCapture`

DESIGN.md §3 の実装。libperl-rs 側の前提作業がほぼ不要で、
Inspect の eval カバレッジ(「アプリが eval したコードも inspect できる」)を補完し、
`compile_info` の 3 面展開(§5)を完成させる。

### 3.4 第 3 弾 — LSP backend モード

`perl-inspect --serve`(JSON over stdio)。**新 LSP は書かない**。
既存 LSP への供給であり、schema の外部消費者(または自作の薄い統合アダプタ
1 本での実演)を得てから着手する。issue #20(MULTIPLICITY 吸収層)は
この段の foundation 作業として計画(worker をプロセスにすれば更に先送りも可)。

### 3.5 後続(fixtures 駆動)— B(tracer/debugger)→ E(fuzz/PBT)

- B は Inspect の ID モデル + PartialEval runloop の die/JMPENV 硬化の後。
  `PL_runops` / PERLDB フックは FFI 面が完成済みで安全層だけが無い状態。
- E は argspec/types → generator 推論、runloop → coverage feedback の接続。

## 4. 参照ファイル(実装時の起点)

| 対象 | パス |
|---|---|
| Step 2 の定義 | libperl-rs `docs/plan/README.md` §4 Step 2, §3.3 |
| Inspect が継承する資産一覧 | perl-optree-analyzer `docs/status-2026-07-05.md`(特にバックログ #3)|
| upstream すべき raw 層(正) | perl-optree-analyzer `crates/analyzer-capture/src/raw.rs` |
| 同(子孫) | PartialEval `crates/partial-eval-engine/src/raw.rs` |
| BEGIN 帰属 / use 抽出 | PartialEval `crates/partial-eval-engine/src/compile.rs` |
| EvalCapture 設計 / 家名憲章 / spawn 型 | PartialEval `docs/DESIGN.md` §3 / §8 / §2 |
| 採用指針(use 抽出単体公開) | PartialEval `docs/API-REVIEW-2026-08.md` §5-8 |
| proto0 の walker/pad/sv 例 | libperl-proto0 `examples/102〜110`, `examples/eg/*` |

## 4.5 実施状況 (2026-08-28 追記)

- **Foundation track (§3.1) 実装完了** — branch `step2-introspection` = **PR #24**:
  Op/Cop/Gv/PadName newtypes、SvKind、StashWalker、Cv/Hv/Perl 拡張、
  north-star 例 (walker_stash / walker_subs)、統合テスト。
  - 5.28/5.30 の Padname* 欠落は libperl-sys/src/perl_core.rs の compat 実装で
    吸収 (macrogen 次期 unskip round の解除候補。apidoc v1.14 での抑制であることを
    再確認済み)。CvGV は **5.32 だけ引数型が `*const CV`** (他は `*const SV`) —
    推論キャストで両対応 (commit 2e02518)。
  - **訂正 (2026-08-28)**: 当初「GvGP 系は 5.44 で CASCADE_UNAVAILABLE」と判断して
    Gv を gp 構造体直読みで実装したが、これは **8/22 起源の古い実験状態の
    multi-perl 成果物 (stale apidoc-cache) の見誤り**で、v0.1.11 では
    GvGP/GvCV/GvFILE/GvLINE が 5.28〜5.44 全 leg × 両モードで生成される
    (シグネチャも一様、GvCV のみ `*const GV` 引数)。前ラウンドで文書化した
    「stale apidoc-cache の罠」の再発 — **leg 成果物のシンボル有無を設計根拠に
    する前に、生成日時と apidoc 世代を確認し、疑わしければ再生成すること**。
    Gv は生成 API 消費 + require-codegen 追加に切り替え済み。
  - 検証: ローカル 5.42 `-D warnings` 全 green / multi-perl 3 leg
    (5.28-threaded, 5.28-non-threaded, 5.44-threaded) smoke=OK downstream=OK /
    **CI 18 セル全 green** (run 33144988969)。
  - `Perl::parse_only` は新設せず: 現行 `Perl::parse` が compile-only であり、
    新設の `Perl::run` との分離として明文化 (src/perl.rs doc)。
- **Product track §3.2 第 1 歩 (改名) 完了** — perl-optree-analyzer リポジトリの
  branch `libperlrs-inspect-rename` (commit 5ac1fb2) で
  `OpTree::Analyzer` → **`LibPerlRs::Inspect`** へ機械的改名
  (crates も inspect-{core,capture,xs} へ)。9 ファイル 72 テスト全 PASS。
  記録: 同リポジトリ `docs/rename-libperlrs-inspect-2026-08.md`。
- 残: PR #24 の merge (green 確認済み) と libperl-rs 0.5 リリース判断、
  Inspect 側 branch の main への merge、その後 §3.2 の capture 薄化 + inspect-cli。

## 5. 採用戦略ノート

- **use 抽出を看板に**(API-REVIEW §5-8 の既定路線): PartialEval 内での
  POD 独立化 + Inspect M2 + EvalCapture の 3 面展開。
  アナウンスは「string-eval されたコードが何を `use` しているか正確に見える」から。
- **live-mode の信頼モデルは `perl -c` と同一**と正直に位置づける
  (Perl Navigator も同じモデル)。緩和は (a) 使い捨て worker プロセス
  (CLI 形状で自動的に成立)、(b) 既定は最小 `@INC`・プロジェクト環境は
  明示 `--live`、の 2 段。Safe.pm 型 ops 制限サンドボックスは v1 では
  作らない(既知の泥沼)— 制限として文書化する。
- **配布は「利用者の perl に対してソースビルド」一択**(libperl-sys が
  build 時に 1 つの perl に束縛される性質上、バイナリ配布は不可能)。
  EUMM + cargo テンプレが既に解決済み。ビルド依存(rust toolchain /
  libclang / perl-devel)と offline vendoring レシピを README で目立たせる。
  perl 下限はファミリー CI の実証範囲(5.42+ required, 5.28+ probe)から
  出発し、後から広げる(下限拡大をリリースゲートにしない)。
