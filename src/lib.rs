pub mod database;
pub mod scanner;
pub mod util;
pub mod web;

pub(crate) fn is_debug() -> bool {
    cfg!(debug_assertions)
}
