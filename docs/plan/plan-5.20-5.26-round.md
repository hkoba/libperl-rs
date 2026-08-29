# 引継: perl 5.20〜5.26 対応ラウンド (libperl-sys 生成の最終段)

作成: 2026-08-29 (perl-LibPerlRs-PartialEval セッションからの依頼指示書)。
下流 = perl-LibPerlRs-PartialEval (GH-16 の部分評価枠組み) の対応 perl
拡大ラウンド第 3 段。前例:

- 第 1 段 (5.34〜5.44): macrogen 0.1.10 反映、PR #22
- 第 2 段 (5.28〜5.32): macrogen 0.1.11 反映、PR #23、
  `plan-5.28-5.32-round.md` (本書はその §5「実施結果」の書式も踏襲する)
- macrogen 側前例: `~/db/github/libperl-macrogen/doc/plan/`
  `unskip-copline-5.34-5.36.md` (0.1.10) /
  `unskip-refcnt-padlist-5.28-5.30.md` (0.1.11 — **§0' の「ハーネスの罠」
  は本ラウンドでも必読**)

現況 (2026-08-29): master には GH-20 MULTIPLICITY 吸収層 (PR #27) +
GH-21 perl-identity stamp (PR #26) + cc_system_includes の C ロケール固定
(2d7ed96) まで入っており、下流 PartialEval は CI run 33248242079 で
**5.28〜5(latest) × threaded/non-threaded の全セル green**。残る赤は
**5.20/5.22/5.24/5.26 × 両モードの 8 probe セル**のみ。いずれも
libperl-sys の段階 (build.rs の require-codegen fail-fast、または
macro_bindings.rs のコンパイル) で落ちており、下流 engine のコードには
未到達。エラー内容は threaded/non-threaded で同一。

## 0. 失敗の棚卸し (PartialEval CI run 33248242079 実測)

版ごとに失敗クラスが違う。降順 (= 着手推奨順) に:

### 5.26 — require-codegen fail-fast (stale auto-skip の見込み)

```
RequireViolation PadlistARRAY: CODEGEN_SUPPRESSED (apidoc patch):
    type-check fails on perl 5.26 (auto-generated)
RequireViolation PadlistMAX:   同上
RequireViolation PadlistNAMES: CASCADE_UNAVAILABLE (dep: PadlistARRAY)
RequireViolation PAD_SET_CUR:  CASCADE_UNAVAILABLE (dep: PAD_SET_CUR_NOSAVE)
```

macrogen の `apidoc/v5.26.patches.json` は **auto-generated skip_codegen を
62 件**抱えており (Padlist 4 種を含む)、これは 0.1.7 時代 (78762b9,
2026-04-25) の採取。5.28/5.30 の同種 skip は 0.1.11 ラウンドで「**全部
stale — unskip するだけで green**」だったので (unskip-refcnt §0'-1)、
5.26 も同じ期待値で再評価から入るのが最短。require-codegen.txt の
コメントにも「5.26 以前は未解除 = 次段」と予告済み。

### 5.24 — macro_bindings.rs コンパイルエラー 44 件 (skip 採取漏れ)

E0308 (mismatched types) / E0277 (cannot add `u8` to `u32`) が
macro_bindings.rs の**行 586〜750 の先頭領域** = hv_func.h のハッシュ関数
群 (murmur 等) に集中。決定的な事実: **`apidoc/v5.24.patches.json` には
`S_perl_hash_*` 一族の skip が無い** (RCPV_* 4 件のみ) のに対し、
v5.20 / v5.22 の patches.json には同一族の skip が入っている (だから
5.20/5.22 ではこのクラスが出ない)。= 5.24 だけ auto 採取から漏れている。
`tools/build-error-to-vpatches.pl` の再走が最短ルート
(patches.json 冒頭コメントの案内どおり)。

### 5.22 — E0067 が 2 件だけ (gp_flags bitfield への代入)

```
{ (* GvGP (gv)) . gp_flags () &= ! 1 as u32 ; ... }   // 行 17712
{ (* GvGP (gv)) . gp_flags () |= 1 as u32 ; ... }     // 行 17721
```

この版では `gp` 構造体の `gp_flags` が bindgen の bitfield アクセサ
(メソッド) になり、生成体が lvalue にならない。GVf_INTRO (=1) の
on/off マクロ (GvINTRO_on / GvINTRO_off 相当) の生成体 2 つ。下流に
利用者は居ないので skip 追加が最短 (bitfield 書き込みの生成は新機能に
なるため今回は見送りが妥当 — 正確なマクロ名は生成ログで確定させる)。

### 5.20 — require-codegen fail-fast (正当な不在)

```
RequireViolation OpSIBLING: not a codegen target
    (no apidoc declaration or not defined in headers)
```

`OpSIBLING` マクロは perl 5.21.2 (= 5.22) 新設で、**5.20 のヘッダには
存在しない正当な不在** (5.20 は `op->op_sibling` 直読みの世界)。
require-codegen.txt の Step 2 節に無条件で載っているのが直接原因。
対応は libperl-rs 側 (§2-2)。なおこの向こうには旧観測
(run 32679249770, 2026-08-24) の「unused_parens ×3 が -D warnings で
error 化」が残っている可能性がある — require を越えた後に再確認。

## 1. 前提: macrogen 側の先行タスク (→ 0.1.12)

unskip-refcnt-padlist-5.28-5.30.md と同型の
「stale skip 再評価 → 必要な分だけ型修正/skip 補充 → リリース」:

1. **v5.26.patches.json 62 件の再評価**: 最低限 `PadlistARRAY` /
   `PadlistMAX` / `PadlistNAMESARRAY` / `PadlistNAMESMAX` と、
   `PAD_SET_CUR_NOSAVE` の生成を塞いでいる分。5.28/5.30 の前例では
   unskip だけで green だった。解除できたものは expect overlay
   (`scripts/multi-perl-expect/v5.26.json`) の `must_generate_extra` に
   登録して再退行防止 (threaded 限定の PAD_SET_CUR は threaded 節へ)。
2. **v5.24: S_perl_hash_* 一族の skip 補充** (v5.20/v5.22 と同套)。
   `tools/build-error-to-vpatches.pl` 再走で採取するのが早い。型推論の
   根治 (整数リテラルの u8/u32) はコスト次第で見送り可。
3. **v5.22: gp_flags bitfield 書き込み 2 件の skip 追加**。
4. **v5.20: 必須作業なし見込み** (OpSIBLING は libperl-rs 側)。ただし
   multi-perl の 5.20 leg smoke を回して、§0 末尾の unused_parens 級が
   出たら skip/修正。
5. **検証**: `scripts/multi-perl.tcl --downstream` (+ `--downstream-test`)
   に 5.20/5.22/5.24/5.26 × threaded/non-threaded の leg。expect overlay
   は v5.20〜v5.26.json が既存。**apidoc-cache / stale target の罠**
   (unskip-refcnt §0' 末尾) に注意: patches 編集の反復中は
   `rm -rf tmp/multi-perl/out/<leg>/apidoc-cache` と
   `rm -rf tmp/multi-perl/target-downstream/<leg>/debug/build/libperl-sys-*`。
6. **リリース**: apidoc data version bump + 0.1.12 (publish.tcl)。

## 2. libperl-rs 側の作業

1. **macrogen 0.1.12 bump** (PR #22 = ac2bb4b / PR #23 が前例)。
2. **OpSIBLING の 5.20 対応**: 2 段構え。
   - `src/perl_core.rs` に compat shim (Stack_off_t / Perl_SvREFCNT_dec
     alias の前例の隣):
     ```rust
     // 5.20: OpSIBLING マクロは 5.21.2 新設。旧世界の op_sibling 直読み
     // をミラーする (シグネチャは 5.22+ の生成体に揃える)
     #[cfg(not(perlapi_ver22))]
     pub unsafe fn OpSIBLING(o: *mut OP) -> *mut OP {
         (*o).op_sibling
     }
     ```
     (5.22+ の生成体は `pub unsafe fn OpSIBLING(o: *mut OP) -> *mut OP`。
     5.20 bindings の op 構造体でのフィールド名/型は要実物確認)
   - require-codegen.txt からは OpSIBLING を **perl >= 5.22 のときだけ
     require** する形へ: build.rs に `use_legacy_skip` (perl_minor 比較)
     と同じ形の分岐で、`require-codegen-since22.txt` 等の別ファイルに
     移すのが既存構造 (threaded 分割の前例) と揃う。
3. **compat 実装の cfg 下限の点検**: perl_core.rs の
   `all(perlapi_ver28, not(perlapi_ver32))` 族 —
   `S_SvREFCNT_dec as Perl_SvREFCNT_dec` alias と
   `Padnamelist{MAX,ARRAY}` / `Padname{PV,LEN,TYPE}` の手書き compat。
   **alias は 2 箇所ある**ことに注意: perl_core.rs に加えて
   lib.rs の `thx` モジュール内にも同型 alias
   (`pub use self::S_SvREFCNT_dec as Perl_SvREFCNT_dec;`) があり、
   下限を広げるときは両方揃えて変更する (下流 engine は GH-20 移行後
   `sys::thx::Perl_SvREFCNT_dec` を呼ぶので thx 側が実際の消費経路)。
   ≤5.26 で S_SvREFCNT_dec / Padname* が生成されるかを multi-perl
   成果物で確認し、生成されるなら下限を ver26/ver22/… へ広げる
   (生成されない版はこの compat 自体の対象拡大が要る)。5.26 の
   PadlistNAMES* は §1-1 の unskip 対象なので、その成否と連動。
4. **skip-codegen-legacy.txt**: macrogen 側で patches にせず libperl-sys
   側 skip で足りる軽微分 (warning 級) が出たらこちらへ。原則は
   macrogen patches 側に置く (このファイルは「-D warnings で error 化する
   warning」級の受け皿という既存区分を維持)。
5. **CI matrix 拡張**: 現在 5.28〜'5' × 両モード (18 セル)。green に
   なった版から 5.26 → 5.24 → 5.22 → 5.20 を追加。
6. xs-demo の downstream-test は Test2 欠如時 skip ガード対応済みだが、
   5.20 系コンテナ perl で別モジュール欠如が出たら同型で追随。

## 3. 検証

- 主戦場は macrogen の multi-perl ハーネス (コンテナ)。**ローカル plenv
  には 5.20〜5.26 が無い** (5.29.2/5.29.5 は 5.30 系寄りで代用不可)。
  ローカルに作る場合は `plenv install 5.26.3 -Dusethreads` 等
  (threaded は -Dusethreads 必須。cargo build/test だけなら
  libperl.a 非 PIC でも足りる — cdylib を積む下流 .t は不可)。
- perl 切替の stale bindings 問題 (issue #21) は perl-identity stamp
  (cc00a5e) で解消済み。macrogen ハーネス側のキャッシュ罠は §1-5。
- 非英語ロケール環境で古い perl をローカルビルドして使う場合の
  cc_system_includes は 2d7ed96 (LC_ALL=C 固定) で対応済み。この修正が
  効くのは「perl の incpth が指す gcc ディレクトリが消えている」ケース
  (ビルド後に gcc がメジャー更新された古い plenv perl) — まさに本
  ラウンドでローカル perl を作り置きすると将来踏む形なので記憶しておく。

## 4. 下流 (PartialEval) での最終確認

- merge 前検証の確立済み手順: PartialEval 側で
  `gh workflow run CI --ref <branch> -f libperl-rs-ref=<本リポジトリの作業ブランチ>`
  → 5.26〜5.20 probe セルの結果を見る。
- **libperl-sys が通った先で、下流 engine 側の問題が新たに露出する見込み**
  (そこから先は PartialEval 側セッションの担当):
  - ≤5.24 では raw.rs の `op_tree_sibling` が `cfg(not(perlapi_ver26))`
    の op_sibling 直読み経路に**初めて**入る
  - opcode の版 cfg (OP_MULTIDEREF = ver22 等) はコンパイルは通る想定、
    ランタイムは未知
  - **教訓 (2026-08-29, no-ithreads ラウンド実測)**: 盲書きの cfg 分岐は
    「コンパイルが通っても初実行で踏み抜く」。anoncode の proto 取得は
    非 threaded でも `PAD_SV(op_targ)` が正 (op_sv 読みは誤り) で、
    t/45 の 1 サブテストでだけ発覚した。≤5.26 も **Build green で
    満足せず必ず Test (193 件) まで見る**こと。
- 完了条件: PartialEval の probe セル green が 5.26 → 5.24 → 5.22 →
  5.20 と拡大すること。全緑になったら probe (continue-on-error) 指定の
  昇格も検討。

## 5. スコープ / 優先順位

- ユーザー決定 (2026-08-24): サポート目標は**段階的に 5.20+ まで**。
  1 版ずつ降順 (5.26 から) が現実的 — 5.26 = unskip 主体で最短、
  5.24 = 採取漏れ補充、5.22 = 2 件、5.20 = OpSIBLING + α。
  途中版まででも green になった分だけ CI に反映して切り上げ可能な構成
  にする (1 版 1 PR でも良い)。
- Perl_ 名 alias の macrogen 生成機能は引き続き見送り
  (unskip-refcnt §0'-2 の判断を維持。compat は libperl-rs 側 shim で)。
- ≤5.18 は対象外。docs.rs / crates.io publish はこのラウンドの必須では
  ない (下流は path 依存 + sibling checkout)。
