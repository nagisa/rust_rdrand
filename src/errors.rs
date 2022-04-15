use core::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
};

/// Errors in this library
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum ErrorCode {
    /// The hardware instruction is not supported
    UnsupportedInstruction,
    /// There was a hardware failure
    HardwareFailure,
}

impl ErrorCode {
    #[cfg(not(feature = "std"))]
    const fn as_randcore_code(self) -> core::num::NonZeroU32 {
        /// Arbitrary, off top of head bitmask for error codes that come from rdrand
        const RDRAND_TAG: u32 = rand_core::Error::CUSTOM_START + 0x3D34_7D00;
        unsafe { core::num::NonZeroU32::new_unchecked(RDRAND_TAG + self as u32) }
    }
}

#[cfg(not(feature = "std"))]
impl From<ErrorCode> for rand_core::Error {
    fn from(code: ErrorCode) -> rand_core::Error {
        code.as_randcore_code().into()
    }
}

#[cfg(feature = "std")]
impl From<ErrorCode> for rand_core::Error {
    fn from(code: ErrorCode) -> rand_core::Error {
        rand_core::Error::new(code)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ErrorCode {}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            ErrorCode::UnsupportedInstruction => "the hardware instruction is not supported",
            ErrorCode::HardwareFailure => "hardware generator failure",
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NotAnErrorCode;

#[cfg(feature = "std")]
impl std::error::Error for NotAnErrorCode {}

impl Display for NotAnErrorCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("the error is not an rdrand error")
    }
}

impl TryFrom<&rand_core::Error> for ErrorCode {
    type Error = NotAnErrorCode;
    #[cfg(feature = "std")]
    fn try_from(error: &rand_core::Error) -> Result<Self, Self::Error> {
        error
            .inner()
            .downcast_ref::<ErrorCode>()
            .copied()
            .ok_or(NotAnErrorCode)
    }
    #[cfg(not(feature = "std"))]
    fn try_from(error: &rand_core::Error) -> Result<Self, Self::Error> {
        let code = error.code().ok_or(NotAnErrorCode)?;
        if code == ErrorCode::UnsupportedInstruction.as_randcore_code() {
            Ok(ErrorCode::UnsupportedInstruction)
        } else if code == ErrorCode::HardwareFailure.as_randcore_code() {
            Ok(ErrorCode::HardwareFailure)
        } else {
            Err(NotAnErrorCode)
        }
    }
}

impl TryFrom<rand_core::Error> for ErrorCode {
    type Error = NotAnErrorCode;
    fn try_from(error: rand_core::Error) -> Result<Self, Self::Error> {
        <ErrorCode as TryFrom<&rand_core::Error>>::try_from(&error)
    }
}

#[cfg(test)]
mod test {
    use super::ErrorCode;
    use core::convert::TryInto;
    use rand_core::Error;

    #[test]
    fn error_code_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ErrorCode>();
    }

    #[test]
    fn error_code_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<ErrorCode>();
    }

    #[test]
    fn error_code_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<ErrorCode>();
    }

    #[test]
    fn error_code_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<ErrorCode>();
    }

    #[test]
    #[cfg(feature = "std")]
    fn error_code_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<ErrorCode>();
    }

    #[test]
    fn conversion_roundtrip_unsupported_hardware() {
        let core_rand: Error = ErrorCode::UnsupportedInstruction.into();
        let code: ErrorCode = core_rand.try_into().expect("should convert back");
        assert!(matches!(code, ErrorCode::UnsupportedInstruction));
    }

    #[test]
    fn conversion_roundtrip_hardware_failure() {
        let core_rand: Error = ErrorCode::HardwareFailure.into();
        let code: ErrorCode = core_rand.try_into().expect("should convert back");
        assert!(matches!(code, ErrorCode::HardwareFailure));
    }
}
