#[cfg(perlapi_ver22)]
use std::env;

// cargo run --example 102_padname_type -- -le 'my main $x; my $y'

#[cfg(perlapi_ver22)]
use libperl_proto0::Perl;

mod eg;

fn main() {
    my_test();
}

#[cfg(not(perlapi_ver22))]
fn my_test() {
    // padname API (struct padname / padnamelist) は perl 5.22 生まれ
    println!("SKIP: this example requires perl >= 5.22");
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
                     , eg::pad0::PadnamePV(padname)
                     , eg::pad0::PadnameTYPE(padname)
            );
            ix += 1;
        }
    }
}

