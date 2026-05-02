# libperl-rs 再構築プラン

本ドキュメントは、libperl-macrogen を前提とした libperl-rs の再構築計画を
まとめたものです。会話で合意した方針を後から見返せるように書き出しています。

## 構成

- `README.md` (本ファイル) — プロジェクト目標、現状、設計判断、ロードマップ
- `appendix-rust-proc-macro-limits.md` — `aTHX_!()` / `pTHX_!()` を
  Rust proc-macro で実現可能か検証した結果と、そこから導かれる API 形

---

## 1. 背景と最終目標

libperl-rs は Perl 5 の内部 API (libperl) を Rust から利用できるようにする
ためのライブラリ。歴代の構成は

```
libperl (C) ── bindgen ──> libperl-sys ── 手書きラッパ ──> libperl-rs
```

で、Perl 内部の C マクロ群を Rust 側で **手書きで再実装** する負担が大きく、
研究段階に留まっていた。

その課題を解くために libperl-macrogen を別途開発し、Perl の C マクロおよび
inline 関数を Rust の `unsafe fn` に **自動変換** できるようになった。
これにより、これまで `examples/eg/sv0.rs` などで手書きしていた `SvTYPE`,
`HvNAME`, `CvSTART`, `HEK_KEY` といった macro 群の大半が macrogen 出力で
代替できるようになっている (確認済み: 1684 個生成、73 個 skip-codegen 対象)。

これを土台に、**過去の API 互換は捨てて理想形を作り直す**。想定される上位
アプリケーションは:

- Perl の実際の内部状態を利用した language server
- コード誤り (静的) 検出器
- Perl の XS を Rust で書くための支援

## 2. 現状の把握 (2026-04 時点)

- `libperl-sys/build.rs` に bindgen + libperl-macrogen の両方が組み込まれて
  おり、`OUT_DIR/macro_bindings.rs` が **macrogen から自動生成** されている。
- bindings.rs から関数シグネチャ辞書 `sigdb.rs` も生成済み。LSP/コードジェン
  系の素材として利用可能。
- ただし `src/perl.rs` 270 行目あたりに `SvTRUE` のバグがあり、**現時点で
  トップレベル `cargo build` は失敗** する (E0599: `not_null` メソッド無し、
  E0317: else 欠落)。`SvTRUE` は実装途中で放置された形。
- 旧 API の hand-written ラッパ (`perl_api!`/`unsafe_perl_api!` macro、
  `Perl` 構造体、`examples/eg/*.rs`) が残っている。これらは macrogen 出力と
  機能が重複している部分が多い。

## 3. 設計判断 (確定済み)

### 3.1 threaded / non-threaded 両モードのサポートを継続

両モードを必須機能として維持する。むしろ「**もっと使いやすいマクロ**」を
用意することが課題。具体的には:

- **関数呼び出しの THX 吸収**は libperl-macrogen が既に担当 (生成された
  関数は my_perl 引数を一様に取る、または取らない)。よって従来の
  `perl_api!` / `unsafe_perl_api!` macro は役割を終える。
- **PL_* グローバル変数アクセス**は依然として両モードで形が違う:
  - threaded: `(*my_perl).I<name>`
  - non-threaded: `libperl_sys::PL_<name>`
- **mark stack 等の構造体メンバ直接操作**も同様。

### 3.2 ターゲット perl は build-time に固定

- `libperl-sys` / `libperl-macrogen` の出力は、ビルド時に決定された 1 つの
  perl 専用とする。
- これにより、生成コード内に `#[cfg(perlapi_verNN)]` のバージョンガードを
  入れる必要がない。**「対象 perl に存在しない関数/変数はそもそも emit
  しない」** で済む。
- ただし threaded/non-threaded の cfg は **ビルド時固定** ではあるが、
  **同じソースを両モードでコンパイル可能にする**ことは引き続き目標。
  (1 ソースの両ターゲットビルドは支える、バージョン差は固定)

### 3.3 PL_* アクセサは libperl-macrogen が一括生成する

- macrogen は既に `perlvar_parser.rs` で PERLVAR を読んでいる。
- 同じパス上に **Rust マクロ + メソッド** の生成を追加する:
  - `PL_main_root!()` のような **per-variable declarative macro**
    (no-arg 形、後述の通り `my_perl` が in scope であることを前提とする)
  - `Perl::pl_main_root(&self) -> *mut OP` のような **method**
  - 書き込み用 `PL_xxx_set!(val)` (必要に応じて)
- 両者を併存させる方針:
  - C コードからの移植時は `PL_xxx!()` 形がそのまま使えて移植コストが低い
  - 純粋に Rust で書くときは `perl.pl_xxx()` のメソッド形が読みやすい
- 個別マクロにすることで、**バージョン差を「定義の有無」で表現**できる
  (上述の通りターゲット固定なので結果的に cfg は不要)。
- threaded build での展開形は `(*my_perl).I<name>`、non-threaded build では
  `libperl_sys::PL_<name>` をそのまま参照する。**threaded 形の `my_perl` は
  呼び出し位置のスコープにあることを前提とする** (§3.8 の命名規約と整合)。
  この設計は C 側の `aTHX_` に倣ったもの。

