//! utaformatix-rs is a library that allows you to use UtaFormatix in Rust.
//!
//! # Feature Flags
//!
//! - `quickjs`: Use QuickJS as the JavaScript engine. (default)
//! - `boajs`: Use BoaJS as the JavaScript engine.
//!
//! In most cases, `quickjs` is recommended for better performance and compatibility. However,
//! if you want pure-rust binary, you can use `boajs` instead. Note that `boajs` can cause
//! some issues, such as stack overflow errors when parsing large files, and is much slower than
//! `quickjs`.
#[cfg(not(any(feature = "boajs", feature = "quickjs")))]
compile_error!("either feature `boajs` or `quickjs` must be enabled");

pub mod base;
mod error;
#[cfg(all(feature = "boajs", not(feature = "quickjs")))]
mod job_queue;
mod js_impls;
mod model;
mod process;
mod project;

pub use error::*;
pub use model::*;
pub use project::*;
