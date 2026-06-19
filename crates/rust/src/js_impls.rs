#[cfg(feature = "quickjs")]
#[path = "js_impls/quickjs.rs"]
mod runtime;

#[cfg(all(feature = "boajs", not(feature = "quickjs")))]
#[path = "js_impls/boa.rs"]
mod runtime;

pub(crate) use runtime::*;