### 3.4 Perl 構造体: `NonNull<PerlInterpreter>` を採用

旧 `Perl` 構造体は `pub my_perl: *mut PerlInterpreter` を保持していたが、
これは「`my_perl` は決して null ではない」という不変条件を型で表現
できない。新 API では `std::ptr::NonNull<PerlInterpreter>` を採用する。

```rust
use std::ptr::NonNull;

pub struct Perl {
    my_perl: NonNull<PerlInterpreter>,
    args: Vec<CString>,
    env: Vec<CString>,
}

impl Perl {
    pub fn new() -> Self {
        let raw = unsafe { libperl_sys::perl_alloc() };
        let my_perl = NonNull::new(raw)
            .expect("perl_alloc returned null (out of memory?)");
        unsafe { libperl_sys::perl_construct(my_perl.as_ptr()) };
        Perl { my_perl, args: Vec::new(), env: Vec::new() }
    }

    /// FFI に渡すための生ポインタ。null でないことが型で保証されている。
    #[inline]
    pub fn as_ptr(&self) -> *mut PerlInterpreter { self.my_perl.as_ptr() }
}

impl Drop for Perl {
    fn drop(&mut self) {
        unsafe { libperl_sys::perl_destruct(self.my_perl.as_ptr()) };
    }
}
```

採用理由:

- `NonNull<T>` は **「null でない `*mut T`」という不変条件のみ**を型に
  押し込み、エイリアシング・可変性は raw pointer のままにする。Perl API
  のように同一 interpreter を多重に触る用途と相性が良い。
- サイズ・ABI は `*mut T` と同じ。FFI 越しの取り回しコストは無し。
- `&mut PerlInterpreter` を保持する案は、(a) 排他借用が API 全体と衝突、
  (b) 自己参照構造体になり lifetime を素直に書けない、という二重の理由
  で却下。
- `NonNull<T>` は自動で `!Send !Sync`。1 interpreter = 1 thread の Perl
  慣習に合致するためデフォルト維持。**`unsafe impl Send/Sync` は今は付け
  ない**。
- `perl_alloc` 失敗は OOM のみで稀なため `new()` で panic を許容。
  必要なら `try_new() -> Result<Self, PerlAllocError>` を別途用意する。

メンバ読み出し用に `interp(&self) -> &PerlInterpreter` を内部 method として
持ち (`unsafe { self.my_perl.as_ref() }`)、`PL_xxx!()` macro と method
両方の実装基盤に使う。借用は式の評価が終わるまでで、FFI 呼び出しを跨いで
保持しないこと。

### 3.5 `aTHX_` / `pTHX_` の C 完全互換は不可能

検証結果は `appendix-rust-proc-macro-limits.md` に記録。要点:

- Rust の関数パラメータ位置は **マクロ展開の対象ではあるが、展開結果は
  完結したパターン 1 個でなければならない**。「`my_perl: *mut T ,` という
  断片を引数列の途中に挿入する」用途は declarative でも proc-macro でも不可。
- 関数呼び出し引数位置も同様で、「式 + カンマ + 別トークン」を返すと
  `macro expansion ignores , and any tokens following` エラーになる。
- これは Rust の構文規則で、proc-macro でも回避できない。

### 3.6 代替: attribute proc-macro と呼び出し全体ラップマクロ

`aTHX_` / `pTHX_` の役割は、以下の 2 つで代替する:

| 旧 C パターン | Rust での代替 |
|--------------|---------------|
| `pTHX_` (関数定義のシグネチャ) | **attribute proc-macro `#[thx]`** が関数 item 全体を書き換え。threaded なら `my_perl: *mut PerlInterpreter` を引数に追加する。本体冒頭への束縛注入は行わない (§3.8 を参照) |
| `aTHX_` (関数呼び出しの引数) | **declarative macro `perl_call!(F(args...))`** が呼び出し全体をくるみ、threaded/non-threaded で適切な形を出力する。`my_perl` は in scope 前提 |
| `pTHX` を持つ通常の Rust helper | 素直に `fn helper(perl: &Perl, ...)` と書く (Rust では C より自然) |

`#[thx]` および `#[xs_sub]` は新クレート `libperl-macros` に置く
(`proc-macro = true`)。

#### 3.6.1 命名: なぜ `#[thx]` か (旧称 `#[xs_sub]` から変更)

当初は `#[xs_sub]` という名前を想定していたが、これは「XS sub のための
もの」という限定された読み方を強いる。Perl API は XS のためだけでなく、
通常の helper 関数 (`Perl_*` を呼ぶだけのもの) でも threaded build では
`my_perl` を差し込む必要がある。

採用した命名は **`#[thx]`**。理由:

1. **短い** — attribute は宣言冒頭に常に出るので視覚的ノイズが少ない。
2. **C と同じ語彙** — `pTHX` / `aTHX` / `dTHX` は Perl C 文化で確立した
   用語。移植してきた人にそのまま通じる。
