#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CStr;
use libperl_sys::*;

use super::av0::*;

pub fn CvPADLIST(cv: *const CV) -> *const PADLIST {
    let xpvcv = unsafe {(*cv).sv_any};
    // print!("xpvcv = {:?}\n", unsafe {*xpvcv});

    unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist}
}

pub fn PadlistARRAY(pl: *const PADLIST) -> *const *const PAD {
    (unsafe {(*pl).xpadl_arr.xpadlarr_alloc})
        as *const *const PAD
}

pub fn PAD_BASE_SV(pl: *const PADLIST, po: isize) -> *const SV {
    let pad = (unsafe {*(PadlistARRAY(pl).add(1))})
        as *const AV;
    if pad.is_null() {
        std::ptr::null()
    } else {
        let array = AvARRAY(pad);
        unsafe {*(array.add(po as usize)) as *const SV}
    }
}

pub fn cv_padnamelist<'a>(cv: *const CV) -> Option<&'a PADNAMELIST> {
    let padnamelist_ptr = fetch_padnamelist(CvPADLIST(cv));
    
    unsafe {padnamelist_ptr.as_ref()}
}

pub fn padnamelist_nth<'a>(pn: &padnamelist, ix: usize) -> Option<&'a padname> {
    unsafe {
        (*(pn.xpadnl_alloc.add(ix))).as_ref()
    }
}

#[cfg(perlapi_ver24)]
pub fn fetch_padnamelist(padlist: *const PADLIST) -> *const PADNAMELIST {
    unsafe {
        (*(*padlist).xpadl_arr.xpadlarr_dbg).padnl
    }
}

#[cfg(not(perlapi_ver24))]
pub fn fetch_padnamelist(padlist: *const PADLIST) -> *const PADNAMELIST {
    unsafe {
        *((*padlist).xpadl_alloc
          as *const *const PADNAMELIST)
    }
}

#[cfg(perlapi_ver22)]
pub fn perl__PadnameTYPE(pn: &padname) -> Option<String> {
    if pn.xpadn_pv.is_null() {
        None
    }
    else if let Some(typestash) = unsafe {
        pn.xpadn_type_u.xpadn_typestash.as_ref()
    } {
        Some(hvname_get(typestash))
    }
    else {
        None
    }
}

#[cfg(perlapi_ver22)]
pub fn perl__PadnamePV(pn: &padname) -> Option<String> {
    if pn.xpadn_pv.is_null() {
        None
    }
    else {
        let varname = unsafe {CStr::from_ptr(pn.xpadn_pv)};
        Some(varname.to_string_lossy().into_owned())
    }
}

fn hv_svook(hv: &HV) -> bool {
    (hv.sv_flags & SVf_OOK) != 0
}

fn hv_sv_any(hv: &HV) -> xpvhv {
    unsafe {*(hv.sv_any as *const xpvhv)}
}

fn hvaux(hv: &HV) -> *const xpvhv_aux {
    unsafe {
        hv.sv_u.svu_hash.add(hv_sv_any(hv).xhv_max + 1)
            as *const xpvhv_aux
    }
}

fn hvaux_hek(hvaux: &xpvhv_aux) -> *const HEK {
    if unsafe {hvaux.xhv_name_u.xhvnameu_name}.is_null() {
        std::ptr::null()
    }
    else if hvaux.xhv_name_count > 0 {
        unsafe {*hvaux.xhv_name_u.xhvnameu_names}
    } else {
        unsafe {hvaux.xhv_name_u.xhvnameu_name}
    }
}

pub fn hvname_get(hv: &HV) -> String {
    if hv_svook(hv) {
        let aux = unsafe {*hvaux(hv)};
        if let Some(hek) = unsafe {hvaux_hek(&aux).as_ref()} {
            return unsafe {CStr::from_ptr(&hek.hek_key[0] as *const c_char)}
                .to_string_lossy().into_owned()
        }
    }
    String::new()
}
