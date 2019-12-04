#![allow(non_snake_case)]
pub fn HEK_KEY(hek: *const libperl_sys::HEK) -> String {
    if ! hek.is_null() {
        let cs = unsafe {&(*hek).hek_key[0]};
        unsafe {std::ffi::CStr::from_ptr(cs).to_string_lossy().into_owned()}
    } else {
        String::new()
    }
}

