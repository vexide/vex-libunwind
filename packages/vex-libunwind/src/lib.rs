//! Idiomatic Rust bindings for LLVM `libunwind` on VEX V5 robots
//!
//! ```no_run
//! # use vex_libunwind::*;
//! let context = UnwindContext::new().unwrap();
//! let mut cursor = UnwindCursor::new(&context);
//!
//! loop {
//!     // Print instruction pointer (i.e. "program counter")
//!     println!("{:?}", cursor.register(registers::UNW_REG_IP));
//!
//!     if !cursor.step().unwrap() {
//!         // End of stack reached
//!         break;
//!     }
//! }
//! ```
#![no_std]

use core::{cell::RefCell, ffi::CStr, fmt::Debug, mem::MaybeUninit};

use snafu::Snafu;
pub use vex_libunwind_sys::registers;
use vex_libunwind_sys::*;

/// An error that can occur during unwinding.
#[derive(Debug, Snafu)]
pub enum UnwindError {
    /// Unspecified/general error.
    Unspecified,
    /// Out of memory
    NoMemory,
    /// Invalid register
    BadRegister,
    /// Attempt to write to a read-only register
    WriteToReadOnlyRegister,
    /// Stop unwinding
    StopUnwinding,
    /// Invalid instruction pointer
    InvalidIP,
    /// Bad frame
    BadFrame,
    /// Unsupported operation or bad value
    BadValue,
    /// Unwind info has unsupported version
    BadVersion,
    /// No unwind info found
    NoInfo,
    /// An error with an unknown error code occured
    #[snafu(display("libunwind error {code}"))]
    Unknown {
        /// The error's code
        code: uw_error_t,
    },
}

impl UnwindError {
    /// Creates a `Result` that is `Ok` if the error code represents a success
    /// and `Err` if it represents an error.
    pub const fn from_code(code: uw_error_t) -> Result<uw_error_t, UnwindError> {
        if code >= error::UNW_ESUCCESS {
            Ok(code)
        } else {
            Err(match code {
                error::UNW_EUNSPEC => UnwindError::Unspecified,
                error::UNW_ENOMEM => UnwindError::NoMemory,
                error::UNW_EBADREG => UnwindError::BadRegister,
                error::UNW_EREADONLYREG => UnwindError::WriteToReadOnlyRegister,
                error::UNW_ESTOPUNWIND => UnwindError::StopUnwinding,
                error::UNW_EINVALIDIP => UnwindError::InvalidIP,
                error::UNW_EBADFRAME => UnwindError::BadFrame,
                error::UNW_EINVAL => UnwindError::BadValue,
                error::UNW_EBADVERSION => UnwindError::BadVersion,
                error::UNW_ENOINFO => UnwindError::NoInfo,
                code => UnwindError::Unknown { code },
            })
        }
    }
}

/// Holds a snapshot of the state of the CPU's registers at a certain point of
/// execution.
#[derive(Clone)]
pub struct UnwindContext {
    // RefCells are used because FFI functions that do not mutate take mutable pointers for some
    // reason.
    inner: RefCell<unw_context_t>,
}

impl UnwindContext {
    /// Creates a snapshot of the current CPU state, allowing for local
    /// unwinding.
    #[inline(always)] // Inlining keeps this function from appearing in backtraces
    pub fn new() -> Result<Self, UnwindError> {
        let mut inner = MaybeUninit::<unw_context_t>::uninit();
        // SAFETY: `unw_getcontext` initializes the context struct.
        let inner = unsafe {
            UnwindError::from_code(unw_getcontext(inner.as_mut_ptr()))?;
            inner.assume_init()
        };
        Ok(Self {
            inner: RefCell::new(inner),
        })
    }

    /// Returns the underlying `libunwind` object.
    pub fn as_mut_ptr(&mut self) -> *mut unw_context_t {
        &mut *self.inner.get_mut()
    }
}

impl Debug for UnwindContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UnwindContext").finish_non_exhaustive()
    }
}

/// A cursor that can move up the call chain and gather information about stack
/// frames.
///
/// This struct provides functionality for reading and writing the CPU registers
/// that were preserved in stack frames, as well as moving "up" the call chain
/// to previous function calls.
#[derive(Clone)]
pub struct UnwindCursor {
    inner: RefCell<unw_cursor_t>,
}

impl UnwindCursor {
    /// Initializes a cursor for local unwinding using the state captured by the
    /// given [`UnwindContext`].
    pub fn new(context: &UnwindContext) -> Result<Self, UnwindError> {
        let mut cursor = MaybeUninit::<unw_cursor_t>::uninit();
        // SAFETY: `unw_init_local` initializes the cursor struct. A reference to
        // `context` is not stored in the cursor.
        let cursor = unsafe {
            UnwindError::from_code(unw_init_local(
                cursor.as_mut_ptr(),
                &mut *context.inner.borrow_mut(),
            ))?;
            cursor.assume_init()
        };
        Ok(Self {
            inner: RefCell::new(cursor),
        })
    }

