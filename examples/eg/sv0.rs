#![allow(non_snake_case)]

pub use libperl_sys::{SV, svtype};

use super::gv0::*;
use super::hek0::*;
use super::hv0::*;

pub struct VarName<'a> (pub &'a str);

impl<'a> std::fmt::Debug for VarName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

struct SvtypeWrap(svtype);

impl std::fmt::Debug for SvtypeWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("svtype::")?;
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub enum IVUV {
    IV(isize),
    UV(usize),
}

// pub enum Immortal {
//     UNDEF, NO, YES, ZERO, PLACEHOLDER,
// }

pub enum Sv {
    SCALAR {
        svtype: svtype,
        ivuv: Option<IVUV>,
        nv: Option<f64>,
        pv: Option<String>,
        /*special: OptionCore<),*/
        sv: *const libperl_sys::sv,
    },
    REF(Box<Sv>),
    REGEXP(*const libperl_sys::REGEXP),
    GLOB {
        gv: *const libperl_sys::gv,
        name: String,
        stash: (Option<String>, *const libperl_sys::HV),
        gp: *const libperl_sys::gp,
    },
    ARRAY(*const libperl_sys::av),
    HASH(*const libperl_sys::hv),
    CODE(*const libperl_sys::cv),
    NIMPL(svtype, *const libperl_sys::sv),
}

impl std::fmt::Debug for Sv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Sv::SCALAR {svtype, ref ivuv, nv, pv, sv: _} => {
                f.debug_struct("Sv::SCALAR")
                    .field("svtype", &SvtypeWrap(*svtype))
                    .field("ivuv", ivuv)
                    .field("nv", nv)
                    .field("pv", pv)
                    .field("sv", &VarName("_"))
                    .finish()
            },
            Sv::REF(_) => {
                f.debug_tuple("Sv::REF").field(&VarName("_")).finish()
            },
            Sv::REGEXP(_) => {
                f.debug_tuple("Sv::REGEXP").field(&VarName("_")).finish()
            },
            Sv::GLOB {ref name, ref stash, ..} => {
                f.debug_struct("Sv::GLOB")
                    .field("gv", &VarName("_"))
                    .field("name", name)
                    .field("stash", stash)
                    .field("gp", &VarName("_"))
                    .finish()
            },
            Sv::ARRAY(_) => {
                f.debug_tuple("Sv::ARRAY").field(&VarName("_")).finish()
            },
            Sv::HASH(_) => {
                f.debug_tuple("Sv::HASH").field(&VarName("_")).finish()
            },
            Sv::CODE(_) => {
                f.debug_tuple("Sv::CODE").field(&VarName("_")).finish()
            },
            Sv::NIMPL(typ, _sv) => {
                f.debug_tuple("Sv::NIMPL")
                    .field(&typ)
                    .field(&VarName("_"))
                    .finish()
            },
        }
    }
}

pub fn sv_extract/*<'a>*/(sv: *const libperl_sys::sv) -> Sv/*<'a>*/ {
    // TODO: STASH, MAGIC
    match SvTYPE(sv) {
        svtype::SVt_PVAV => Sv::ARRAY(sv as *const libperl_sys::av),
        svtype::SVt_PVHV => Sv::HASH(sv as *const libperl_sys::hv),
        svtype::SVt_PVCV => Sv::CODE(sv as *const libperl_sys::cv),
        svtype::SVt_REGEXP => Sv::REGEXP(sv as *const libperl_sys::REGEXP),
        svtype::SVt_PVGV => {
            let gv = sv as *const libperl_sys::gv;
            let stash = GvSTASH(gv);
            Sv::GLOB {
                gv,
                name: HEK_KEY(GvNAME_HEK(gv)),
                stash: (HvNAME(stash), stash),
                gp: GvGP(gv),
            }
        },
        // svtype::SVt_IV => {
        //     Sv::SCALAR {
        //         svtype: SvTYPE(sv), sv, ivuv: sv_extract_ivuv(sv),
        //         nv: None, pv: None, rv: None,
        //     }
        // },
        _ => sv_extract_scalar(sv),
    }
}

fn sv_extract_scalar(sv: *const libperl_sys::sv) -> Sv {
    let svt = SvTYPE(sv);
    if (svt as u32) < (svtype::SVt_PVAV as u32) {
        if let Some(rv) = SvRV(sv) {
            return Sv::REF(Box::new(sv_extract(rv)))
        }
        let ivuv = sv_extract_ivuv(sv);
        let nv = sv_extract_nv(sv);
        let pv = sv_extract_pv(sv);
        Sv::SCALAR {
            svtype: svt, sv, ivuv, nv, pv
        }
    } else {
        Sv::NIMPL(svt, sv)
    }
}