3. **「変数名」と「修飾の概念」の分離** — `my_perl` という変数名は §3.8
   の命名規約で管理し、attribute 名は「何の概念を加えているか」を表す。
   `#[thx]` を付けると `my_perl` が出現する、という対応関係。

`#[xs_sub]` は廃止せず、**`#[thx]` の上に XS 固有の作法 (`extern "C"`、
mark stack RAII、`dXSARGS` 相当) を乗せた糖衣** として残す。Step 3 (XS
支援) で必要になってから本実装する。

```rust
// プリミティブ: 一般の THX-aware helper
#[thx]
fn my_helper(sv: *mut SV) -> i32 {
    perl_call!(Perl_sv_setiv(sv, 42));
    0
}

// 複合: XS sub (内部で #[thx] 相当の処理 + XS 固有の作法)
#[xs_sub]
fn my_xs(cv: *mut CV) {
    // ...
}
```

### 3.7 旧コードの保管: `libperl-proto0` サブクレート

旧 API 利用者の救済を目的として、現在の `src/perl.rs` および `examples/`
一式を `libperl-proto0` という workspace member に退避する。

- `publish = false` で crates.io には出さない (必要が出れば後から検討)。
- 旧 examples (`000_perl_parse.rs` 〜 `110_call_method.rs`) をそのまま温存。
- `examples/eg/*.rs` (手書きの SV/OP 抽出関数群) も同梱して proto0 内では
  完結させる。
- 積極的にメンテはしない。`libperl-sys = "=0.3.x"` のように pin して
  破綻しにくくする。

### 3.8 命名規約: THX 関連の変数名は `my_perl` で統一

Perl C API との整合性のため、以下の名前を **常に `my_perl`** で揃える:

- `#[thx]` / `#[xs_sub]` proc-macro が threaded build で注入する仮引数
  → `my_perl: *mut PerlInterpreter`
- `PL_xxx!()` macro が threaded build で参照する変数
  → `(*my_perl).I<name>` (≒ `my_perl` が呼び出しスコープに居る前提)
- `perl_call!(F(args...))` macro が threaded build で `F` の第 1 引数に
  渡す変数 → `my_perl`

ユーザコードで `Perl` 構造体から `my_perl` を取り出す典型形:

```rust
fn do_something(perl: &Perl) {
    let my_perl = perl.as_ptr();        // C の慣例どおり my_perl を導入
    let root = PL_main_root!();          // macro は my_perl が in scope 前提で展開
    perl_call!(Perl_sv_setiv(sv, 42));   // 同上
}

#[thx]
fn my_helper(sv: *mut SV) {
    // threaded build:
    //   fn my_helper(my_perl: *mut PerlInterpreter, sv: *mut SV)
    // non-threaded build:
    //   fn my_helper(sv: *mut SV)
    let root = PL_main_root!();   // どちらのモードでも書き換え不要
    perl_call!(Perl_sv_setiv(sv, 42));
}
```

non-threaded build では `my_perl` は使わないので、in scope に無くても
マクロ展開はエラーにならない (展開結果に `my_perl` が現れないため)。

### 3.9 THX 引数の型: `*mut PerlInterpreter` を維持する

「`my_perl` は決して null として呼ばれない。だから macrogen 出力でも
`my_perl: &mut PerlInterpreter` にすべきではないか?」という疑問が
あったが、**結論は否**。理由は Rust の参照型が持つエイリアシング規則が
Perl API の実態と本質的に矛盾するため。

#### 3.9.1 `&mut T` は「非 null」だけでなく「排他借用」を意味する

Rust の `&mut T` は 2 つの不変条件を一度に表現する型:

1. **non-null である**
2. **同じ T への他の参照 (mut でも immut でも) が同時に存在しない** (= 排他)

「null でない pointer が欲しい、aliasing は raw pointer のまま」という
要望には、参照型ではなく **`NonNull<T>`** が答えになる (§3.4 で `Perl`
構造体に採用したのと同じ理由)。

#### 3.9.2 Perl API は再入が前提 → `&mut` 排他性と衝突する

Perl の C API は callback だらけ:

- magic vtable (get/set/clear/free)
- tied variables の FETCH/STORE
- SV/HV の DESTROY
- signal handler
- `call_method` / `call_sv` 経由で Perl コードが Rust XS を呼び返す
- format/regex/overload の callout

`my_perl: &mut PerlInterpreter` を関数が受け取った瞬間に、その関数の
スコープ内では「**この `&mut` がインタプリタに対する唯一の生きている
参照**」というアサーションが立つ。ところが上記の callback が起きると、
callback 側にも `my_perl` が渡され、そこでも `&mut PerlInterpreter` を
作ることになる。**外側の `&mut` がスタックフレームに残ったまま、内側で
同じインタプリタへの 2 つ目の `&mut` を作る → aliasing 違反 (UB)**。

具体例:

