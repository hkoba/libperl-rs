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
fn CvFILE(cv: *const libperl_sys::cv) -> Option<String> {
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

#[allow(non_snake_case)]
fn GvGP(gv: *const libperl_sys::gv) -> *const libperl_sys::gp {
    match SvTYPE(gv as *const libperl_sys::sv) {
        svtype::SVt_PVGV | svtype::SVt_PVLV if (unsafe {(*gv).sv_flags} & (libperl_sys::SVp_POK as u32|libperl_sys::SVpgv_GP as u32)) == libperl_sys::SVpgv_GP
            => unsafe {(*gv).sv_u.svu_gp},
        _ => std::ptr::null()
    }
}

#[allow(non_snake_case)]
fn GvLINE(gv: *const libperl_sys::gv) -> u32 {
    let gp = GvGP(gv);
    assert_ne!(gp, std::ptr::null_mut());
    unsafe {(*gp).gp_line()}
}

#[allow(non_snake_case)]
fn GvFILE(gv: *const libperl_sys::gv) -> Option<String> {
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


#[derive(Debug)]
enum Sv {
    SCALAR(*const libperl_sys::sv),
    GLOB(*const libperl_sys::gv),
    ARRAY(*const libperl_sys::av),
    HASH(*const libperl_sys::hv),
    CODE(*const libperl_sys::cv),
}

fn sv_extract(sv: *const libperl_sys::sv) -> Sv {
    if svtype_raw(sv) == svtype::SVt_PVGV as u32 {
        Sv::GLOB(sv as *const libperl_sys::gv)
    }
    else if svtype_raw(sv) < svtype::SVt_PVAV as u32 {
        Sv::SCALAR(sv)
    }
    else {
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

    println!("main file = {:?}", CvFILE(perl.get_main_cv()));

    let emitter = |name: &String, cv: *const libperl_sys::cv| {
        println!("sub {:?} file {:?}", name, CvFILE(cv));
        walker.walk(CvROOT(cv), 0);
        println!("");
    };

    for (name, item) in eg::hv_iter0::HvIter::new(&perl, stash) {

        // ref $main::{foo} eq 'CODE'
        if let Some(Sv::CODE(cv)) = SvRV(item).map(|sv| sv_extract(sv)) {
            emitter(&name, cv)
        }
        // ref (\$main::{foo}) eq 'GLOB'
        else if let Sv::GLOB(gv) = sv_extract(item) {
            let gp = GvGP(gv);
            let cv = unsafe {(*gp).gp_cv};
            if let Some(file) = CvFILE(cv) {
                if file == "-e" {
                    emitter(&name, cv);
                } else {
                    println!("name = {:?} from file {:?}", name, file);
                }
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
