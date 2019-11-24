use libperl_sys;
use super::sv0::*;

#[allow(non_snake_case)]
pub fn CvSTART(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {(*xpvcv).xcv_start_u.xcv_start}
}

#[allow(non_snake_case)]
pub fn CvROOT(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {(*xpvcv).xcv_root_u.xcv_root}
}

#[allow(non_snake_case)]
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

