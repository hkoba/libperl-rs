#![allow(non_snake_case)]

use std::env;
use std::ptr;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char /*, c_int*/};

use libperl_rs::Perl;
use libperl_sys::*;

fn main() {
    my_test();
}

#[cfg(perlapi_ver22)]
fn my_test() {

    let mut perl = Perl::new();

    perl.parse_env_args(env::args(), env::vars());
    
    let main_cv = get_main_cv(&perl);
    print!("main_cv = {:?}\n", unsafe {*main_cv});

    let xpvcv = unsafe {(*main_cv).sv_any};
    print!("xpvcv = {:?}\n", unsafe {*xpvcv});

    let padlist = unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist};
    print!("padlist = {:?}\n", unsafe {*padlist});

    let padnamelist_ptr = fetch_padnamelist(padlist);
    if let Some(padnamelist) = unsafe {padnamelist_ptr.as_ref()} {
        println!("padnamelist = {:?}", padnamelist);
        let mut ix: usize = 0;
        while ix < (padnamelist.xpadnl_fill as usize) {
            let padname = unsafe {(*(padnamelist.xpadnl_alloc.add(ix)))
                                  .as_ref()}.unwrap();
            println!("padname {} = var{{name: {:?}}}, type: {:?}"
                     , ix
                     , perl__PadnamePV(padname)
                     , perl__PadnameTYPE(padname)
            );
            ix += 1;
        }
    }
}

//========================================

#[cfg(perl_useithreads)]
fn get_main_cv(perl: &Perl) -> *const cv {
    unsafe {*perl.my_perl}.Imain_cv
}

#[cfg(not(perl_useithreads))]
fn get_main_cv(_perl: &Perl) -> *const cv {
    unsafe {libperl_sys::PL_main_cv}
}

#[cfg(perlapi_ver24)]
fn fetch_padnamelist(padlist: *const PADLIST) -> *const PADNAMELIST {
    unsafe {
        (*(*padlist).xpadl_arr.xpadlarr_dbg).padnl
    }
}

#[cfg(not(perlapi_ver24))]
fn fetch_padnamelist(padlist: *const PADLIST) -> *const PADNAMELIST {
    unsafe {
        *((*padlist).xpadl_alloc
          as *const *const PADNAMELIST)
    }
}

#[cfg(perlapi_ver22)]
fn perl__PadnameTYPE(pn: &padname) -> Option<String> {
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
fn perl__PadnamePV(pn: &padname) -> Option<String> {
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

fn hvname_get(hv: &HV) -> String {
    if hv_svook(hv) {
        let aux = unsafe {*hvaux(hv)};
        if let Some(hek) = unsafe {hvaux_hek(&aux).as_ref()} {
            return unsafe {CStr::from_ptr(&hek.hek_key[0] as *const c_char)}
                .to_string_lossy().into_owned()
        }
    }
    String::new()
}
