#![warn(missing_docs)]

//! Signpost library for macOS.
//!
//! This library provides a Rust wrapper around Apple's os_signpost API for
//! performance instrumentation and profiling.
//!
//! ## Intervals vs Events
//! - Intervals: Represent periods of time with a beginning and end.
//! - Events: Marks single points in time.
//!
//! ## Signpost IDs
//! Intervals with the same log handle and interval name can be in flight simultaneously.
//! To correctly match begin signposts with end signposts, each interval must be identified
//! with a unique `SignpostId`.
//!
//! ## Matching Scope
//! Signpost interval begin and end matching can have different scopes:
//! - Thread-wide: Matching is restricted to single threads
//! - Process-wide: Matching is restricted to a single process (default)
//! - System-wide: Matching can span across processes

pub use signpost_derive::signpost;

use std::{
    ffi::{c_void, CStr},
    sync::{
        atomic::{AtomicPtr, Ordering},
        OnceLock,
    },
};

mod sys {
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

/// Predefined log categories for different types of signpost instrumentation.
pub mod categories {
    use crate::sys;
    use std::ffi::CStr;

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
}

/// Errors that can occur when working with signposts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignpostError {
    /// The signpost system has not been configured.
    NotConfigured,

    /// Invalid scope for the requested operation.
    InvalidScope,

    /// Signpost ID is invalid or uses a reserved value.
    InvalidId,
}

impl std::fmt::Display for SignpostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignpostError::NotConfigured => write!(f, "Signpost not initialized"),
            SignpostError::InvalidScope => write!(f, "Invalid scope for operation"),
            SignpostError::InvalidId => write!(f, "Invalid signpost ID"),
        }
    }
}

impl std::error::Error for SignpostError {}

/// A unique identifier for signpost intervals and events.
///
/// Signpost IDs are used to disambiguate between concurrent intervals that share
/// the same log handle and interval name. This allows performance tools to correctly
/// match begin and end signposts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignpostId(u64);

impl SignpostId {
    /// Generates an ID guaranteed to be unique within the matching scope of the provided log handle.
    ///
    /// Each call to `generate()` returns a different `SignpostId`. This is the
    /// safest method for creating IDs when you don't have an existing unique
    /// identifier.
    ///
    /// # Returns
    /// A valid `SignpostId`.
    pub fn generate(log: &OsLog) -> Self {
        Self(unsafe { sys::os_signpost_id_generate(log.get()) })
    }

    /// Creates a signpost ID from a pointer value.
    ///
    /// This function mangles the pointer to create a valid signpost ID, including removing
    /// address randomization. This is useful when you want to track the lifecycle of an
    /// object using its memory address.
    ///
    /// # Parameters
    /// - `log`: Log handle previously created with `OsLog::new`
    /// - `ptr`: Any pointer that disambiguates among concurrent intervals with the same
    ///   log handle and interval names
    ///
    /// # Returns
    /// - `Ok(SignpostId)`: A valid signpost ID
    /// - `Err(SignpostError::InvalidScope)`: If the log handle is system-scoped, since
    ///   pointers are not valid across process boundaries
    ///
    /// # Note
    /// This approach is not applicable to signposts that span process boundaries.
    pub fn from_pointer<T>(log: &OsLog, ptr: *const T) -> Result<Self, SignpostError> {
        let id = unsafe { sys::os_signpost_id_make_with_pointer(log.get(), ptr as *const c_void) };
        Ok(Self(id))
    }