```rust
// もし &mut にしてしまったら:
fn outer(my_perl: &mut PerlInterpreter) {
    let sv = Perl_newSV(my_perl, 0);          // ← &mut 借用 #1 (戻り値返却で終わる)
    Perl_sv_setpvn(my_perl, sv, b"hi", 2);    // ← &mut 借用 #2
    //   この呼び出し中、SET magic が走り、Rust XS callback が呼ばれる:
    //   extern "C" fn my_callback(my_perl: &mut PerlInterpreter, cv: *mut CV) {
    //       //  ← &mut 借用 #3 が、#2 と同時に存在する!
    //   }
}
```

借用検査は `unsafe` の境界を越えると強制力を失うが、**Stacked Borrows /
Tree Borrows という Rust の正式な aliasing model** では UB のまま。Miri
で実行すれば即検出されるし、将来のコンパイラ最適化が aliasing 仮定を
使って意図しない書き換えをする可能性もある。

これは callback が「実際に起きるかどうか」の問題ではなく、**型が与える
保証 (= 排他性) が API の現実と矛盾している**という、根の深い設計上の
ミスマッチ。

#### 3.9.3 `&PerlInterpreter` (shared 参照) も詰む

「mutable は強すぎるなら、shared 参照ならどう?」という案も検討したが:

- `&T` は「`T` の中身は変わらない」を意味する。`PerlInterpreter` の中身
  (`Imain_root` 等) は API 呼び出しで常に書き換わる。
- 書き換えを許すには `&UnsafeCell<PerlInterpreter>` のように **構造体側
  で interior mutability を宣言**する必要がある。
- bindgen が出力する `PerlInterpreter` は素の struct で、フィールドを
  `UnsafeCell` で包んでいない。これを書き換えるのも非現実的。

shared 参照路線は「`UnsafeCell` でラップする層を新設する」という大工事を
要し、その対価として得られるのは「ポインタが non-null であることの
コンパイル時保証」だけ — それは `NonNull<T>` で代替可能。割に合わない。

#### 3.9.4 `NonNull<PerlInterpreter>` を macrogen 出力にも使うか? → 使わない

技術的には可能で、しかも `&mut` のような副作用は持たないが、現状では
コストが上回る:

| 観点 | `*mut PerlInterpreter` (現状) | `NonNull<PerlInterpreter>` |
|------|-------------------------------|---------------------------|
| non-null の型保証 | 無し | **有り** |
| aliasing 問題 | 無し | 無し |
| bindgen 出力との一貫性 | **bindgen と同じ** | bindgen は `*mut` のまま (変えられない) → **境界で `.as_ptr()` 必要** |
| call site の見た目 | そのまま渡せる | `.as_ptr()` を頻繁に書く必要あり |
| 内部実装 (macrogen 自身) | bindgen 関数を直接呼べる | bindgen 関数を呼ぶ前に `.as_ptr()` |

特に重い問題は **bindgen 出力との混在**。macrogen 関数が `Perl_xxx`
(bindgen 出力) を内部で呼ぶケースは無数にあり、その境界で毎回
`.as_ptr()` が要る。`NonNull` を一貫して使うなら **bindgen 出力も
`NonNull` に書き換える後処理が必要**になり、それは大工事。

得られる対価は「`my_perl` の null チェックを忘れない」だけで、**実際には
ユーザは `Perl::as_ptr()` の戻り値しか持たない**ので、null 混入の経路は
ほぼゼロ。**型保証の境界を `Perl` 構造体に集中させ、その内側の FFI 層は
`*mut` 統一**、というのが綺麗な分業。

#### 3.9.5 推奨設計: 層別の責務

```
┌─────────────────────────────────────────────────────────────────┐
│ ユーザコード                                                     │
│   - 受ける: &Perl                                                │
│   - 取り出し: let my_perl = perl.as_ptr();  // *mut             │
│   - 以降は my_perl を生ポインタとして macrogen / bindgen 関数へ  │
├─────────────────────────────────────────────────────────────────┤
│ libperl-rs (safe 境界)                                           │
│   - Perl { my_perl: NonNull<PerlInterpreter>, ... }              │
│     ← non-null 不変条件はここで型保証                             │
│   - as_ptr() -> *mut PerlInterpreter で FFI 境界に降ろす          │
├─────────────────────────────────────────────────────────────────┤
│ libperl-sys (FFI 境界)                                           │
│   - bindgen 出力: *mut PerlInterpreter                           │
│   - macrogen 出力: *mut PerlInterpreter (bindgen と一貫)         │
│   - aliasing 規則は raw pointer のまま (Perl の callback 再入と  │
│     整合)                                                         │
└─────────────────────────────────────────────────────────────────┘
```

「`my_perl` が null のまま API を呼ぶのはコードの誤り」という不変条件は、
**`Perl` 構造体を経由しないと `my_perl` を入手できない**という設計で
保証される。型 (`NonNull`) ではなく **API ガード** で実現する、と言い
換えても良い。`Perl::as_ptr()` を経ずに `*mut PerlInterpreter` を作る
方法はライブラリ利用者には開かれていない (`unsafe` で意図的にやらない
限り)。

### 3.10 SV/CV/HV/AV 引数の型: `*const SV` / `*mut SV` 等を維持する

