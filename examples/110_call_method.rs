#[cfg(all(perl_useithreads,perlapi_ver26))]
use std::env;
#[cfg(all(perl_useithreads,perlapi_ver26))]
use libperl_rs::*;
#[cfg(all(perl_useithreads,perlapi_ver26))]
use libperl_sys::*;
#[cfg(all(perl_useithreads,perlapi_ver26))]
use std::convert::TryInto;

#[cfg(all(perl_useithreads,perlapi_ver26))]
mod eg;
#[cfg(all(perl_useithreads,perlapi_ver26))]
use eg::sv0::*;

#[cfg(all(perl_useithreads,perlapi_ver26))]
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
    
    match call_list_method(&mut perl, class_name, method_name, method_args) {
        Ok(ary) => {
            for item in ary {
                println!("{:?}", item);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }
}

#[cfg(all(perl_useithreads,perlapi_ver26))]
fn call_list_method(perl: &mut Perl, class_name: String, method_name: String, args: Vec<String>) -> Result<Vec<Sv>,String>
{

    let my_perl = perl.my_perl();

    // dSP
    let mut sp = my_perl.Istack_sp;

    // ENTER
    unsafe_perl_api!{Perl_push_scope(perl.my_perl)};

    // SAVETMPS
    unsafe_perl_api!{Perl_savetmps(perl.my_perl)};

    // PUSHMARK(SP)
    perl.pushmark(sp);
    
    // (... argument pushing ...)
    // EXTEND(SP, 1+method_args.len())
    sp = unsafe_perl_api!{Perl_stack_grow(perl.my_perl, sp, sp, (1 + args.len()).try_into().unwrap())};
    
    for s in [&[class_name], args.as_slice()].concat() {
        sp_push!(sp, perl.str2svpv_mortal(s.as_str()));
    }

    // PUTBACK
    my_perl.Istack_sp = sp;

    // call_method
    let cnt = unsafe_perl_api!{Perl_call_method(perl.my_perl, method_name.as_ptr() as *const i8, (G_METHOD_NAMED | G_LIST) as i32)};
    
    // SPAGAIN
    // sp = my_perl.Istack_sp;
    // (PUTBACK)

    let res = stack_extract(&perl, cnt);

    // FREETMPS
    perl.free_tmps();
    // LEAVE
    unsafe_perl_api!{Perl_pop_scope(perl.my_perl)};
    
    Ok(res)
}

#[cfg(all(perl_useithreads,perlapi_ver26))]
fn stack_extract(perl: &Perl, count: perl_stack_size_t) -> Vec<Sv> {
    let mut res = Vec::new();

    let mut src = unsafe {(*(perl.my_perl)).Istack_base.add(1)};

    for _i in 0..count {
        let sv = unsafe {*src};
        res.push(sv_extract(sv));
        src = unsafe {src.add(1)}
    }
    
    res
}

#[cfg(not(all(perl_useithreads,perlapi_ver26)))]
fn my_test() {
}

fn main() {
    my_test()
}