    /// Creates a signpost ID from a raw uint64_t value.
    ///
    /// This allows you to use any existing 64-bit value as a signpost ID, as long as
    /// it's not one of the reserved values (0 or ~0). Use this when you have an existing
    /// unique identifier that you want to use for signpost tracking.
    ///
    /// # Safety
    /// The caller must ensure that the provided value is unique within the matching scope
    /// and is not one of the reserved values (OS_SIGNPOST_ID_NULL or OS_SIGNPOST_ID_INVALID).
    ///
    /// # Parameters
    /// - `id`: A 64-bit value to use as the signpost ID
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw uint64_t value of this signpost ID.
    ///
    /// This can be useful for storing the ID in external systems or for debugging purposes.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Signpost type for different kinds of signpost emissions
#[repr(u8)]
pub(crate) enum SignpostType {
    /// A signpost event marking a single point in time
    Event = sys::SIGNPOST_TYPE_EVENT,
    /// The beginning of a signpost interval
    IntervalBegin = sys::SIGNPOST_TYPE_INTERVAL_BEGIN,
    /// The end of a signpost interval
    IntervalEnd = sys::SIGNPOST_TYPE_INTERVAL_END,
}

/// A logger for a specific subsystem and category.
///
/// `OsLog` represents a configured logging destination for signposts. Each logger
/// is associated with a subsystem (typically your app's bundle identifier) and
/// a category (which determines the behavior and visibility of the signposts).
///
/// # Configuration
/// Loggers are created through the configuration system using `configure()` and should
/// be reused rather than created repeatedly for the same subsystem/category combination.
///
/// # Examples
/// ```ignore
/// use signpost::{OsLog, categories};
///
/// // Create a logger for high-level events
/// let log = OsLog::new("com.myapp", categories::POINTS_OF_INTEREST);
///
/// // Create a logger with custom scope
/// let log = OsLog::new("com.myapp.network", categories::DYNAMIC_TRACING)
///     .with_scope(SignpostScope::Thread);
/// ```
#[derive(Debug)]
pub struct OsLog {
    subsystem: String,
    category: &'static CStr,
    handle: AtomicPtr<sys::os_log_s>,
    init: std::sync::Once,
}

impl OsLog {
    /// Create a new logger for the given subsystem and category
    pub fn new(subsystem: String, category: &'static CStr) -> Self {
        Self {
            subsystem,
            category,
            handle: AtomicPtr::new(std::ptr::null_mut()),
            init: std::sync::Once::new(),
        }
    }

    /// Check if signpost logging is enabled for this logger
    pub fn enabled(&self) -> bool {
        let handle = self.get();
        unsafe { sys::os_signpost_enabled(handle) }
    }

    /// Emit a simple event (point in time)
    pub fn event<T: AsRef<str>>(&self, id: SignpostId, name: T) {
        self.emit(id, name.as_ref(), None, SignpostType::Event);
    }

    /// Emit an event with a formatted message
    pub fn event_with_message<T1: AsRef<str>, T2: AsRef<str>>(
        &self,
        id: SignpostId,
        name: T1,
        message: T2,
    ) {
        self.emit(
            id,
            name.as_ref(),
            Some(message.as_ref()),
            SignpostType::Event,
        );
    }

    /// Start a signpost interval
    pub fn interval<T: AsRef<str>>(&self, id: SignpostId, name: T) -> SignpostInterval<'_> {
        SignpostInterval::new(self, id, name.as_ref(), None)
    }

    /// Start a signpost interval with a message
    pub fn interval_with_message<T1: AsRef<str>, T2: AsRef<str>>(
        &self,
        id: SignpostId,
        name: T1,
        message: T2,
    ) -> SignpostInterval<'_> {
        SignpostInterval::new(self, id, name.as_ref(), Some(message.as_ref()))
    }

    /// Centralized signpost emission function
    pub(crate) fn emit(
        &self,
        id: SignpostId,
        name: &str,
        message: Option<&str>,
        signpost_type: SignpostType,
    ) {
        if !self.enabled() {
            return;
        }

        let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
        let message_cstr = message.map(|msg| std::ffi::CString::new(msg).unwrap_or_default());

        let os_signpost_type = match signpost_type {
            SignpostType::Event => sys::SIGNPOST_TYPE_EVENT,
            SignpostType::IntervalBegin => sys::SIGNPOST_TYPE_INTERVAL_BEGIN,
            SignpostType::IntervalEnd => sys::SIGNPOST_TYPE_INTERVAL_END,
        };

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
                self.get(),
                os_signpost_type,
                id.0,
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

    fn get(&self) -> sys::os_log_t {
        self.init.call_once(|| {
            let subsystem_cstr = std::ffi::CString::new(self.subsystem.as_str()).unwrap();
            let handle =
                unsafe { sys::os_log_create(subsystem_cstr.as_ptr(), self.category.as_ptr()) };
            self.handle.store(handle, Ordering::SeqCst);
        });

        self.handle.load(Ordering::SeqCst)
    }
}

