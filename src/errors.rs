//! Error handling types.

use std::error;

/// Any error that can occur when reading values from XML.
#[derive(Debug)]
pub enum Error {
    /// Parsing a value failed, a value was present but invalid.
    ParseValue(Box<error::Error + Send + Sync>),

    /// If a requested node was not found.
    NodeNotFound(String),

    /// There was an error parsing the XML document.
    ParseXml(::sxd_document::parser::Error),

    Xpath(XpathError),
}

#[derive(Debug)]
pub enum XpathError {
    // TODO
    External(::sxd_xpath::Error),

    /// An empty XPath expression was supplied.
    Empty,

    // TODO: Does this belong here?
    NotNodeset(String),
}

impl<T> From<T> for XpathError where T: Into<::sxd_xpath::Error> {
    fn from(t: T) -> Self {
        XpathError::External(t.into())
    }
}
