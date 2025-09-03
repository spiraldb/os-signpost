// Platform-specific implementations
#[cfg(target_vendor = "apple")]
pub(crate) mod apple;
#[cfg(target_vendor = "apple")]
pub(crate) use apple::*;
#[cfg(target_vendor = "apple")]
pub use apple::{DYNAMIC_STACK_TRACING, DYNAMIC_TRACING, POINTS_OF_INTEREST};

#[cfg(not(target_vendor = "apple"))]
pub(crate) mod stub;
#[cfg(not(target_vendor = "apple"))]
pub(crate) use stub::*;
#[cfg(not(target_vendor = "apple"))]
pub use stub::{DYNAMIC_STACK_TRACING, DYNAMIC_TRACING, POINTS_OF_INTEREST};