「`SvANY()`, `SvFLAGS()`, `CvANON()` のような sv_family を受け取る
macrogen 出力関数の引数は `*const SV` ではなく `&SV` の方が適切では
ないか?」という疑問について。**結論は §3.9 と同じく否、`*const SV` /
`*mut SV` を維持する**。ただし論点は §3.9 (`my_perl`) より 4 つ多い。

#### 3.10.1 「pure read マクロは大丈夫」という仮説 — 実は条件付き

`SvANY()` `SvFLAGS()` `CvANON()` `SvTYPE()` のように **構造体フィールドを
読むだけで magic を trigger しない** マクロに限定すれば、`&SV` を取らせて
も直観的には問題なさそうに見える。関数の中で構造体フィールドを読むだけ
なら、その関数の実行中に SV が変わる経路はないため。

問題は **借用が呼び出し元の文脈にまで及ぶ** こと:

```rust
fn pure_read_then_call(sv: &SV) -> i32 {
    let _flags = SvFLAGS(sv);                  // 借用 A は SvFLAGS 内で生死
    let _iv = unsafe { Perl_sv_2iv(my_perl, sv as *const _ as *mut _) };
    //   ↑ ここで GET magic 経由で Rust callback → callback 側で sv に再アクセス
    //   親フレームの &SV はまだ生きていて、callback 側でも &SV を作ると、
    //   GET magic が SV を mutate した時点で aliasing 違反 (UB)
    SvFLAGS(sv) as i32
}
```

`my_perl` ほどの再入頻度ではないにせよ、SV/CV/HV/AV も magic / DESTROY /
overload / tied のすべてが mutate 経路を持つので、**「この型は mutate
されない」と参照型で約束するのはリスクが高い**。

#### 3.10.2 lifetime の源が無い

`my_perl` には自然な lifetime 源があった (`&Perl` から `as_ptr()`)。SV は
そうではない:

- PAD から取り出した SV (`PAD_BASE_SV`) — lifetime は padlist? cv?
- スタックから取った SV (`*sp`) — lifetime はスタックの底?
- `Perl_call_method` の戻り値 — lifetime は call スコープ?
- `newSV` で作った SV — lifetime は誰?

C 側の API は **「refcount > 0 なら生存」** という refcount semantics で
管理していて、これは Rust の lexical lifetime とは合わない世界観。`&SV`
を引数にすると、**呼び出し側で借用を組み立てるための lifetime を毎回
でっち上げる必要がある**:

```rust
let sv: *mut SV = unsafe { *sp };         // raw pointer
let sv_ref: &SV = unsafe { &*sv };        // ← この借用の lifetime は何?
```

`my_perl` は呼び出しチェーンを通じて流れる「ただ 1 つのアンカー」だった
ので `&Perl` 経由で扱えた。SV は **個別のオブジェクトが大量に流れる**
ので、安全境界を `&SV` に置くと毎回のヒモ付けコストが嵩む。

#### 3.10.3 bindgen 出力との混在 (§3.9.4 と同じ問題、影響はもっと大きい)

`my_perl` のときと同じ理屈で、macrogen 関数が内部で呼ぶ bindgen 関数
(`Perl_sv_*`, `Perl_hv_*` 等) は **大半が `*mut SV` を取る**。`my_perl`
は引数 1 個だったが、SV/CV/HV/AV は **macrogen 関数の主役の引数**なので、
`.as_ptr()` ノイズが質的に多い。

```rust
// macrogen 関数を &SV 化した場合のイメージ
pub unsafe fn SvAMAGIC_off(sv: &SV) {
    Perl_sv_setpv(my_perl, sv as *const _ as *mut _, /*...*/);
    //                     ~~~~~~~~~~~~~~~~~~~~~~~~~ ← 毎回これが要る
}
```

`*const _ as *mut _` のような const→mut の cast は、`unsafe` でも本来は
provenance 的に怪しい操作 (Rust の最近の議論)。

#### 3.10.4 refcount 観点で `&SV` は表現力不足

Perl の SV は **REFCNT で生存管理**されている value。本格的な safe wrapper
を作るなら、本来欲しいのは:

- **`Sv`** (所有、Drop で `SvREFCNT_dec`)
- **`SvRef<'a>`** (借用、Drop しない)
- **`SvWeak`** (weak ref、`sv_weaken` 経由)
- **`Mortal<'a>`** (mortal stack 上、Drop しない)

これらの区別は `&SV` 1 つでは表現できない。逆に `&SV` を採ると **「refcount
は誰が管理しているのか?」** が型から消えてしまい、後で safe wrapper を
載せるときに手戻りが起きやすい。

#### 3.10.5 推奨設計: 安全境界は newtype 層に置く

§3.9 の `my_perl` と同じ層別責務を SV-family にも適用する。安全境界は
**libperl-rs の newtype 層** (Step 2 で実装):

