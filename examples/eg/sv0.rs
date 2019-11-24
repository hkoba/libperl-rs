pub use libperl_sys::svtype;

#[derive(Debug)]
pub enum Sv {
    SCALAR(*const libperl_sys::sv),
    GLOB(*const libperl_sys::gv),
    ARRAY(*const libperl_sys::av),
    HASH(*const libperl_sys::hv),
    CODE(*const libperl_sys::cv),
}

pub fn sv_extract(sv: *const libperl_sys::sv) -> Sv {
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

#[allow(non_snake_case)]
pub fn SvTYPE(sv: *const libperl_sys::sv) -> svtype {
    let svt = svtype_raw(sv);
    unsafe {*(&svt as *const u32 as *const svtype)}
}
pub fn svtype_raw(sv: *const libperl_sys::sv) -> u32 {
    let flags = unsafe {(*sv).sv_flags};
    flags & libperl_sys::SVTYPEMASK
}

#[allow(non_snake_case)]
pub fn SvRV<'a>(sv: *const libperl_sys::sv) -> Option<&'a libperl_sys::sv> {
    if (unsafe {(*sv).sv_flags} & libperl_sys::SVf_ROK) != 0 {
        let s = unsafe {(*sv).sv_u.svu_rv};
        unsafe {s.as_ref()}
    } else {
        None
    }
}