pub fn sv_extract_pv(sv: *const libperl_sys::sv) -> Option<String> {
    let ptr = SvPVX(sv);
    if !ptr.is_null() {
        Some (unsafe {std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()})
    } else {
        None
    }
}

pub fn SvPVX(sv: *const libperl_sys::sv) -> *const std::os::raw::c_char {
    if sv_has_pv(sv) && !isREGEXP(sv) {
        unsafe {(*sv).sv_u.svu_pv}
    } else {
        std::ptr::null()
    }
}


fn sv_has_pv(sv: *const libperl_sys::sv) -> bool {
    match SvTYPE(sv) {
        svtype::SVt_PV | 
        svtype::SVt_INVLIST |
        svtype::SVt_REGEXP => true,
        svtype::SVt_PVGV | svtype::SVt_PVLV => {
            use libperl_sys::{SVp_POK, SVpgv_GP};
            (SvFLAGS(sv) & (SVp_POK|SVpgv_GP)) != SVpgv_GP
        },
        svtype::SVt_PVMG => {
            use libperl_sys::{SVf_POK, SVp_POK};
            (SvFLAGS(sv) & (SVf_POK|SVp_POK)) == (SVf_POK|SVp_POK)
        },
        // SVt_PVIO => {
        //     // IoFLAGS(sv) & IOf_FAKE_DIRP
        // },
        _ => false,
    }
}

pub fn isREGEXP(sv: *const libperl_sys::sv) -> bool {
    use libperl_sys::{SVTYPEMASK, SVpgv_GP, SVf_FAKE, svtype::SVt_PVLV};
    SvTYPE(sv) == svtype::SVt_REGEXP || {
        (SvFLAGS(sv) & (SVTYPEMASK|SVpgv_GP|SVf_FAKE))
            == (SVt_PVLV as u32|SVf_FAKE)
    }
}

fn sv_extract_ivuv(sv: *const libperl_sys::sv) -> Option<IVUV> {
    use libperl_sys::SVf_IVisUV;
    if !sv_has_ivuv(sv) {
        None
    } else if (SvFLAGS(sv) & SVf_IVisUV) != 0  {
        let xpvuv = (unsafe {(*sv).sv_any}) as *const libperl_sys::xpvuv;
        Some(IVUV::UV((unsafe {(*xpvuv).xuv_u.xivu_uv}) as usize))
    } else {
        let xpviv = (unsafe {(*sv).sv_any}) as *const libperl_sys::xpviv;
        Some(IVUV::IV((unsafe {(*xpviv).xiv_u.xivu_iv}) as isize))
    }
}

fn sv_has_ivuv(sv: *const libperl_sys::sv) -> bool {
    match SvTYPE(sv) {
        svtype::SVt_IV => !SvROK(sv),
        svtype::SVt_PVIV | svtype::SVt_PVNV => true,
        svtype::SVt_PVMG
            => // XXX: !isGV_with_GP(sv) && !SvVALID(sv))
            false,
        _ => false,
    }
}

fn sv_extract_nv(sv: *const libperl_sys::sv) -> Option<f64> {
    if !sv_has_nv(sv) {
        None
    } else {
        let xpvnv = (unsafe {(*sv).sv_any}) as *const libperl_sys::xpvnv;
        Some(unsafe {(*xpvnv).xnv_u.xnv_nv})
    }
}

fn sv_has_nv(sv: *const libperl_sys::sv) -> bool {
    match SvTYPE(sv) {
        svtype::SVt_NV => true,
        svtype::SVt_PVIV | svtype::SVt_PVNV => true,
        svtype::SVt_PVMG
            => // XXX: !isGV_with_GP(sv) && !SvVALID(sv))
            false,
        _ => false,
    }    
}

pub fn SvTYPE(sv: *const libperl_sys::sv) -> svtype {
    let svt = svtype_raw(sv);
    unsafe {*(&svt as *const u32 as *const svtype)}
}
pub fn svtype_raw(sv: *const libperl_sys::sv) -> u32 {
    SvFLAGS(sv) & libperl_sys::SVTYPEMASK
}

pub fn SvROK(sv: *const libperl_sys::sv) -> bool {
    (SvFLAGS(sv) & libperl_sys::SVf_ROK) != 0
}

pub fn SvRV<'a>(sv: *const libperl_sys::sv) -> Option<&'a libperl_sys::sv> {
    if SvROK(sv) {
        let s = unsafe {(*sv).sv_u.svu_rv};
        unsafe {s.as_ref()}
    } else {
        None
    }
}

pub fn SvOOK(sv: *const SV) -> bool {
    if sv.is_null() {
        false
    } else {
        (SvFLAGS(sv) & libperl_sys::SVf_OOK) != 0
    }
}

pub fn SvFLAGS(sv: *const libperl_sys::sv) -> u32 {
    assert_ne!(sv, std::ptr::null_mut());
    unsafe {(*sv).sv_flags}
}
