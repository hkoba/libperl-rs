use std::env;

use libperl_rs::Perl;

#[cfg(perlapi_ver26)]
use libperl_sys::OPclass;

#[derive(Debug)]
enum Op<'a> {
    NULL,
    OP (&'a libperl_sys::op),
    UNOP (&'a libperl_sys::unop),
    BINOP (&'a libperl_sys::binop),
    LOGOP (&'a libperl_sys::logop),
    LISTOP (&'a libperl_sys::listop),
    PMOP (&'a libperl_sys::pmop),
    SVOP (&'a libperl_sys::svop),
    PADOP (&'a libperl_sys::padop),
    PVOP (&'a libperl_sys::pvop),
    LOOP (&'a libperl_sys::loop_),
    COP (&'a libperl_sys::cop),
    METHOP (&'a libperl_sys::methop),
}

struct Walker<'a> {
    perl: &'a Perl,
}

impl<'a> Walker<'a> {

    #[cfg(perl_useithreads)]
    pub fn main_root(&'a self) -> *const libperl_sys::op {
        unsafe {*self.perl.my_perl}.Imain_root
    }

    #[cfg(not(perl_useithreads))]
    pub fn main_root(&'a self) -> *const libperl_sys::op {
        unsafe {libperl_sys::PL_main_root}
    }

    #[cfg(perlapi_ver26)]
    fn op_extract(&'a self, o: *const libperl_sys::op) -> Op {
        let cls = self.perl.op_class(o);
        match cls {
            OPclass::OPclass_NULL => Op::NULL,
            OPclass::OPclass_BASEOP => Op::OP(unsafe {o.as_ref()}.unwrap()),
            OPclass::OPclass_UNOP => Op::UNOP(unsafe {(o as *const libperl_sys::unop).as_ref()}.unwrap()),
            OPclass::OPclass_BINOP => Op::BINOP(unsafe {(o as *const libperl_sys::binop).as_ref()}.unwrap()),
            OPclass::OPclass_LOGOP => Op::LOGOP(unsafe {(o as *const libperl_sys::logop).as_ref()}.unwrap()),
            OPclass::OPclass_LISTOP => Op::LISTOP(unsafe {(o as *const libperl_sys::listop).as_ref()}.unwrap()),
            OPclass::OPclass_PMOP => Op::PMOP(unsafe {(o as *const libperl_sys::pmop).as_ref()}.unwrap()),
            OPclass::OPclass_SVOP => Op::SVOP(unsafe {(o as *const libperl_sys::svop).as_ref()}.unwrap()),
            OPclass::OPclass_PADOP => Op::PADOP(unsafe {(o as *const libperl_sys::padop).as_ref()}.unwrap()),
            OPclass::OPclass_PVOP => Op::PVOP(unsafe {(o as *const libperl_sys::pvop).as_ref()}.unwrap()),
            OPclass::OPclass_LOOP => Op::LOOP(unsafe {(o as *const libperl_sys::loop_).as_ref()}.unwrap()),
            OPclass::OPclass_COP => Op::COP(unsafe {(o as *const libperl_sys::cop).as_ref()}.unwrap()),
            OPclass::OPclass_METHOP => Op::METHOP(unsafe {(o as *const libperl_sys::methop).as_ref()}.unwrap()),
            //        OPclass::OPclass_UNOP_AUX => Op::UNOP_AUX(unsafe {(o as *const libperl_sys::unop_aux).as_ref()}.unwrap()),
            _ => panic!("Unknown op type {:#?}", o),
        }
    }

    fn op_first(o: *const libperl_sys::op) -> *const libperl_sys::op {
        if o.is_null() || (unsafe {*o}.op_flags & libperl_sys::OPf_KIDS as u8) == 0 {
            std::ptr::null()
        } else {
            let uo = o as *const libperl_sys::unop;
            unsafe {*uo}.op_first as *const libperl_sys::op
        }
    }

    #[cfg(perlapi_ver26)]
    fn op_sibling(op: *const libperl_sys::unop) -> *const libperl_sys::op {
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
    fn op_sibling(op: *const libperl_sys::unop) -> *const libperl_sys::op {
        if let Some(op) = unsafe {op.as_ref()} {
            op.op_sibling
        } else {
            std::ptr::null()
        }
    }

    pub fn op_name(&'a self, o: *const libperl_sys::op) -> String {
        let ty = unsafe {*o}.op_type();
        unsafe {
            std::ffi::CStr::from_ptr(libperl_sys::PL_op_name[ty as usize])
        }.to_str().unwrap().to_string()
    }

    #[cfg(perlapi_ver26)]
    pub fn walk(&'a self, o: *const libperl_sys::op, level: isize) {
        let mut kid = Self::op_first(o);
        while !kid.is_null() {
            self.walk(kid, level+1);
            kid = Self::op_sibling(kid as *const libperl_sys::unop);
        }
        print!("{}", "  ".repeat(level as usize));
        println!("{:?} {:?}", self.op_name(o), self.op_extract(o));
    }
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let walker = Walker {perl: &perl};

    let main_root = walker.main_root();
    
    walker.walk(main_root, 0);
}

#[cfg(not(perlapi_ver26))]
fn my_test() {}

fn main() {
    my_test();
}
