pub mod perl_core;
pub use perl_core::*;



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let perl = perl_alloc();
    }
}
