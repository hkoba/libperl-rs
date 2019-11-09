use std::env;

use libperl_rs::Perl;

mod eg;

fn main() {
    my_test();
}

#[cfg(perlapi_ver22)]
fn my_test() {

    let mut perl = Perl::new();

    perl.parse_env_args(env::args(), env::vars());
    
    let main_cv = perl.get_main_cv();
    print!("main_cv = {:?}\n", unsafe {*main_cv});

    let xpvcv = unsafe {(*main_cv).sv_any};
    print!("xpvcv = {:?}\n", unsafe {*xpvcv});

    let padlist = unsafe {(*xpvcv).xcv_padlist_u.xcv_padlist};
    print!("padlist = {:?}\n", unsafe {*padlist});

    let padnamelist_ptr = eg::pad0::fetch_padnamelist(padlist);
    if let Some(padnamelist) = unsafe {padnamelist_ptr.as_ref()} {
        println!("padnamelist = {:?}", padnamelist);
        let mut ix: usize = 0;
        while ix < (padnamelist.xpadnl_fill as usize) {
            let padname = unsafe {(*(padnamelist.xpadnl_alloc.add(ix)))
                                  .as_ref()}.unwrap();
            println!("padname {} = var{{name: {:?}}}, type: {:?}"
                     , ix
                     , eg::pad0::perl__PadnamePV(padname)
                     , eg::pad0::perl__PadnameTYPE(padname)
            );
            ix += 1;
        }
    }
}

