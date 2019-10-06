use std::env;

use libperl_rs::perl::Perl;
use libperl_sys;

#[cfg(perl_useithreads)]
fn test() {
    let mut perl = Perl::new();

    perl.parse_env_args(env::args(), env::vars());
    
    let main_cv = unsafe {(*perl.my_perl)}.Imain_cv;
    print!("main_cv = {:#?}\n", main_cv);
    let xpvcv = unsafe {(*main_cv).sv_any};
    print!("xpvcv = {:#?}\n", xpvcv);
    let padlist = unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist};
    print!("padlist = {:#?}\n", padlist);
    let padnamelist_ptr = unsafe {
        *((*padlist).xpadl_arr.xpadlarr_alloc
          as *const *const libperl_sys::PADNAMELIST)
    };
    if let Some(padnamelist) = unsafe {padnamelist_ptr.as_ref()} {
        println!("padnamelist = {:?}", padnamelist);
        let mut ix: usize = 0;
        while ix < (padnamelist.xpadnl_fill as usize) {
            let padname = unsafe {(*(padnamelist.xpadnl_alloc.add(ix)))
                                  .as_ref()};
            println!("padname {} = {:?}"
                     , ix
                     , padname);
            ix += 1;
        }
    }
}

fn main() {
    test();
}