```rust
// 安全境界はここ
pub struct Sv(NonNull<libperl_sys::SV>);   // §3.4 と同じ NonNull パターン
pub struct Cv(NonNull<libperl_sys::CV>);
pub struct Hv(NonNull<libperl_sys::HV>);
pub struct Av(NonNull<libperl_sys::AV>);

impl Sv {
    #[inline] pub fn as_ptr(&self) -> *mut libperl_sys::SV { self.0.as_ptr() }

    /// pure-read accessor。借用は &self (newtype の borrow) で表現。
    /// 内部の SV 構造体への参照ではないので aliasing 問題は出ない。
    #[inline]
    pub fn flags(&self) -> u32 {
        unsafe { libperl_sys::SvFLAGS(self.as_ptr()) }
    }
}
```

`&self` が借りているのは **`Sv` newtype** (= `NonNull<SV>`) であって、**SV
構造体自身ではない**ことが肝心。`NonNull<T>` は aliasing 規則を持たない
ので、複数の `&Sv` が同じ underlying SV を指していても問題なし。Magic
callback で内部が書き換わっても、newtype 自体は何の保証もしていないので
UB にならない。

#### 3.10.6 §3.9 との対応関係

| 抽象境界 | `my_perl` (§3.9) | SV/CV/HV/AV (§3.10) |
|---------|-----------------|---------------------|
| 安全境界 (libperl-rs) | `Perl { my_perl: NonNull<PerlInterpreter> }` | `Sv(NonNull<SV>)` `Cv(NonNull<CV>)` 等 |
| 取り出し API | `perl.as_ptr() -> *mut PerlInterpreter` | `sv.as_ptr() -> *mut SV` |
| FFI 境界 (libperl-sys) | `*mut PerlInterpreter` | `*const SV` / `*mut SV` |
| 主な aliasing リスク | callback 再入 | callback 再入 + magic + DESTROY + overload |
| 余分な論点 | — | lifetime 源不明、refcount semantics |

**共通原則**: FFI 層 (macrogen 出力) を参照型で締め上げる方向には進まず、
**安全境界 (Perl 構造体 / Sv newtype 等)** に責務を集中させる。

### 3.11 XS 支援の 3 層分業

実験リポジトリ <https://github.com/hkoba/exp-libperl-rs-xs1> で、生の
libperl-sys を使って Rust から XS sub を書く実験が行われた。1 個の sub
(`Mytest::is_even`) を完全手書きしたコードを参照すると、機械化すべき
boilerplate と、設計上の分業が見えてくる。

#### 3.11.1 XS sub は本質的に 3 層構造

実験コードでは 1 個の sub に対して以下 3 つを手書きしている:

1. **boot 関数** (`boot_Mytest`) — モジュール初期化。`Perl_newXS_deffile`
   で各 sub を登録、`Perl_xs_boot_epilog` で締める。`extern "C"`
   `#[unsafe(no_mangle)]`。
2. **C trampoline** (`is_even_C`) — Perl から呼ばれる入口。
   `extern "C" fn(*mut PerlInterpreter, *mut CV)` シグネチャ。null
   チェックして body へ dispatch。
3. **body** (`is_even`) — 本体ロジック。引数取り出し / 戻り値 push を
   手作業で書く。

`#[xs_sub]` で機械化すべきは **(2) と (3) の関係**。(1) は別の仕掛けが
必要 (§3.11.4)。

#### 3.11.2 機械化対象の boilerplate

実験コードに現れた、毎回手書きしていた塊:

| 役割 | C XS の対応 | 実験コードでの表現 |
|------|-------------|-------------------|
| stack pointer 取得 | `dSP` | `let sp = my_perl.Istack_sp;` |
| mark stack 取り出し | `dAXMARK` / `POPMARK` | `Imarkstack_ptr` 操作 + `sub(1)` |
| 引数個数 | `dITEMS` | `sp.offset_from(mark)` |
| arity check | `Perl_croak_xs_usage(cv, msg)` | 手書き、msg 文字列を生で渡す |
| 引数取り出し | `ST(i)` + `SvIV/SvNV/SvPV_*` | `*Istack_base.add(ax + i)` を読んで型変換 |
| 戻り値 push | `PUSHi` / `PUSHn` / `PUSHp` | `Perl_sv_newmortal` → `Perl_sv_setiv` → `*sp = targ` |
| stack pointer 更新 | (PUSH マクロ内蔵) | `Istack_sp = Istack_base.add(ax + n - 1)` (off-by-one になりやすい) |

これらすべて、Rust 関数のシグネチャ (引数の型、戻り値の型) から
**構造的に導ける**。`#[xs_sub]` の本領は **「Rust の関数シグネチャを XS の
引数解析・戻り値構築に変換する」型駆動コード生成**。

#### 3.11.3 構造的に避けるべきバグ (実験コードに混入していたもの)

`#[xs_sub]` で **構造として防ぐ** べき潜在バグ:

1. **NUL 終端されていない usage 文字列**
   実験では `msg.as_bytes().as_ptr()` を `Perl_croak_xs_usage` に渡して
   いたが、C 側は NUL 終端文字列を期待する。proc-macro は **`c"..."`
   リテラル**を emit して構造的に防ぐ。
2. **`as *const i8`**
   `c_char` が `u8` のプラットフォーム (aarch64 等) では型不一致。
   proc-macro は `c_char` を使う。
