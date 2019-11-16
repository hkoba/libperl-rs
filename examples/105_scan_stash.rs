use std::env;

use libperl_rs::*;

mod eg;

use libperl_sys::svtype;

#[allow(non_snake_case)]
fn SvRV<'a>(sv: *const libperl_sys::sv) -> Option<&'a libperl_sys::sv> {
    if (unsafe {(*sv).sv_flags} & libperl_sys::SVf_ROK) != 0 {
        let s = unsafe {(*sv).sv_u.svu_rv};
        unsafe {s.as_ref()}
    } else {
        None
    }
}

#[allow(non_snake_case)]
fn SvTYPE(sv: *const libperl_sys::sv) -> svtype {
    let svt = svtype_raw(sv);
    unsafe {*(&svt as *const u32 as *const svtype)}
}
fn svtype_raw(sv: *const libperl_sys::sv) -> u32 {
    let flags = unsafe {(*sv).sv_flags};
    flags & libperl_sys::SVTYPEMASK
}

#[allow(non_snake_case)]
fn CvSTART(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {(*xpvcv).xcv_start_u.xcv_start}
}

enum Sv {
    SCALAR(*const libperl_sys::sv),
    ARRAY(*const libperl_sys::av),
    HASH(*const libperl_sys::hv),
    CODE(*const libperl_sys::cv),
}

fn sv_extract(sv: *const libperl_sys::sv) -> Sv {
    if svtype_raw(sv) < svtype::SVt_PVAV as u32 {
        Sv::SCALAR(sv)
    } else {
        match SvTYPE(sv) {
            svtype::SVt_PVAV => Sv::ARRAY(sv as *const libperl_sys::av),
            svtype::SVt_PVHV => Sv::HASH(sv as *const libperl_sys::hv),
            svtype::SVt_PVCV => Sv::CODE(sv as *const libperl_sys::cv),
            _ => {
                panic!("really?")
            }
        }
    }
}

fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    // get_cvstash(perl.get_main_cv()); returns null. Why?
    
    let stash = perl.get_defstash();
    
    assert_eq!(perl.gv_stashpv("main", 0), stash);
    
    for (name, gv) in eg::hv_iter0::HvIter::new(&perl, stash) {

        if let Some(sv) = SvRV(gv) {
            match sv_extract(sv) {
                Sv::CODE(cv) => {
                    println!("sub {:?}", name);
                    for op in eg::op0::next_iter(CvSTART(cv)) {
                        print!("{:?}\n", eg::op0::op_extract(&perl, op));
                    }
                    println!("");
                }
                _ => {}
            }
        } else {
            
        }
    }
}

fn main() {
    my_test();
}
