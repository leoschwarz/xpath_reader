// Copyright 2018-2019 Leonardo Schwarz <mail@leoschwarz.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! XPath expression convenience typing.
//!
//! Provides a way to pass both pre-parsed and unparsed expressions
//! as parameter to other methods.

use errors::{Error, ErrorKind};
use std::borrow::{Borrow, Cow};
use sxd_xpath::{Factory, XPath};
use util::Refable;

/// An XPath expression that can be evaluated on documents.
///
/// `From` implementations exist so you can use this crate easily, by
/// providing strings as XPath expressions directly. However for better
/// performance in repeated evaluation of the same XPath expression, you
/// should use the module level function `expression::parse` so it will
/// be parsed exactly once.
#[derive(Debug)]
pub struct XPathExpression<'a>(Repr<'a>);

/// Parse an expression in advance, this can be useful
/// if you want to avoid an XPath expression being parsed
/// on every invocation.
pub fn parse(xpath_expr: &str) -> Result<XPathExpression<'static>, Error> {
    parse_xpath(xpath_expr).map(|x| XPathExpression(Repr::Parsed(Refable::Owned(x))))
}

#[derive(Debug)]
enum Repr<'a> {
    Parsed(Refable<'a, XPath>),
    Unparsed(Cow<'a, str>),
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
        XPathExpression(Repr::Unparsed(Cow::Borrowed(s)))
    }
}

impl<'a> From<String> for XPathExpression<'a> {
    fn from(s: String) -> Self {
        XPathExpression(Repr::Unparsed(Cow::Owned(s)))
    }
}

impl<'a> From<&'a XPathExpression<'a>> for XPathExpression<'a> {
    fn from(x: &'a XPathExpression<'a>) -> Self {
        match x.0 {
            Repr::Parsed(ref refable) => XPathExpression(Repr::Parsed(refable.clone_ref())),
            Repr::Unparsed(ref s) => XPathExpression(Repr::Unparsed(s.clone())),
        }
    }
}

fn parse_xpath(xpath_expr: &str) -> Result<XPath, Error> {
    Factory::new()
        .build(xpath_expr)
        .map_err(|e| Error::internal(format!("{}", e), ErrorKind::ParseXPath))?
        .ok_or_else(|| Error::internal("Empty XPath expression.", ErrorKind::ParseXPath))
}