3. **`Stack_off_t` の version 差**
   perl のバージョンで型が違う。macrogen 側 (libperl-sys) で吸収済みの
   はずだが、`#[xs_sub]` 出力でも `try_into().unwrap()` が必要に
   ならないように type-aware に書く。
4. **stack pointer の off-by-one**
   `ax + off - 1` のような手計算が散在しないよう、1 箇所に集約。
5. **`my_perl: &mut PerlInterpreter`**
   実験では採用していたが、§3.9 の判断に従って `*mut PerlInterpreter`
   に変える (callback 再入時の aliasing UB を避けるため)。

#### 3.11.4 boot 関数の集約: `xs_boot!` declarative macro を併用

Rust の proc-macro は **個々の item にしか見えない**ため、「全部の
`#[xs_sub]` を集めて `boot_<モジュール名>` を作る」という横断処理が
できない。選択肢:

| 方式 | 仕組み | 評価 |
|------|--------|------|
| (a) `inventory` クレート | static の link-time aggregation | macOS / Windows / cdylib で動作が壊れることがある。**不採用** |
| (b) `xs_boot!` declarative macro | ユーザが sub のリストを書いて渡す | 透明・移植性最高・ABI 安定。**採用** |
| (c) `#[xs_module] mod xs { ... }` | モジュール全体を 1 attribute proc-macro が処理 | エルゴノミクス最良だが proc-macro 側の AST 操作が大きい。**将来検討** |

(b) で実用上の摩擦が顕在化したら (c) に移る、という二段構え。

使用イメージ:

```rust
#[xs_sub]                            // C trampoline + body を生成
fn is_even(n: IV) -> bool { n % 2 == 0 }

#[xs_sub]
fn add(a: IV, b: IV) -> IV { a + b }

xs_boot! {                           // boot_Mytest を生成
    package = "Mytest";
    subs = [is_even, add];
}
```

#### 3.11.5 サポート型の最小セット (Step 3 の初版)

`#[xs_sub]` が Rust 関数シグネチャから読み取って自動変換する型を、
Step 3 の最初の実装では以下に限定する。それ以外の型はコンパイルエラー
として明示する (silently fall through しない):

| 引数 (`ST(i)` → Rust) | 戻り値 (Rust → push) |
|-----------------------|---------------------|
| `IV` (← `SvIV`) | `IV` (→ `Perl_sv_setiv` + PUSHi) |
| `UV` (← `SvUV`) | `UV` (→ `Perl_sv_setuv`) |
| `NV` (← `SvNV`) | `NV` (→ `Perl_sv_setnv`) |
| `&str` / `&CStr` (← `SvPV` 系) | `String` / `&str` (→ `Perl_sv_setpvn`) |
| `*mut SV` (生で受ける) | `bool` (→ IV 0/1 として) |
| | `*mut SV` (生で push) |
| | `Result<T, String>` (Err は `Perl_croak`) |

#### 3.11.6 実験コードからの設計帰結 (まとめ)

- `#[xs_sub]` は **C trampoline と body を 1 セットで生成**する attribute
  proc-macro として実装する (3 層のうち 2 層を担当)。
- boot 関数は **`xs_boot!` declarative macro で集約**する (3 層の残り 1 層)。
- proc-macro 出力では **`*mut PerlInterpreter`**, **`c_char`**,
  **`c"..."` リテラル** を一貫して使い、実験コードに混入していた
  potential bug を構造的に防ぐ。
- 引数 / 戻り値の型変換は **最小セット (IV/UV/NV/str/bool/SV/Result)**
  からスタートし、必要に応じて拡張する。

### 3.12 ワークスペース最終形

```
libperl-rs/                ← トップレベル (新 API、最初は空に近い)
├── Cargo.toml             ← workspace = [libperl-sys, libperl-config,
│                                          libperl-proto0, libperl-macros]
├── src/lib.rs             ← 当初は pub use libperl_sys::*; だけ
├── examples/              ← 新例 (Step 1 以降に追加)
├── libperl-config/        既存
├── libperl-sys/           既存 (macrogen + bindgen + sigdb)
├── libperl-macros/        ★新規 proc-macro クレート
│   ├── Cargo.toml         (proc-macro = true)
│   ├── build.rs           (libperl-config 経由で perl_useithreads cfg を立てる)
│   └── src/lib.rs         (#[thx], #[xs_sub] 等)
└── libperl-proto0/        ★旧コード保管庫
    ├── Cargo.toml         (publish = false)
    ├── src/lib.rs         ← 旧 src/perl.rs
    └── examples/          ← 旧 examples/*.rs と examples/eg/* 一式
```

## 4. ロードマップ (Step 0 〜 Step 3)

### Step 0: 整理 + ビルド緑化 (最初の検証ポイント)

1. workspace に `libperl-proto0` を追加し、現在の `src/perl.rs`,
   `examples/*.rs`, `examples/eg/*` を移動。
2. `libperl-proto0/Cargo.toml` で旧 examples 000-110 がビルド可能なことを
   確認。`SvTRUE` バグもこの段階で proto0 内で修正する。