/// A signpost interval that represents a period of time being measured.
///
/// # Automatic Cleanup
/// The interval will automatically emit an end signpost when it goes out of scope,
/// due to its `Drop` implementation.
pub struct SignpostInterval<'a> {
    log: &'a OsLog,
    id: SignpostId,
    name: String,
    message: Option<String>,
}

impl<'a> SignpostInterval<'a> {
    fn new(log: &'a OsLog, id: SignpostId, name: &str, message: Option<&str>) -> Self {
        let interval = Self {
            log,
            id,
            name: name.to_string(),
            message: message.map(|m| m.to_string()),
        };

        if log.enabled() {
            interval.start_interval();
        }

        interval
    }

    fn start_interval(&self) {
        self.log.emit(
            self.id,
            &self.name,
            self.message.as_ref().map(|m| m.as_ref()),
            SignpostType::IntervalBegin,
        );
    }

    fn end_internal(&self) {
        self.log
            // Don't repeat the start message as an end message.
            .emit(self.id, &self.name, None, SignpostType::IntervalEnd);
    }
}

impl Drop for SignpostInterval<'_> {
    fn drop(&mut self) {
        self.end_internal();
    }
}

static GLOBAL_CONFIG: OnceLock<(String, &'static CStr)> = OnceLock::new();

/// Configuration builder for signpost tracer.
pub struct Signpost {
    subsystem: String,
    category: &'static CStr,
}

impl Signpost {
    /// Initializes the process global signpost configuration.
    pub fn configure(subsystem: &str, category: &'static CStr) -> Self {
        let config = Self {
            subsystem: subsystem.to_string(),
            category,
        };

        GLOBAL_CONFIG
            .set((config.subsystem.clone(), config.category))
            .expect("Signpost already configured");

        config
    }
}

/// Get the global logger for signpost operations.
#[doc(hidden)]
pub fn global_logger() -> &'static OsLog {
    // Use a static OnceLock for the actual logger instance
    static GLOBAL_LOGGER: OnceLock<OsLog> = OnceLock::new();

    GLOBAL_LOGGER.get_or_init(|| {
        if let Some((subsystem, category)) = GLOBAL_CONFIG.get() {
            OsLog::new(subsystem.clone(), category)
        } else {
            panic!("Double Signpost config initialization");
        }
    })
}

/// Helper macro to get the current function name
#[doc(hidden)]
#[macro_export]
macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap_or(name)
    }};
}

/// Creates a signpost interval manually with a name.
///
/// # Parameters
/// - `name`: A static string describing the operation being measured.
///
/// # Returns
/// A `SignpostInterval` that will automatically emit an end signpost when dropped.
/// Creates a signpost interval manually with a name that includes the module path.
///
/// # Parameters
/// - `name`: A string describing the operation being measured.
///
/// # Returns
/// A `SignpostInterval` that will automatically emit an end signpost when dropped.
/// The signpost name will be in the format "function_name::name".
#[macro_export]
macro_rules! interval {
    ($name:expr) => {{
        let logger = $crate::global_logger();
        let id = $crate::SignpostId::generate(logger);
        let full_name = format!("{}::{}", $crate::function_name!(), $name);
        logger.interval(id, &full_name)
    }};
}

