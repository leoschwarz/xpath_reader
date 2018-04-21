use errors::{Error, XpathError};
use std::borrow::Borrow;
use sxd_xpath::{Factory, XPath};
use util::Refable;

#[derive(Debug)]
pub struct XPathExpression<'a>(Repr<'a>);

#[derive(Debug)]
enum Repr<'a> {
    Parsed(Refable<'a, XPath>),
    Unparsed(&'a str),
}

impl XPathExpression<'static> {
    /// Parse the expression in advance, this can be useful
    /// if you want to avoid an XPath expression being parsed
    /// on every invocation.
    pub fn parse(xpath_expr: &str) -> Result<Self, Error> {
        parse_xpath(xpath_expr).map(|x| XPathExpression(Repr::Parsed(Refable::Owned(x))))
    }
}

impl<'a> XPathExpression<'a> {
    pub(crate) fn parsed(&self) -> Result<Refable<XPath>, Error> {
        match self.0 {
            Repr::Parsed(ref refable) => Ok(refable.clone_ref()),
            Repr::Unparsed(ref s) => parse_xpath(s).map(|x| Refable::Owned(x)),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self.0 {
            Repr::Parsed(ref refable) => {
                let xpath: &XPath = refable.borrow();
                format!("{:?}", xpath)
            }
            Repr::Unparsed(ref s) => s.to_string(),
        }
    }
}

impl From<XPath> for XPathExpression<'static> {
    fn from(xpath: XPath) -> Self {
        XPathExpression(Repr::Parsed(Refable::Owned(xpath)))
    }
}

impl<'a> From<&'a XPath> for XPathExpression<'a> {
    fn from(xpath: &'a XPath) -> Self {
        XPathExpression(Repr::Parsed(Refable::Borrowed(xpath)))
    }
}

impl<'a> From<&'a str> for XPathExpression<'a> {
    fn from(s: &'a str) -> Self {
        XPathExpression(Repr::Unparsed(s))
    }
}

impl<'a> From<&'a XPathExpression<'a>> for XPathExpression<'a> {
    fn from(x: &'a XPathExpression<'a>) -> Self {
        match x.0 {
            Repr::Parsed(ref refable) => XPathExpression(Repr::Parsed(refable.clone_ref())),
            Repr::Unparsed(ref s) => XPathExpression(Repr::Unparsed(s)),
        }
    }
}

fn parse_xpath(xpath_expr: &str) -> Result<XPath, Error> {
    Factory::new()
        .build(xpath_expr)
        .map_err(|e| Error::Xpath(e.into()))?
        .ok_or(Error::Xpath(XpathError::Empty))
}
