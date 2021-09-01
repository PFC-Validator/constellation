pub mod actor;
pub(crate) mod commands;
mod errors;
mod handler;
mod task;

pub use task::run;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
