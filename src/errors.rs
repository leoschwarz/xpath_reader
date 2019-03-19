//! Error definitions.

use std::{error, fmt};

/// The error type used throughout the crate.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    data: ErrorData,
}

/// Describes the kind of the error.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// There was an error parsing the XML document.
    ParseXml,
    /// There was an error parsing the XPath expression.
    ParseXPath,
    /// There was an error evaluation the XPath expression.
    EvalXPath,
    /// There was an other error.
    Other,
}

pub(crate) trait InternalError: fmt::Display + fmt::Debug + Send + Sync {}

impl<T> InternalError for T where T: fmt::Display + fmt::Debug + Send + Sync {}

#[derive(Debug)]
enum ErrorData {
    Internal(Box<InternalError>),
    Custom(CustomError),
}

#[derive(Debug)]
pub enum CustomError {
    Message(String),
    Error(Box<error::Error + Send + Sync>),
    ErrorWithMessage(Box<error::Error + Send + Sync>, String),
}

impl Error {
    pub(crate) fn internal<E: 'static + InternalError>(error: E, kind: ErrorKind) -> Self {
        Error {
            kind: kind,
            data: ErrorData::Internal(Box::new(error)),
        }
    }

    /// Returns the error kind of this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Create a new custom error by providing an error message.
    pub fn custom_msg<S: Into<String>>(s: S) -> Self {
        let data = CustomError::Message(s.into());
        Error {
            kind: ErrorKind::Other,
            data: ErrorData::Custom(data),
        }
    }

    /// Create a new custom error by providing an error object.
    pub fn custom_err<E: 'static + error::Error + Send + Sync>(e: E) -> Self {
        let data = CustomError::Error(Box::new(e));
        Error {
            kind: ErrorKind::Other,
            data: ErrorData::Custom(data),
        }
    }

    /// Create a new custom error by providing an error object and an additional message.
    pub fn custom_err_msg<E: 'static + error::Error + Send + Sync, S: Into<String>>(
        e: E,
        s: S,
    ) -> Self {
        Error {
            kind: ErrorKind::Other,
            data: ErrorData::Custom(CustomError::ErrorWithMessage(Box::new(e), s.into())),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "xpath_reader error: kind = {:?}, message = ", self.kind)?;
        match self.data {
            ErrorData::Internal(ref e) => write!(f, "{}, source = internal", e),
            ErrorData::Custom(CustomError::Message(ref s)) => {
                write!(f, "{}, source = custom msg", s)
            }
            ErrorData::Custom(CustomError::Error(ref e)) => write!(f, "{}, source = custom_err", e),
            ErrorData::Custom(CustomError::ErrorWithMessage(ref e, ref s)) => {
                write!(f, "{}, source = custom_err({})", e, s)
            }
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "xpath_reader error"
    }
}
