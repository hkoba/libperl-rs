#![allow(non_snake_case)]
use libperl_sys;
use libperl_sys::svtype;
use super::sv0::*;
use super::hek0::*;

pub fn GvNAME_HEK(gv: *const libperl_sys::gv) -> *const libperl_sys::HEK {
    assert!(isGV_with_GP(gv));
    let xpvgv = unsafe {(*gv).sv_any};
    unsafe {(*xpvgv).xiv_u.xivu_namehek}
}

pub fn GvSTASH(gv: *const libperl_sys::gv) -> *const libperl_sys::HV {
    assert!(isGV_with_GP(gv));
    let xpvgv = unsafe {(*gv).sv_any};
    unsafe {(*xpvgv).xnv_u.xgv_stash}
}

pub fn isGV_with_GP(gv: *const libperl_sys::gv) -> bool {
    use libperl_sys::{SVp_POK, SVpgv_GP};
    let flags = unsafe {(*gv).sv_flags};
    (flags & (SVp_POK|SVpgv_GP)) == SVpgv_GP
        &&
        match SvTYPE(gv as *const libperl_sys::sv) {
            svtype::SVt_PVGV | svtype::SVt_PVLV => true,
            _ => false,
        }
}

#[allow(non_snake_case)]
pub fn GvCV(gv: *const libperl_sys::gv) -> *const libperl_sys::cv {
    let gp = GvGP(gv);
    if gp.is_null() {
        std::ptr::null()
    } else {
        unsafe {(*gp).gp_cv}
    }
}

#[allow(non_snake_case)]
pub fn GvGP(gv: *const libperl_sys::gv) -> *const libperl_sys::gp {
    if isGV_with_GP(gv) {
        unsafe {(*gv).sv_u.svu_gp}
    } else {
        std::ptr::null()
    }
}

#[allow(non_snake_case)]
pub fn GvLINE(gv: *const libperl_sys::gv) -> u32 {
    let gp = GvGP(gv);
    assert_ne!(gp, std::ptr::null_mut());
    unsafe {(*gp).gp_line()}
}

#[allow(non_snake_case)]
pub fn GvFILE(gv: *const libperl_sys::gv) -> String {
    let gp = GvGP(gv);
    assert_ne!(gp, std::ptr::null_mut());
    HEK_KEY(unsafe {(*gp).gp_file_hek})
}


