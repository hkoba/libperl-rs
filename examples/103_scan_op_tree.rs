use std::env;
use std::ffi::CStr;

use libperl_rs::Perl;
use libperl_sys::*;

#[cfg(perl_useithreads)]
fn get_main_root(perl: &Perl) -> *const op {
    unsafe {*perl.my_perl}.Imain_root
}

#[cfg(not(perl_useithreads))]
fn get_main_root(_perl: &Perl) -> *const op {
    unsafe {libperl_sys::PL_main_root}
}

#[cfg(perlapi_ver26)]
fn op_sibling(op: *const unop) -> *const op {
    // PERL_OP_PARENT is on since 5.26
    if let Some(op) = unsafe {op.as_ref()} {
        if op.op_moresib() == 1 as u32 {
            op.op_sibparent
        } else {
            std::ptr::null()
        }
    } else {
        std::ptr::null()
    }
}

#[cfg(not(perlapi_ver26))]
fn op_sibling(op: *const unop) -> *const op {
    if let Some(op) = unsafe {op.as_ref()} {
        op.op_sibling
    } else {
        std::ptr::null()
    }
}


fn tree(op: *const op, level: isize) {
    if !op.is_null() && (unsafe {*op}.op_flags & OPf_KIDS as u8) != 0 {
        let op = op as *const unop;
        let mut kid = unsafe {*op}.op_first as *const unop;
        while ! kid.is_null() {
            tree(kid as *const op, level+1);
            kid = op_sibling(kid) as *const unop;
        }
    }
    let ty = unsafe {*op}.op_type();
    print!("{}", "  ".repeat(level as usize));
    println!("{} {:#?} {}", level, op
             , unsafe {
                 CStr::from_ptr(PL_op_name[ty as usize])
             }.to_str().unwrap());
}

fn my_test() {
    let mut perl = Perl::new();
    
    perl.parse_env_args(env::args(), env::vars());
    
    let op = get_main_root(&perl);
    tree(op, 0);
}

fn main() {
    my_test()
}
