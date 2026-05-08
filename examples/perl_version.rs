//! Quick demo: print the Perl version libperl-sys was built against.

fn main() {
    println!("PERL_VERSION  = {}", libperl_sys::PERL_VERSION);
    println!("PERL_THREADED = {}", libperl_sys::PERL_THREADED);
    println!("PERL_ARCHNAME = {}", libperl_sys::PERL_ARCHNAME);
}
