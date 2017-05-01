//! Errors used in this crate.
//! We are using `error-chain` so if you are using it too you can just add a link for this crate's
//! errors.

error_chain! {
    types {
        XpathError, XpathErrorKind, ChainXpathErr, XpathResult;
    }

    foreign_links {
        ParseBoolError(::std::str::ParseBoolError);
        ParseIntError(::std::num::ParseIntError);
        XmlParseError(::sxd_document::parser::Error);
        XpathError(::sxd_xpath::Error);
        XpathExecuteError(::sxd_xpath::ExecutionError);
        XpathParseError(::sxd_xpath::ParserError);
    }

    errors {
        /// XPath expression failed to evaluate to a value.
        /// The String variant contains a copy of the XPath expression.
        NodeNotFound(xpath: String) {
            description("XPath failed to find a node")
            display("XPath expression '{}' failed to find a node", xpath)
        }
    }
}

// TODO: Take this upstream, either the tuple should implement std::Error or another type should be
// used which does.
impl From<(usize, ::std::vec::Vec<::sxd_document::parser::Error>)> for XpathError {
    fn from(err: (usize, ::std::vec::Vec<::sxd_document::parser::Error>)) -> XpathError {
        XpathErrorKind::XmlParseError(err.1[0]).into()
    }
}
