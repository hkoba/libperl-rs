
// cargo run --example 103_scan_op_tree -- -le 'print "FOO"'

use std::env;
use std::ffi::CStr;

use libperl_rs::Perl;
use libperl_sys::*;

mod eg;

fn tree(op: *const op, level: isize) {
    if !op.is_null() && (unsafe {*op}.op_flags & OPf_KIDS as u8) != 0 {
        let op = op as *const unop;
        let mut kid = unsafe {*op}.op_first as *const unop;
        while ! kid.is_null() {
            tree(kid as *const op, level+1);
            kid = eg::op0::op_sibling(kid) as *const unop;
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
    
    let op = perl.get_main_root();
    tree(op, 0);
}

fn main() {
    my_test()
}
