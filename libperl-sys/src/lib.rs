pub mod perl_core;
pub use perl_core::*;

use std::ffi::CStr;

use std::os::raw::{c_char, c_int /*, c_void, c_schar*/};

fn core_op_name(o: &op) -> Option<String> {
    let ty = o.op_type();
    if (ty as usize) < unsafe {PL_op_name.len()} {
        let op_name = unsafe {CStr::from_ptr(PL_op_name[ty as usize])};
        Some(String::from(op_name.to_str().unwrap()))
    } else {
        None
    }
}

impl std::fmt::Display for op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {:?}={:#?} {:?} }}"
               , core_op_name(&self)
               , (self as *const op)
               , self)
    }
}

fn hv_svook(hv: &HV) -> bool {
    (hv.sv_flags & SVf_OOK) != 0
}

fn hv_sv_any(hv: &HV) -> xpvhv {
    unsafe {*(hv.sv_any as *const xpvhv)}
}

fn hvaux(hv: &HV) -> *const xpvhv_aux {
    unsafe {
        hv.sv_u.svu_hash.add(hv_sv_any(hv).xhv_max + 1)
            as *const xpvhv_aux
    }
}

fn hvaux_hek(hvaux: &xpvhv_aux) -> *const HEK {
    if unsafe {hvaux.xhv_name_u.xhvnameu_name}.is_null() {
        std::ptr::null()
    }
    else if hvaux.xhv_name_count > 0 {
        unsafe {*hvaux.xhv_name_u.xhvnameu_names}
    } else {
        unsafe {hvaux.xhv_name_u.xhvnameu_name}
    }
}

fn hvname_get(hv: &HV) -> String {
    if hv_svook(hv) {
        let aux = unsafe {*hvaux(hv)};
        if let Some(hek) = unsafe {hvaux_hek(&aux).as_ref()} {
            return unsafe {CStr::from_ptr(&hek.hek_key[0] as *const c_char)}
                .to_string_lossy().into_owned()
        }
    }
    String::new()
}

impl std::fmt::Debug for padname {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.xpadn_pv.is_null() {
            return Ok(());
        }
        let varname = unsafe {CStr::from_ptr(self.xpadn_pv)};
        let varname = varname.to_string_lossy().into_owned();
        if let Some(typestash) = unsafe {
            self.xpadn_type_u.xpadn_typestash.as_ref()
        } {
            write!(f, "var {{name: {:?}, type: {:?}}}"
                   , varname, hvname_get(typestash))
        } else {
            write!(f, "var {{name: {:?}}}"
                   , varname)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let perl = unsafe { super::perl_alloc() };
        unsafe {
            super::perl_construct(perl);
        };
    }
}
