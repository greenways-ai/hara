#[path = "task/promise.rs"]
pub mod promise;

pub use promise::{
    LocalPromiseProvider, Promise, PromiseProvider, PromiseRejection, PromiseState,
};
