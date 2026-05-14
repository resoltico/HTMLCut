#[cfg(any(test, not(feature = "http-client")))]
mod disabled;
#[cfg(feature = "http-client")]
mod enabled;

#[cfg(not(feature = "http-client"))]
pub(crate) use disabled::*;
#[cfg(feature = "http-client")]
pub(crate) use enabled::*;
