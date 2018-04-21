//! Error definitions.

// TODO: impl fmt::Display, error::Error for all error types.

use std::{error, fmt};

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

/*

/// Any error that can occur when reading values from XML.
#[derive(Debug)]
pub enum Error {
    /// There was an error parsing the XML document.
    ParseXml(::sxd_document::parser::Error),

    /// An error due to XPath expression parsing or evaluation.
    XPath(String),


    /// One or more required nodes are missing, may optionally include
    /// a description of the missing nodes.
    MissingNodes(Option<String>),

    Message(String),
    Other(Box<error::Error + Send + Sync>)
}

impl<T> From<T> for Error
where
    T: Into<::sxd_xpath::Error>,
{
    fn from(t: T) -> Self {
        Error::XPath(format!("{}", t.into()))
    }
}
*/
