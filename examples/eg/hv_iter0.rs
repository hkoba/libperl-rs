use libperl_sys::*;
use libperl_rs::Perl;

pub struct HvIter<'a> {
    perl: &'a Perl,
    hv: *mut HV,
    he: *mut HE,
}

impl<'a> HvIter<'a> {
    pub fn new(perl: &'a Perl, hv: *mut HV) -> HvIter<'a> {
        perl.hv_iterinit(hv);
        HvIter {perl, hv, he: std::ptr::null_mut()}
    }
}

impl<'a> Iterator for HvIter<'a> {
    type Item = (String, Option<&'a SV>);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.he = self.perl.hv_iternext(self.hv);
        if !self.he.is_null() {
            let name = self.perl.hv_iterkey(self.he);
            let value = self.perl.hv_iterval(self.hv, self.he);
            Some((name, value))
        } else {
            None
        }
    }
}
