#![allow(non_snake_case)]
use libperl_sys::*;
use super::hek0::*;
use super::sv0::*;

pub fn HvNAME(hv: *const HV) -> Option<String> {
    if !SvOOK(hv as *const SV) {
        None
    } else {
        let hek = HvNAME_HEK_NN(hv);
        if !hek.is_null() {
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

pub fn HvAUX(hv: *const HV) -> *const xpvhv_aux {
    let hva = HvARRAY(hv);
    let aux_off = unsafe {*HvANY(hv)}.xhv_max + 1;
    (unsafe {hva.add(aux_off)})
        as *const xpvhv_aux
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
