//! Signpost implementation using Apple's os_signpost API

use std::{
    ffi::{c_void, CStr},
    sync::{
        atomic::{AtomicPtr, Ordering},
        Once,
    },
};

pub mod sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    // Provide compatibility constants with standard names
    pub use self::{
        os_signpost_type_t_OS_SIGNPOST_EVENT as SIGNPOST_TYPE_EVENT,
        os_signpost_type_t_OS_SIGNPOST_INTERVAL_BEGIN as SIGNPOST_TYPE_INTERVAL_BEGIN,
        os_signpost_type_t_OS_SIGNPOST_INTERVAL_END as SIGNPOST_TYPE_INTERVAL_END,
    };
}

pub type SignpostType = sys::os_signpost_type_t;

/// A signpost event marking a single point in time
pub const SIGNPOST_TYPE_EVENT: SignpostType = sys::SIGNPOST_TYPE_EVENT;
/// The beginning of a signpost interval
pub const SIGNPOST_TYPE_INTERVAL_BEGIN: SignpostType = sys::SIGNPOST_TYPE_INTERVAL_BEGIN;
/// The end of a signpost interval
pub const SIGNPOST_TYPE_INTERVAL_END: SignpostType = sys::SIGNPOST_TYPE_INTERVAL_END;

/// Log handle that lazily initializes the actual os_log_t
pub struct LogHandle {
    subsystem: std::ffi::CString,
    category: &'static CStr,
    handle: AtomicPtr<sys::os_log_s>,
    init: Once,
}

impl std::fmt::Debug for LogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogHandle")
            .field("subsystem", &self.subsystem)
            .field("category", &self.category)
            .finish()
    }
}

impl LogHandle {
    fn get(&self) -> sys::os_log_t {
        self.init.call_once(|| {
            let handle =
                unsafe { sys::os_log_create(self.subsystem.as_ptr(), self.category.as_ptr()) };
            self.handle.store(handle, Ordering::SeqCst);
        });
        self.handle.load(Ordering::SeqCst)
    }
}

pub fn os_log_create(subsystem: &CStr, category: &'static CStr) -> LogHandle {
    LogHandle {
        subsystem: subsystem.to_owned(),
        category,
        handle: AtomicPtr::new(std::ptr::null_mut()),
        init: Once::new(),
    }
}

pub fn os_signpost_enabled(log: &LogHandle) -> bool {
    unsafe { sys::os_signpost_enabled(log.get()) }
}

pub fn os_signpost_id_generate(log: &LogHandle) -> u64 {
    unsafe { sys::os_signpost_id_generate(log.get()) }
}

pub fn os_signpost_id_make_with_pointer(log: &LogHandle, ptr: *const c_void) -> u64 {
    unsafe { sys::os_signpost_id_make_with_pointer(log.get(), ptr) }
}

pub fn emit_signpost(
    log: &LogHandle,
    id: u64,
    name: &str,
    message: Option<&str>,
    signpost_type: SignpostType,
) {
    if !os_signpost_enabled(log) {
        return;
    }

    let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
    let message_cstr = message.map(|msg| std::ffi::CString::new(msg).unwrap_or_default());

    // Dart SDK for reference on how to set up the format buffer:
    // https://github.com/dart-lang/sdk/blob/3e2d3bc77fa8bb5139b869e9b3a5357b5487df18/runtime/vm/timeline_macos.cc#L34C1-L34C34
    const FORMAT_BUFFER_LEN: usize = 64;

    #[repr(align(16))]
    struct AlignedBuffer {
        data: [u8; FORMAT_BUFFER_LEN],
    }

    static FORMAT_BUFFER: AlignedBuffer = AlignedBuffer {
        data: [0; FORMAT_BUFFER_LEN],
    };

    unsafe {
        sys::_os_signpost_emit_with_name_impl(
            (&raw mut sys::__dso_handle) as *mut usize as *mut c_void,
            log.get(),
            signpost_type,
            id,
            name_cstr.as_ptr(),
            message_cstr
                .as_ref()
                .map(|msg| msg.as_ptr())
                .unwrap_or(std::ptr::null()),
            &FORMAT_BUFFER.data as *const _ as *mut u8,
            FORMAT_BUFFER_LEN as u32,
        );
    }
}

/// Provide this value as the category to os_log_create to indicate that
/// signposts on the resulting log handle provide high-level events that can be
/// used to orient a developer looking at performance data. These will be
/// displayed by default by performance tools like Instruments.app.
pub const POINTS_OF_INTEREST: &CStr =
    unsafe { &*(sys::OS_LOG_CATEGORY_POINTS_OF_INTEREST as *const [u8] as *const CStr) };

/// Use this category for signposts that should be disabled by default to reduce runtime
/// overhead. These signposts will only be active when a performance tool like Instruments
/// is actively recording, providing detailed insights without impacting normal operation.
pub const DYNAMIC_TRACING: &CStr =
    unsafe { &*(sys::OS_LOG_CATEGORY_DYNAMIC_TRACING as *const [u8] as *const CStr) };

/// Use this category for signposts that should capture user backtraces. This behavior is
/// more expensive than regular signposts, so it will only be active when a performance
/// tool like Instruments is actively recording.
pub const DYNAMIC_STACK_TRACING: &CStr =
    unsafe { &*(sys::OS_LOG_CATEGORY_DYNAMIC_STACK_TRACING as *const [u8] as *const CStr) };
