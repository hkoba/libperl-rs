use std::env;

use libperl_rs::*;

mod eg;
use eg::op0::*;

use libperl_sys::op;
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

#[allow(non_snake_case)]
fn CvROOT(cv: *const libperl_sys::cv) -> *const libperl_sys::OP {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {(*xpvcv).xcv_root_u.xcv_root}
}

#[allow(non_snake_case)]
fn CvFILE(cv: *const libperl_sys::cv) -> String {
    assert_eq!(SvTYPE(cv as *const libperl_sys::sv), svtype::SVt_PVCV);
    let xpvcv = unsafe {(*cv).sv_any as *const libperl_sys::xpvcv};
    unsafe {std::ffi::CStr::from_ptr((*xpvcv).xcv_file).to_string_lossy().into_owned()}
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
                panic!("Not yet implemented")
            }
        }
    }
}

#[cfg(perlapi_ver26)]
pub struct Walker<'a> {
    pub perl: &'a Perl,
}

#[cfg(perlapi_ver26)]
impl<'a> Walker<'a> {
    pub fn walk(&'a self, o: *const op, level: isize) {
        for kid in sibling_iter(o) {
            self.walk(kid, level+1);
        }
        print!("{}", "  ".repeat(level as usize));
        println!("{:?} {:?}", op_name(o), op_extract(&self.perl, o));
    }
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    // get_cvstash(perl.get_main_cv()); returns null. Why?
    
    let stash = perl.get_defstash();
    
    assert_eq!(perl.gv_stashpv("main", 0), stash);
    
    let walker = Walker {perl: &perl};

    for (name, gv) in eg::hv_iter0::HvIter::new(&perl, stash) {

        if let Some(sv) = SvRV(gv) {
            if let Sv::CODE(cv) = sv_extract(sv) {
                println!("sub {:?} file {}", name, CvFILE(cv));
                walker.walk(CvROOT(cv), 0);
                println!("");
            }
        }
    }
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
}

fn main() {
    my_test();
}
