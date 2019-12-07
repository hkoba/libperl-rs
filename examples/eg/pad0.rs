#![allow(non_snake_case)]

use std::ffi::CStr;
use libperl_sys::*;

use super::av0::*;
use super::hv0::*;

pub fn CvPADLIST(cv: *const CV) -> *const PADLIST {
    let xpvcv = unsafe {(*cv).sv_any};
    // print!("xpvcv = {:?}\n", unsafe {*xpvcv});

    unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist}
}

#[cfg(perlapi_ver24)]
pub fn PadlistARRAY(pl: *const PADLIST) -> *const *const PAD {
    (unsafe {(*pl).xpadl_arr.xpadlarr_alloc})
        as *const *const PAD
}

#[cfg(not(perlapi_ver24))]
pub fn PadlistARRAY(pl: *const PADLIST) -> *const *const PAD {
    (unsafe { (*pl).xpadl_alloc })
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
    let pl = CvPADLIST(cv);
    if pl.is_null() {return None}
    let padnamelist_ptr = fetch_padnamelist(pl);
    
    unsafe {padnamelist_ptr.as_ref()}
}

pub fn padnamelist_nth<'a>(pn: &padnamelist, ix: usize) -> Option<&'a padname> {
    if ix >= (pn.xpadnl_max) as usize {
        return None
    }
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
pub fn PadnameTYPE(pn: &padname) -> Option<String> {
    if pn.xpadn_pv.is_null() {
        None
    }
    else if let Some(typestash) = unsafe {
        pn.xpadn_type_u.xpadn_typestash.as_ref()
    } {
        HvNAME(typestash)
    }
    else {
        None
    }
}

#[cfg(perlapi_ver22)]
pub fn PadnamePV(pn: &padname) -> Option<String> {
    if pn.xpadn_pv.is_null() {
        None
    }
    else {
        let varname = unsafe {CStr::from_ptr(pn.xpadn_pv)};
        Some(varname.to_string_lossy().into_owned())
    }
}