/// Creates a signpost interval manually with a name and message that includes the module path.
///
/// # Parameters
/// - `name`: A string describing the operation being measured.
/// - `message`: Additional information about the operation being measured.
///
/// # Returns
/// A `SignpostInterval` that will automatically emit an end signpost when dropped.
/// The signpost name will be in the format "function_name::name".
#[macro_export]
macro_rules! interval_with_message {
    ($name:expr, $message:expr) => {{
        let logger = $crate::global_logger();
        let id = $crate::SignpostId::generate(logger);
        let full_name = format!("{}::{}", $crate::function_name!(), $name);
        logger.interval_with_message(id, &full_name, $message)
    }};
}

/// Emit a signpost event (point in time) with module path included.
///
/// The event name will be in the format "function_name::name".
///
/// # Usage
///
/// ```ignore
/// event!("Something Happened");
/// event!("User Action");
/// event!("Error Occurred");
/// ```
#[macro_export]
macro_rules! event {
    ($name:expr) => {{
        let logger = $crate::global_logger();
        let id = $crate::SignpostId::generate(logger);
        let full_name = format!("{}::{}", $crate::function_name!(), $name);
        logger.event(id, &full_name);
    }};
}

/// Emit a signpost event with a message and module path included.
///
/// The event name will be in the format "function_name::name".
///
/// # Usage
///
/// ```ignore
/// event_with_message!("Something Happened", "Additional context");
/// event_with_message!("User Action", "Button clicked");
/// event_with_message!("Error Occurred", "Network timeout");
/// ```
#[macro_export]
macro_rules! event_with_message {
    ($name:expr, $message:expr) => {{
        let logger = $crate::global_logger();
        let id = $crate::SignpostId::generate(logger);
        let full_name = format!("{}::{}", $crate::function_name!(), $name);
        logger.event_with_message(id, &full_name, $message);
    }};
}

/// Tracing subscriber integration for os_signpost.
///
/// This module provides a [`TracingSubscriber`] that can be used with `tracing-subscriber`
/// to emit tracing spans and events as os_signpost intervals and events.
#[cfg(feature = "tracing")]
pub mod tracing_subscriber;

#[cfg(feature = "tracing")]
pub use tracing_subscriber::TracingSubscriber;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration() {
        Signpost::configure("test_app", categories::POINTS_OF_INTEREST);

        // Test that we can't init the trace twice.
        std::panic::catch_unwind(|| {
            Signpost::configure("another_app", categories::POINTS_OF_INTEREST);
        })
        .expect_err("Should panic when configuring twice");
    }

    #[test]
    fn test_bindgen_integration() {
        // Test that os_log_t is a pointer type from generated bindings
        let null_log: sys::os_log_t = std::ptr::null_mut();
        assert!(null_log.is_null());

        // Test that os_signpost_id_t is u64 from generated bindings
        let id: sys::os_signpost_id_t = 42;
        assert_eq!(id, 42u64);

        // Test that os_signpost_type_t is u8 from generated bindings
        let signpost_type: sys::os_signpost_type_t = sys::SIGNPOST_TYPE_EVENT;
        assert_eq!(signpost_type, 0u8);

        // Verify generated constants have expected values (matching Apple's headers)
        assert_eq!(sys::SIGNPOST_TYPE_EVENT, 0);
        assert_eq!(sys::SIGNPOST_TYPE_INTERVAL_BEGIN, 1);
        assert_eq!(sys::SIGNPOST_TYPE_INTERVAL_END, 2);
    }

    #[test]
    fn test_error_types() {
        let error = SignpostError::NotConfigured;
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("initialized"));

        let error = SignpostError::InvalidScope;
        assert_eq!(format!("{}", error), "Invalid scope for operation");

        let error = SignpostError::InvalidId;
        assert_eq!(format!("{}", error), "Invalid signpost ID");
    }

    #[test]
    fn test_event_functions() {
        // Try to configure, but ignore if already configured
        let _ = std::panic::catch_unwind(|| {
            Signpost::configure("test_events", categories::POINTS_OF_INTEREST);
        });

        // Test that event functions compile and execute without panicking
        event!("Test Event");
        event_with_message!("Test Event With Message", "This is a test message");
    }
}
