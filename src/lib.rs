extern crate libperl_sys;

pub mod perl;

#[cfg(test)]
mod tests {
    use super::perl::*;

    #[test]
    fn it_works() {
        let perl = Perl::new();
    }
}
