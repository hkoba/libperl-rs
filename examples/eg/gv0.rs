use libperl_sys;
use libperl_sys::svtype;
use super::sv0::*;

#[allow(non_snake_case)]
pub fn GvGP(gv: *const libperl_sys::gv) -> *const libperl_sys::gp {
    match SvTYPE(gv as *const libperl_sys::sv) {
        svtype::SVt_PVGV | svtype::SVt_PVLV if (unsafe {(*gv).sv_flags} & (libperl_sys::SVp_POK as u32|libperl_sys::SVpgv_GP as u32)) == libperl_sys::SVpgv_GP
            => unsafe {(*gv).sv_u.svu_gp},
        _ => std::ptr::null()
    }
}

#[allow(non_snake_case)]
pub fn GvLINE(gv: *const libperl_sys::gv) -> u32 {
    let gp = GvGP(gv);
    assert_ne!(gp, std::ptr::null_mut());
    unsafe {(*gp).gp_line()}
}

#[allow(non_snake_case)]
pub fn GvFILE(gv: *const libperl_sys::gv) -> Option<String> {
    let gp = GvGP(gv);
    assert_ne!(gp, std::ptr::null_mut());
    let hek = unsafe {(*gp).gp_file_hek};
    if ! hek.is_null() {
        let cs = unsafe {&(*hek).hek_key[0]};
        Some(unsafe {std::ffi::CStr::from_ptr(cs).to_string_lossy().into_owned()})
    } else {
        None
    }
}


