use libperl_rs::Perl;

use libperl_sys::*;

use super::op0::*;

pub struct Walker<'a> {
    pub perl: &'a Perl,
}

impl<'a> Walker<'a> {
    #[cfg(perlapi_ver26)]
    pub fn walk(&'a self, o: *const op, level: isize) {
        let mut kid = op_first(o);
        while !kid.is_null() {
            self.walk(kid, level+1);
            kid = op_sibling(kid as *const unop);
        }
        print!("{}", "  ".repeat(level as usize));
        println!("{:?} {:?}", op_name(o), op_extract(&self.perl, o));
    }
}
