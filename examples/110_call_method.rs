use std::env;
use libperl_rs::*;
use libperl_sys::*;
use std::convert::TryInto;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::sv0::*;

#[cfg(perl_useithreads)]
fn my_test() {
    
    let mut args = env::args().skip(1);
    let inc_path = args.next().expect("Include path is required");
    let class_name = args.next().expect("Class name is required");
    let method_name = args.next().expect("Method name is required");
    let method_args: Vec<String> = args.collect();
    
    let mut perl = Perl::new();
    perl.parse(&[
        "",
        format!("-I{}", inc_path).as_str(),
        format!("-M{}", class_name).as_str(),
        "-e0",
    ], &[]);
    
    if let Ok(ary) = call_list_method(&mut perl, class_name, method_name, method_args) {
        println!("Got result: {:?}", ary);
    }
}

#[cfg(perlapi_ver26)]
#[cfg(perl_useithreads)]
fn call_list_method(perl: &mut Perl, class_name: String, method_name: String, args: Vec<String>) -> Result<Vec<Sv>,String>
{

    let mut my_perl = unsafe {perl.my_perl.as_mut().unwrap()};

    // dSP
    let mut sp = my_perl.Istack_sp;

    // ENTER
    unsafe {
        perl_api!{Perl_push_scope(perl.my_perl)}
    };

    // SAVETMPS
    unsafe {
        perl_api!{Perl_savetmps(perl.my_perl)}
    };

    // PUSHMARK(SP)
    unsafe {
        my_perl.Imarkstack_ptr = my_perl.Imarkstack_ptr.add(1)
    };
    if my_perl.Imarkstack_ptr == my_perl.Imarkstack_max {
        unsafe {
            perl_api!{Perl_markstack_grow(perl.my_perl)}
        };
    }
    unsafe {
        *(my_perl.Imarkstack_ptr)
            = (sp as usize - my_perl.Istack_base as usize) as i32;
    }
    
    // (... argument pushing ...)
    // EXTEND(SP, 1+method_args.len())
    unsafe {
        sp = perl_api!{Perl_stack_grow(perl.my_perl, sp, sp, (1 + args.len()).try_into().unwrap())};
    }
    
    {
        let sv = perl.str2svpv_flags(class_name.as_str(), SVf_UTF8 | SVs_TEMP);
        unsafe {
            sp = sp.add(1);
            *sp = sv;
        }
    }

    for s in args {
        let sv = perl.str2svpv_flags(s.as_str(), SVf_UTF8 | SVs_TEMP);
        unsafe {
            sp = sp.add(1);
            *sp = sv;
        }
    }

    // PUTBACK
    my_perl.Istack_sp = sp;

    // call_method
    unsafe {
        let nm = method_name.as_ptr() as *const i8;
        perl_api!{Perl_call_method(perl.my_perl, nm, (G_METHOD_NAMED | G_ARRAY) as i32)}
    };

    // SPAGAIN
    // sp = my_perl.Istack_sp;
    // (PUTBACK)

    let res = stack_extract(&perl);

    // FREETMPS
    if my_perl.Itmps_ix > my_perl.Itmps_floor {
        unsafe {
            perl_api!{Perl_free_tmps(perl.my_perl)}
        }
    }
    // LEAVE
    unsafe {
        perl_api!{Perl_pop_scope(perl.my_perl)};
    }
    
    Ok(res)
}

#[cfg(perlapi_ver26)]
#[cfg(perl_useithreads)]
fn stack_extract(perl: &Perl) -> Vec<Sv> {
    let mut res = Vec::new();

    let mut src = unsafe {(*(perl.my_perl)).Istack_base.add(1)};
    let last = unsafe {*(perl.my_perl)}.Istack_sp;

    while src <= last {
        let sv = unsafe {*src};
        res.push(sv_extract(sv));
        src = unsafe {src.add(1)}
    }
    
    res
}

#[cfg(not(perl_useithreads))]
fn my_test() {
}

fn main() {
    my_test()
}
