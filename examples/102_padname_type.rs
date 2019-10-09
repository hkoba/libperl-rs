use std::env;

use libperl_rs::perl::Perl;
use libperl_sys::{cv, PADLIST, PADNAMELIST};

#[cfg(perl_useithreads)]
fn get_main_cv(perl: &Perl) -> *const cv {
    unsafe {(*perl.my_perl)}.Imain_cv
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

fn test() {
    let mut perl = Perl::new();

    perl.parse_env_args(env::args(), env::vars());
    
    let main_cv = get_main_cv(&perl);
    
    print!("main_cv = {:#?}\n", main_cv);
    let xpvcv = unsafe {(*main_cv).sv_any};
    print!("xpvcv = {:#?}\n", xpvcv);
    let padlist = unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist};
    print!("padlist = {:#?}\n", padlist);
    let padnamelist_ptr = fetch_padnamelist(padlist);
    if let Some(padnamelist) = unsafe {padnamelist_ptr.as_ref()} {
        println!("padnamelist = {:?}", padnamelist);
        let mut ix: usize = 0;
        while ix < (padnamelist.xpadnl_fill as usize) {
            let padname = unsafe {(*(padnamelist.xpadnl_alloc.add(ix)))
                                  .as_ref()};
            println!("padname {} = {:?}"
                     , ix
                     , padname);
            ix += 1;
        }
    }
}

fn main() {
    test();
}