    /// Advances to the next (older) frame of the call chain.
    ///
    /// Returns true if was another frame to step to or false
    /// if the cursor has reached the end.
    ///
    /// # Errors
    ///
    /// This function may return one of the following errors:
    ///
    /// - [`UnwindError::Unspecified`] if an unspecified error occurred
    /// - [`UnwindError::NoInfo`] if `libunwind` was unable to locate the
    ///   required unwind info
    /// - [`UnwindError::BadVersion`] if the unwind info has an unsupported
    ///   version or format
    /// - [`UnwindError::InvalidIP`] if the instruction pointer of the next
    ///   frame is invalid
    /// - [`UnwindError::BadFrame`] if the next frame is invalid
    pub fn step(&mut self) -> Result<bool, UnwindError> {
        let code = UnwindError::from_code(unsafe { unw_step(&mut *self.inner.borrow_mut()) })?;
        Ok(code == UNW_STEP_SUCCESS)
    }

    /// Retrieves the value of the given register for the cursor's current
    /// frame.
    ///
    /// # Errors
    ///
    /// This function may return one of the following errors:
    ///
    /// - [`UnwindError::Unspecified`] if an unspecified error occurred
    /// - [`UnwindError::BadRegister`] if the register was invalid or
    ///   inaccessible in the current frame
    pub fn register(&self, register: unw_regnum_t) -> Result<usize, UnwindError> {
        let mut reg_value = 0;
        UnwindError::from_code(unsafe {
            unw_get_reg(&mut *self.inner.borrow_mut(), register, &mut reg_value)
        })?;
        Ok(reg_value)
    }

    /// Sets the value of the given register in the cursor's current frame to
    /// the given value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that updating the stack frame as described above
    /// will not cause undefined behavior.
    ///
    /// # Errors
    ///
    /// This function may return one of the following errors:
    ///
    /// - [`UnwindError::Unspecified`] if an unspecified error occurred
    /// - [`UnwindError::BadRegister`] if the register was invalid or
    ///   inaccessible in the current frame
    /// - [`UnwindError::WriteToReadOnlyRegister`] if the register was read-only
    pub unsafe fn set_register(
        &self,
        register: unw_regnum_t,
        value: unw_word_t,
    ) -> Result<(), UnwindError> {
        UnwindError::from_code(unsafe {
            unw_set_reg(&mut *self.inner.borrow_mut(), register, value)
        })?;
        Ok(())
    }

    /// Retrieves the value of the given floating point register for the
    /// cursor's current frame.
    ///
    /// # Errors
    ///
    /// This function may return one of the following errors:
    ///
    /// - [`UnwindError::Unspecified`] if an unspecified error occurred
    /// - [`UnwindError::BadRegister`] if the register was invalid or
    ///   inaccessible in the current frame
    pub fn fp_register(&self, register: unw_regnum_t) -> Result<usize, UnwindError> {
        let mut reg_value = 0;
        UnwindError::from_code(unsafe {
            unw_get_reg(&mut *self.inner.borrow_mut(), register, &mut reg_value)
        })?;
        Ok(reg_value)
    }

    /// Sets the value of the given floating-point register in the cursor's
    /// current frame to the given value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that updating the stack frame as described above
    /// will not cause undefined behavior.
    ///
    /// # Errors
    ///
    /// This function may return one of the following errors:
    ///
    /// - [`UnwindError::Unspecified`] if an unspecified error occurred
    /// - [`UnwindError::BadRegister`] if the register was invalid or
    ///   inaccessible in the current frame
    /// - [`UnwindError::WriteToReadOnlyRegister`] if the register was read-only
    pub unsafe fn set_fp_register(
        &self,
        register: unw_regnum_t,
        value: unw_fpreg_t,
    ) -> Result<(), UnwindError> {
        UnwindError::from_code(unsafe {
            unw_set_fpreg(&mut *self.inner.borrow_mut(), register, value)
        })?;
        Ok(())
    }

    /// Checks whether the given register is a floating-point register.
    pub fn is_fp_register(&self, register: unw_regnum_t) -> bool {
        unsafe { unw_is_fpreg(&mut *self.inner.borrow_mut(), register) > 0 }
    }

    /// Checks whether the current frame is a "signal frame," which is defined
    /// as a frame created in response to a potentially asynchronous
    /// interruption such as a device interrupt.
    ///
    /// Signal frames offer access to a larger range of registers because their
    /// nature requires saving the contents of registers normally treated as
    /// "scratch" registers.
    ///
    /// Corresponds to [`unw_is_signal_frame`](https://www.nongnu.org/libunwind/man/unw_is_signal_frame(3).html).
    ///
    /// # Errors
    ///
    /// If `libunwind` is unable to determine whether the cursor is pointing to
    /// a signal frame, [`UnwindError::NoInfo`] is returned.
    pub fn is_signal_frame(&self) -> Result<bool, UnwindError> {
        let code = unsafe { unw_is_signal_frame(&mut *self.inner.borrow_mut()) };
        UnwindError::from_code(code)?;
        Ok(code > 0)
    }

    /// Returns the name of the given register as a string, or [`None`] if the
    /// register does not exist.
    pub fn register_name(&self, register: unw_regnum_t) -> Option<&'static CStr> {
        let unknown = c"unknown register";
        // SAFETY: libunwind guarantees string is statically allocated and valid
        let str = unsafe { CStr::from_ptr(unw_regname(&mut *self.inner.borrow_mut(), register)) };
        if str == unknown {
            None
        } else {
            Some(str)
        }
    }
}

impl Debug for UnwindCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("UnwindCursor");
        if let Ok(ip) = self.register(registers::UNW_REG_IP) {
            s.field("ip", &(ip as *const ())).finish()
        } else {
            s.finish_non_exhaustive()
        }
    }
}
