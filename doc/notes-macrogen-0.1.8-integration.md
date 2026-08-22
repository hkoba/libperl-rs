# libperl-macrogen 0.1.8 統合メモ (16-partial-eval-support 向け)

macrogen 側のセッション (2026-08-22、multi-perl テスト基盤 + Phase 9 green 化 +
5.44 対応 + 0.1.8 リリース) からの引き継ぎ。macrogen リポジトリは
`~/db/github/libperl-macrogen`。変更の全容は `git log v0.1.7..v0.1.8` と
macrogen 側 PR #8〜#13、詳細ドキュメントは
`doc/multi-perl-testing.md` / `doc/architecture-apidoc-patches.md` を参照。

## 0.1.8 で何が変わったか (下流視点の要約)

- **旧 perl で消えていた API が復活** (apidoc data 1.2 → 1.12):
  - Cv 一族 6 (CvFILE/CvISXSUB/CvPADLIST/CvROOT/CvSTART/CvSTASH) + HvFILL
    — 5.20〜5.32 (add_decl: MUTABLE_*/AvARRAY/AvFILLp の宣言補充。issue #5)
  - OP_CLASS / OP_DESC / OP_NAME — 〜5.36 (Xop*/Bhk* の token 注釈補完)
  - newSVpvs / *pvs 一族 (+24〜26 関数/leg) — 〜5.34 (文字列リテラル連結
    `("" s "")` のパーサ対応)
  - CvDEPTH — 5.32/5.34 (embed.fnc エントリの帰属補正)
  - PadARRAY/PadMAX/Padlist*/Padname* — <=5.30 (pad.h apidoc の `*` 抜け補正)
- **perl 5.44 対応** (apidoc/v5.44.json 同梱、macrogen issue #6 クローズ):
  sv.h マクロ改名 (`_SV_HEAD` → `SV_HEAD_` 等)、hv_common_key_len の
  inline 化、5.44 新規 apidoc_defn の hv_stores 誤記 (`U32 flags` → 実体は
  `SV *val`) などはすべて macrogen 側で吸収済み
- **`--require-codegen-list` 新設**: 必須 API が生成されなければ理由付きで
  exit 1 (下流採用は本ブランチの課題 — 後述)
- macrogen 側で **5.20〜5.44 × 両モードの smoke + 下流 libperl-sys ビルドを
  publish 前検証済み** (threaded 6 legs: 5.20/5.22/5.30/5.32/5.34/5.44 が
  downstream green。ただし branch は 12-macrogen-2 でのビルド検証)

## 統合チェックリスト

### 1. 依存 bump (罠あり)

`libperl-sys/Cargo.toml` の `libperl-macrogen = "0.1.7"` → `"0.1.8"`。

**罠**: Cargo.lock が旧版を pin していると semver 互換でも旧版が使われ続ける
(`[patch.crates-io]` 検証時は `[[patch.unused]]` として静かに無視される)。
必ず `cargo update -p libperl-macrogen` を実行し、`cargo tree -p libperl-sys |
grep libperl-macrogen` で 0.1.8 が解決されていることを確認する。

### 2. skip リストの再評価 (重要 — 縮小余地が大きい)

macrogen 側で確立した手順: **リストを空にして実走 → まだ壊れるものだけ
理由付きで戻す** (macrogen CLAUDE.md「skip_codegen 運用ポリシー」参照)。

- `libperl-sys/skip-codegen-legacy.txt` (< 5.38 のみ適用):
  - `PadARRAY` / `PadMAX` / `PAD_SET_CUR_NOSAVE` — **0.1.8 で修正済み、
    外せる見込み大** (todo-2026-08-19.md の予想どおり)
  - `LOCK_LC_NUMERIC_STANDARD` / `SET_NUMERIC_STANDARD` — 空マクロ消去後の
    文/式形状問題。0.1.8 でも rustc warning (`remove this semicolon`) として
    残存を確認しているので**まだ必要な可能性が高い**
  - `MgPV` — 要実走確認
- `libperl-sys/skip-codegen.txt` (全版): 「build エラーから機械生成」の
  古いリストで、**0.1.8 で復活済みの API が多数残っている**
  (確認済みの例: `OP_DESC` / `OP_NAME` / `MUTABLE_SV` / `hv_exists` 系)。
  これらを skip したままだと partial eval で使える API を自分で塞ぐことに
  なるため、全面再評価を推奨

再評価の実走には macrogen 側のラッパが使える (checkout を汚さない):

```console
$ cd ~/db/github/libperl-macrogen
$ scripts/multi-perl.tcl --downstream --libperl-rs ~/db/github/libperl-rs \
    --branch 16-partial-eval-support 5.30-threaded
```

(--libperl-rs 指定時も tar で scratch copy を作るので working tree は不変。
 ただし --branch は clone 時のみ意味を持つ点に注意 — ローカル checkout は
 「今 checkout されている内容」がそのまま使われる)

### 3. `with_require_codegen_list()` の採用 (partial eval の保証)

macrogen 0.1.8 の Pipeline API に `PipelineBuilder::with_require_codegen_list(path)`
が追加された。partial eval 必須 API (`CvFILE/CvROOT/CvSTART/CvPADLIST/CvISXSUB`
— macrogen 側 `samples/require-partial-eval.txt` と同一セット) をリストにして
build.rs に渡すと、**生成されなかった場合に rustc の E0425 より手前で
「なぜ生成されなかったか」の理由付き fail-fast** になる。

- 出力自体は最後まで書き切られるので診断に使える
- 同名が skip リストにも載っていると起動時に即エラー (設定矛盾検出)
- CLI では `--require-codegen-list <FILE>` (詳細: macrogen
  `doc/reference-cli-usage.md`)

### 4. build.rs の再生成ガードの改善 (macrogen 検証中に 2 回踏んだ罠)

`libperl-sys/build.rs` は「`bindings.rs` が存在し `wrapper.h` より新しければ
bindgen + macrogen 生成を丸ごとスキップ」する mtime ガードを持つ。このため
**macrogen 側だけを更新しても `macro_bindings.rs` が再生成されない** stale が
起きる (macrogen 側の検証では target dir 削除で回避していた)。

推奨: 再生成条件に macrogen の変化を含める。例えば
`option_env!("CARGO_PKG_VERSION")` ではなく、build-dependencies の
macrogen バージョン (`libperl_macrogen::apidoc_data::APIDOC_DATA_VERSION` が
使いやすい — 0.1.8 で "1.12") をスタンプファイルに書いて比較する、等。

### 5. CI: `'5'` (= 5.44) セルの復帰 (libperl-rs#16)

macrogen 0.1.8 が 5.44 の apidoc データを同梱したので、rust.yml の matrix
から一時除外していた `'5'` セルを戻せる。macrogen 側では 5.44-threaded の
libperl-sys ビルド green を確認済み (ただし 12-macrogen-2 ブランチ)。

ついで: actions-setup-perl 等のビルド済み perl は `$Config{incpth}` に
ビルド時 gcc のパスを埋め込むため、新しい runner では preprocess が
stddef.h 不明で失敗することがある。その場合は
`-I "$(cc -print-file-name=include)"` を補う (macrogen の `--auto` は
-I と併用可能になっている。build.rs の `cc_system_includes()` 相当)。

## 意図的に生成されないもの (0.1.8 の既知の除外)

macrogen 側 `apidoc/v5.XX.patches.json` に理由付きで登録済み。下流で
これらに依存しないよう注意 (詳細理由は各ファイルの `reason` 参照):

- v5.20 (25) / v5.22 (19): Perl_atof (上流 aTHX_ 抜け、5.36 修正) /
  旧ハッシュ実装 inline 群 S_perl_hash_* / bool・cast 残渣クラス /
  5.20 固有の PAD_COMPNAME_* 系
- v5.32 (18) / v5.34 (23): Perl_atof / SvPV* 系の戻り値キャスト欠落 /
  utf8 検証系の bool 残渣 / CopLINE 系 (5.34)
- 非 threaded 全版: samples/bindings.rs (threaded スナップショット) 由来で
  PL_* interpreter 変数が未解決 — **下流の per-version bindgen では
  発生しない** (macrogen のスモーク側の制限)

## 統合結果 (2026-08-22 追記、本ブランチで実施済み)

チェックリスト 1〜5 は全て完了。検証は multi-perl.tcl --downstream
(threaded 5.30/5.32/5.34/5.36/5.38/5.40/5.44) + ローカル 5.42
(`RUSTFLAGS="-D warnings"` で workspace + examples テスト全 green)。
非 threaded はコンテナ未実走 (CI に委ねる)。

- **依存 bump**: 0.1.8 へ。Cargo.lock 更新・`cargo tree` で解決確認済み
- **再生成ガード** (§4): `APIDOC_DATA_VERSION` を OUT_DIR の
  `macrogen-apidoc-version.txt` にスタンプし、不一致で再生成を強制
- **require-codegen.txt 新設** (§3): 必須 5 API を登録。5.30 leg でも
  全 5 API の生成をログで確認 (fail-fast ガード実効)
- **skip-codegen.txt**: 空にして実走 → 78 → 45 エントリ (33 削除)。
  エラー種別ごとに理由コメント付きで再構成
- **skip-codegen-legacy.txt**: PadARRAY / PadMAX / PAD_SET_CUR_NOSAVE は
  予想どおり 0.1.8 で解消し削除。LOCK_LC_NUMERIC_STANDARD /
  SET_NUMERIC_STANDARD (5.30-5.34 warning) と MgPV (5.34/5.36 out-param
  warning) は残留。0.1.8 で新たに生成されるようになった
  Perl_is_c9strict_utf8_string_loclen / sbox32_seed_state96 /
  zaphod32_seed_state / Perl_is_utf8_invariant_string_loc を追加
- **skip-codegen-pre42.txt 新設** (< 5.42 適用、build.rs にゲート追加):
  Perl_newSV_type (5.36-5.40 E0308) と Perl_isC9_STRICT_UTF8_CHAR
  (5.32-5.40 E0308/E0384) は 5.38/5.40 でも壊れるが 5.42+ では正常な
  ため、legacy (< 5.38) でも全版 skip でもなく専用リストに
- **CI**: `'5'` (5.44) セル復帰。5.44-threaded はコンテナで
  workspace テストまで green を確認
- **ついでの修正**: xs-demo-mytest{,2}/tests/perl_smoke.rs が
  CARGO_TARGET_DIR を無視して `<workspace>/target` 直書きで cdylib を
  探していたのを修正 (コンテナ実走はここで落ちていた。CI は
  CARGO_TARGET_DIR 未設定なので露見していなかった)

### 実走で得た知見 (次回の再評価向け)

- multi-perl.tcl のコンテナ内 `cargo build` は `-D warnings` **なし**。
  CI (actions-rust-lang/setup-rust-toolchain) は `-D warnings` ありなので、
  skip 判定はログの error だけでなく **warning も数える** こと
- `--libperl-rs` の scratch copy は plain tar なので**未コミット変更も
  含まれる** (skip リストを編集しながらの反復に使える)
- 型エラーが全部消えると後段の lint pass (unused_must_use /
  path_statements 等) が初めて発火する。1 回 green にしてから
  もう 1 周すること (今回 Perl_cx_popformat / STR_WITH_LEN がこれ)

### macrogen 側へのフィードバック候補 (未報告)

- **OP_DESC / OP_NAME は 5.42/5.44 でも不成立** (戻り値 *mut c_char に
  対し PL_op_desc/PL_op_name 経路が *const)。本メモ冒頭の「0.1.8 で
  復活」は旧 perl の token 注釈補完の話で、新しい perl では依然
  skip が必要だった
- Perl_isC9_STRICT_UTF8_CHAR: 5.36-5.40 で E0384 (非 mut 変数への再代入)
  — ループ内再代入される局所変数の mut 化は generator 側で対応可能そう
- 5.44 で `[apidoc-patches] return_type_override MISS` warning が 4 件
  (RCPV_LEN / RCPV_REFCNT_inc / RCPV_REFCOUNT / RCPV_REFCNT_dec —
  apidoc dict に target が居ない)

## perl 上流への報告候補 (未報告)

- **5.44 hv.h の `=for apidoc_defn Am|SV**|hv_stores|HV *hv|"name"|U32 flags`**:
  第 3 引数の実体は `SV *val` (hv_deletes/hv_name_sets からのコピペ誤記と
  推定)。macrogen は v5.44.patches.json で補正済みだが上流修正が本筋
- (参考) <=5.34 の `Perl_atof` aTHX_ 抜けと <=5.30 の pad.h `*` 抜け群は
  上流では修正済みのため報告不要
