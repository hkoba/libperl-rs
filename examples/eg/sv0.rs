#![allow(non_snake_case)]

pub use libperl_sys::{SV, svtype};

use super::gv0::*;
use super::hek0::*;
use super::hv0::*;

#[derive(Debug)]
pub enum Sv {
    SCALAR(*const libperl_sys::sv),
    GLOB {
        gv: *const libperl_sys::gv,
        name: String,
        stash: (Option<String>, *const libperl_sys::HV),
        gp: *const libperl_sys::gp,
    },
    ARRAY(*const libperl_sys::av),
    HASH(*const libperl_sys::hv),
    CODE(*const libperl_sys::cv),
    NIMPL(svtype, *const libperl_sys::sv),
}

pub fn sv_extract/*<'a>*/(sv: *const libperl_sys::sv) -> Sv/*<'a>*/ {
    if svtype_raw(sv) == svtype::SVt_PVGV as u32 {
        let gv = sv as *const libperl_sys::gv;
        let stash = GvSTASH(gv);
        Sv::GLOB {
            gv,
            name: HEK_KEY(GvNAME_HEK(gv)),
            stash: (HvNAME(stash), stash),
            gp: GvGP(gv),
        }
    }
    else if svtype_raw(sv) < svtype::SVt_PVAV as u32 {
        // let flags = unsafe {(*sv).sv_flags};
        // let iv = if (flags & SVp_IOK) != 0 {
        //     let xpviv = (*sv).sv_any;
        //     Some(unsafe {(*xpviv).xiv_iv})
        // } else {
        //     None
        // };
        Sv::SCALAR(sv)
    }
    else {
        match SvTYPE(sv) {
            svtype::SVt_PVAV => Sv::ARRAY(sv as *const libperl_sys::av),
            svtype::SVt_PVHV => Sv::HASH(sv as *const libperl_sys::hv),
            svtype::SVt_PVCV => Sv::CODE(sv as *const libperl_sys::cv),
            svt => {
                Sv::NIMPL(svt, sv)
            }
        }
    }
}

pub fn SvTYPE(sv: *const libperl_sys::sv) -> svtype {
    let svt = svtype_raw(sv);
    unsafe {*(&svt as *const u32 as *const svtype)}
}
pub fn svtype_raw(sv: *const libperl_sys::sv) -> u32 {
    let flags = unsafe {(*sv).sv_flags};
    flags & libperl_sys::SVTYPEMASK
}

pub fn SvRV<'a>(sv: *const libperl_sys::sv) -> Option<&'a libperl_sys::sv> {
    if (unsafe {(*sv).sv_flags} & libperl_sys::SVf_ROK) != 0 {
        let s = unsafe {(*sv).sv_u.svu_rv};
        unsafe {s.as_ref()}
    } else {
        None
    }
}

pub fn SvOOK(sv: *const SV) -> bool {
    if sv.is_null() {
        false
    } else {
        (unsafe {(*sv)}.sv_flags & libperl_sys::SVf_OOK) != 0
    }
}
