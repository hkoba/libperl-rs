#![allow(non_snake_case)]

use libperl_sys;
use super::sv0::*;

pub fn CvSTART(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    if (unsafe {*xpvcv}.xcv_flags & libperl_sys::CVf_ISXSUB) != 0 {
        return std::ptr::null()
    }
    unsafe {(*xpvcv).xcv_start_u.xcv_start}
}

pub fn CvROOT(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    if (unsafe {*xpvcv}.xcv_flags & libperl_sys::CVf_ISXSUB) != 0 {
        return std::ptr::null()
    }
    unsafe {(*xpvcv).xcv_root_u.xcv_root}
}

pub fn CvFLAGS(cv: *const libperl_sys::cv) -> u32 {
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {*xpvcv}.xcv_flags
}

pub fn CvFILE(cv: *const libperl_sys::cv) -> Option<String> {
    if cv.is_null() {
        return None
    }
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    let xcv_file = unsafe {(*xpvcv).xcv_file};
    if ! xcv_file.is_null() {
        Some(unsafe {std::ffi::CStr::from_ptr(xcv_file).to_string_lossy().into_owned()})
    } else {
        None
    }
}

