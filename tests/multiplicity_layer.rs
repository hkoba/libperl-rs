//! GH-20 smoke test: MULTIPLICITY 吸収層 — 読み書き両用の `PL_xxx_ptr!()`
//! と、threaded 風呼び出し規約の `thx` shim モジュール。
//!
//! tests/perlvar_macros.rs と同じ理由でここに置く: `#[macro_export]`
//! マクロは定義クレート内から絶対パスで参照できないため、下流クレート
//! (libperl-rs) 側から exercise する。ファイル内の #[test] は 1 つ
//! (1 プロセス 1 インタプリタの原則 — 非 threaded では状態が大域)。

use libperl_rs::Perl;
use libperl_sys as sys;

#[test]
fn multiplicity_layer() {
    let mut perl = Perl::new();
    let empty: &[&str] = &[];
    let rc = perl.parse(&["", "-e", "1"], empty);
    assert_eq!(rc, 0, "perl_parse failed");
    let my_perl = perl.as_ptr();

    // ── PL_xxx_ptr!: 読みが read 専用マクロと一致する ──
    let sp_read = sys::PL_stack_sp!(my_perl);
    let sp_via_ptr = unsafe { *sys::PL_stack_sp_ptr!(my_perl) };
    assert_eq!(
        sp_read, sp_via_ptr,
        "PL_stack_sp_ptr! disagrees with PL_stack_sp!"
    );

    // ── PL_xxx_ptr!: 書き込みが read 側から見える (PL_savebegin) ──
    let p = sys::PL_savebegin_ptr!(my_perl);
    let old = unsafe { *p };
    unsafe { *p = !old };
    assert_eq!(
        sys::PL_savebegin!(my_perl),
        !old,
        "write through PL_savebegin_ptr! not visible to PL_savebegin!"
    );
    unsafe { *p = old };

    unsafe {
        // ── thx: context 転送形 (Perl_newSViv は threaded で pTHX を取る) ──
        let sv = sys::thx::Perl_newSViv(my_perl, 42);
        assert!(!sv.is_null(), "thx::Perl_newSViv returned null");
        assert_eq!(sys::thx::Perl_sv_2iv(my_perl, sv), 42);

        // ── thx: context 破棄形 (SvTYPE は C で aTHX を取らない) ──
        let av = sys::thx::Perl_newAV(my_perl);
        assert_eq!(
            sys::thx::SvTYPE(my_perl, av as *mut sys::SV),
            sys::svtype::SVt_PVAV,
            "thx::SvTYPE misreported a fresh AV"
        );

        // ── thx: 5.28/5.30 では alias 経由になる Perl_SvREFCNT_dec ──
        sys::thx::Perl_SvREFCNT_dec(my_perl, sv);
        sys::thx::Perl_SvREFCNT_dec(my_perl, av as *mut sys::SV);
    }
}
