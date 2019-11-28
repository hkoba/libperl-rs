#![allow(non_snake_case)]

use libperl_sys::*;

pub fn AvARRAY(ary: *const libperl_sys::av) -> *const *const SV {
    (unsafe {(*ary).sv_u.svu_array})
        as *const *const SV
}
