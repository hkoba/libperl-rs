use std::env;

// cargo run --example 102_padname_type -- -le 'my main $x; my $y'

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
    // print!("main_cv = {:?}\n", unsafe {*main_cv});

    if let Some(padnamelist) = eg::pad0::cv_padnamelist(main_cv) {
        println!("padnamelist = {:?}", padnamelist);
        let mut ix: usize = 0;
        while ix < (padnamelist.xpadnl_fill as usize) {
            let padname = eg::pad0::padnamelist_nth(padnamelist, ix).unwrap();
            println!("padname {} = var{{name: {:?}}}, type: {:?}"
                     , ix
                     , eg::pad0::perl__PadnamePV(padname)
                     , eg::pad0::perl__PadnameTYPE(padname)
            );
            ix += 1;
        }
    }
}

