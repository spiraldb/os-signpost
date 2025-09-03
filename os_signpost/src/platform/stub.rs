//! Stub implementation for non-Apple platforms (no-op signpost operations)

#![allow(missing_docs)]

use std::ffi::CStr;

pub type SignpostType = u8;

pub const SIGNPOST_TYPE_EVENT: SignpostType = 0;
pub const SIGNPOST_TYPE_INTERVAL_BEGIN: SignpostType = 1;
pub const SIGNPOST_TYPE_INTERVAL_END: SignpostType = 2;

/// No-op log handle for non-Apple platforms
#[derive(Debug)]
#[allow(dead_code)]
pub struct LogHandle {
    subsystem: std::ffi::CString,
    category: &'static CStr,
}

pub fn os_log_create(subsystem: &CStr, category: &'static CStr) -> LogHandle {
    LogHandle {
        subsystem: subsystem.to_owned(),
        category,
    }
}

pub fn os_signpost_enabled(_log: &LogHandle) -> bool {
    false
}

pub fn os_signpost_id_generate(_log: &LogHandle) -> u64 {
    0
}

pub fn os_signpost_id_make_with_pointer(_log: &LogHandle, ptr: *const std::ffi::c_void) -> u64 {
    ptr as u64
}

pub fn emit_signpost(
    _log: &LogHandle,
    _id: u64,
    _name: &str,
    _message: Option<&str>,
    _signpost_type: SignpostType,
) {
    // No-op on non-Apple platforms
}

pub const POINTS_OF_INTEREST: &CStr = c"points_of_interest";
pub const DYNAMIC_TRACING: &CStr = c"dynamic_tracing";
pub const DYNAMIC_STACK_TRACING: &CStr = c"dynamic_stack_tracing";