3. workspace に `libperl-macros` の **空殻** を追加 (`proc-macro = true`、
   build.rs で `perl_useithreads` cfg を立てる、`#[thx]` のスケルトン
   だけ。`#[xs_sub]` は Step 3 で実装するので空殻にも含めなくて良い)。
4. トップレベル `libperl-rs` は `pub use libperl_sys::*;` 程度の最小実装。
5. `cargo build --all` が threaded/non-threaded 両方で通り、
   `runtest-docker.zsh` が緑になることを確認。

### Step 1: 新コア API + PL_* アクセサ

1. **libperl-macrogen に PERLVAR-based 生成機能を追加**:
   - `PL_xxx!()` declarative macro (no-arg、`my_perl` が in scope 前提)
   - `Perl::pl_xxx(&self)` method
   - 書き込み用 `PL_xxx_set!(val)` を必要に応じて
2. **新 `libperl-rs/src/perl.rs`** に最小 `Perl` 構造体を実装:
   - フィールド: `my_perl: NonNull<PerlInterpreter>` (§3.4)
   - メソッド: `new`/`parse`/`as_ptr`/`Drop`
   - `perl_call!(F(args...))` macro を 1 個だけ用意 (`my_perl` in scope
     前提、§3.6 + §3.8)。
3. **北極星例 1 本目**: `examples/100_scan_ops.rs` を新 API + macrogen 出力
   だけで書き直し、threaded/non-threaded 両方で動作確認。冒頭で
   `let my_perl = perl.as_ptr();` を入れ、以降は `PL_main_start!()` 等を
   使うのが標準スタイル。これが PL_* 自動生成 + `NonNull` + `my_perl`
   命名規約の動作証跡になる。

### Step 2: newtype 層と Iterator

1. `Sv` / `Av` / `Hv` / `Cv` / `Op` の newtype。中身は macrogen 関数の薄い
   ラッパ。`SvKind` enum (旧 `Sv::SCALAR/REF/ARRAY/...`) を再構築。
2. Op tree visitor (`OpNextIter`, `OpSiblingIter`)、HV iter を新 API で。
3. **北極星例 2 本目**: `examples/walker_stash.rs` (旧 105/106 相当) と
   `examples/walker_subs.rs` (旧 107-109 相当)。

### Step 3: 新ターゲット機能

1. **Rust 製 XS 支援** (詳細設計は §3.11 を参照):
   `libperl-macros` に以下を本実装する:
   - `#[thx]` (プリミティブ) の展開:
     - threaded: `fn name(my_perl: *mut PerlInterpreter, ...args)`
     - non-threaded: `fn name(...args)`
     - 本体への束縛注入は行わない (`my_perl` 仮引数がそのまま scope に
       居る、§3.8)。
   - `#[xs_sub]` (`#[thx]` を内部で使う、XS 特化) の展開:
     - C trampoline + body の 2 層を 1 セットで生成 (§3.11.1)
     - 引数取り出し / 戻り値 push を Rust シグネチャから自動生成 (§3.11.2)
     - `c"..."` リテラル / `c_char` / `*mut PerlInterpreter` を一貫使用
       して実験コードの潜在バグを構造的に回避 (§3.11.3)
     - サポート型の初版は §3.11.5 の最小セット
   - `xs_boot! { package = ...; subs = [...]; }` declarative macro:
     - boot 関数 (`boot_<モジュール名>`) を生成 (§3.11.4)
     - `Perl_newXS_deffile` での登録 + `Perl_xs_boot_epilog`
   - **検証用例**: 実験リポジトリ
     <https://github.com/hkoba/exp-libperl-rs-xs1> の `is_even` を
     `#[xs_sub]` + `xs_boot!` で書き直し、行数とエラー混入リスクが
     減ることを確認する。
2. **静的解析の素材**: op tree visitor + lexical pad 解決 (`109_scan_subs_intro1.rs` を一般化)。
3. **LSP 向け**: `Perl::parse_only` (実行しないモード) と `CopFILE`/`CopLINE`
   抽出。ここで PL_* アクセサの実用度が問われる。

### Cross-cutting: macrogen への上流フィードバック

- `[CALLS_UNAVAILABLE]` / `[CASCADE_UNAVAILABLE]` で潰れているが下流で
  本当に必要な関数を `doc/handoff-to-downstream.md` の手順で issue 化。
- `skip-codegen.txt` 削減のための原因分析 (Phase 2 の伝播漏れか、codegen
  バグか、構文未対応か)。

## 5. 未確定事項 / 残課題

- [ ] `libperl-proto0` の `Cargo.toml` で `libperl-sys` をどう pin するか
  (`=0.3.1` か `^0.3` か)。
- [ ] `libperl-macros/build.rs` で参照する `libperl-config` を proc-macro
  と consumer crate で **同じ perl** に向ける運用 (CI で固定すれば実害
  なし)。
- [ ] 新 `libperl-rs` のバージョンを `0.4.0` から始めるか、いったん
  `0.4.0-alpha.1` 等にするか。
- [ ] 旧 `src/perl.rs` の `SvTRUE` バグは proto0 移送後に「軽い修正」で
  済ませるか「proto0 では機能 commented-out にする」か。
