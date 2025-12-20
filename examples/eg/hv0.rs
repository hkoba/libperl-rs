#![allow(non_snake_case)]
use libperl_sys::*;
use super::hek0::*;
use super::sv0::*;

pub fn HvNAME(hv: *const HV) -> Option<String> {
    if !SvOOK(hv as *const SV) {
        None
    } else {
        let hek = HvNAME_HEK_NN(hv);
        if let Some(hek) = unsafe {hek.as_ref()} {
            Some(HEK_KEY(hek))
        } else {
            None
        }
    }
}

pub fn HvANY(hv: *const HV) -> *const xpvhv {
    unsafe {(*hv).sv_any}
}

pub fn HvARRAY(hv: *const HV) -> *const *const HE {
    (unsafe {(*hv).sv_u.svu_hash})
        as *const *const HE
}

#[cfg(perlapi_ver36)]
pub fn HvAUX(hv: *const HV) -> *const xpvhv_aux {
    let xpv = unsafe {(*hv).sv_any} as *const xpvhv_with_aux;
    unsafe {& (*xpv).xhv_aux}
}

#[cfg(not(perlapi_ver36))]
pub fn HvAUX(hv: *const HV) -> *const xpvhv_aux {
    let xpv = unsafe {(*hv).sv_any} as *const xpvhv;
    unsafe {& (*xpv).xhv_aux}
}

pub fn HvNAME_HEK_NN(hv: *const HV) -> *const HEK {
    let hvaux = HvAUX(hv);
    if hvaux.is_null() {
        return std::ptr::null()
    }
    let hvaux = unsafe {hvaux.as_ref()}.unwrap();
    if unsafe {hvaux.xhv_name_u.xhvnameu_name}.is_null() {
        std::ptr::null()
    }
    else if hvaux.xhv_name_count > 0 {
        unsafe {*hvaux.xhv_name_u.xhvnameu_names}
    } else {
        unsafe {hvaux.xhv_name_u.xhvnameu_name}
    }
}
