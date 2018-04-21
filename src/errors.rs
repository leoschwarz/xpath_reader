//! Error definitions.

use std::{error, fmt};

/// The error type used throughout the crate.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    data: ErrorData,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    ParseXml,
    ParseXPath,
    EvalXPath,
    Custom,
}

pub(crate) trait InternalError: fmt::Display + fmt::Debug + Send + Sync {}

impl<T> InternalError for T
where
    T: fmt::Display + fmt::Debug + Send + Sync,
{
}

#[derive(Debug)]
enum ErrorData {
    Internal(Box<InternalError>),
    Custom(CustomError),
}

#[derive(Debug)]
pub enum CustomError {
    Message(String),
    Error(Box<error::Error + Send + Sync>),
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

    pub fn custom_msg<S: Into<String>>(s: S) -> Self {
        let data = CustomError::Message(s.into());
        Error {
            kind: ErrorKind::Custom,
            data: ErrorData::Custom(data),
        }
    }

    pub fn custom_err<E: 'static + error::Error + Send + Sync>(e: E) -> Self {
        let data = CustomError::Error(Box::new(e));
        Error {
            kind: ErrorKind::Custom,
            data: ErrorData::Custom(data),
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
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "xpath_reader error"
    }
}
