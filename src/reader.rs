// Copyright 2017-2018 Leonardo Schwarz <mail@leoschwarz.com>
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

//! XPath based document parsing.

use errors::{Error, ErrorKind};
use expression::XPathExpression;
use std::borrow::{Borrow, Cow};
use sxd_document::parser::parse as sxd_parse;
use sxd_document::Package;
use sxd_xpath::nodeset::{Node, Nodeset};
use sxd_xpath::{Context, Value, XPath};
use util::Refable;

/// Convenience redefinition of the FromXml result type.
pub type FromXmlResult<T> = Result<T, Error>;

/// A value that can be deserialized from a XML reader.
pub trait FromXml
where
    Self: Sized,
{
    /// Read an instance of `Self` from the provided `reader`.
    ///
    /// The exact semantics of when this fails or succeeds are implementor
    /// defined. However for `Option<T>` a best effort approach should
    /// be followed, returning `Ok(None)` in absence of a value instead of
    /// an error.
    fn from_xml<'d>(reader: &'d Reader<'d>) -> FromXmlResult<Self>;
}

/// A helper trait to define two `FromXml` implementations at once.
///
/// In general you want to implement `FromXml` directly, however
/// sometimes you want to implement it for `T` and `Option<T>`.
/// In this case you can implement `FromXmlOptional` for `T` and
/// `FromXml` will be implemented for both types where `FromXml`
/// for `T` will fail if the your implementation returns None.
pub trait FromXmlOptional
where
    Self: Sized,
{
    /// `FromXml::from_xml` impl for `Option<T>`.
    fn from_xml_optional<'d>(reader: &'d Reader<'d>) -> FromXmlResult<Option<Self>>;
}

impl<T> FromXml for T
where
    T: FromXmlOptional,
{
    fn from_xml<'d>(reader: &'d Reader<'d>) -> FromXmlResult<Self> {
        // TODO: Better error message.
        T::from_xml_optional(reader).and_then(|opt| {
            opt.ok_or_else(|| {
                Error::custom_msg(format!("Missing value for type {:?}", stringify!(T)))
            })
        })
    }
}

impl<T> FromXml for Option<T>
where
    T: FromXmlOptional,
{
    fn from_xml<'d>(reader: &'d Reader<'d>) -> FromXmlResult<Self> {
        T::from_xml_optional(reader)
    }
}

enum Anchor<'d> {
    Nodeset(Nodeset<'d>),
    Root(Package),
}

/// XML element tree reader using XPath expressions.
///
/// # Anchor nodeset
///
/// An instance of `Reader` either contains a complete document, or
/// references an anchor nodeset (a "relative" reader).
/// There are two aspects for which this distinction is relevant:
///
/// 1) Relative expressions: If there is an anchor nodeset, relative
///    XPath expressions will be evaluated relative to the first
///    node in the nodeset, in document order.
/// 2) `FromXml` implementors can query the anchor nodeset to convert
///    multiple nodes into a single target value.
pub struct Reader<'d> {
    context: Refable<'d, Context<'d>>,
    anchor: Anchor<'d>,
}

impl<'d> Reader<'d> {
    /// Read the result of the XPath expression into a value of type `V`.
    pub fn read<'a, V, X>(&'d self, xpath_expr: X) -> Result<V, Error>
    where
        V: FromXml,
        X: Into<XPathExpression<'a>>,
    {
        let reader = self.with_nodeset_eval(xpath_expr)?;
        V::from_xml(&reader)
    }

    /// Construct a new reader for the specified XML document.
    ///
    /// A context can be specified to define custom functions,
    /// variables and namespaces.
    pub fn from_str(xml: &str, context: Option<&'d Context<'d>>) -> Result<Self, Error> {
        // TODO: Display all.
        let package =
            sxd_parse(xml).map_err(|e| Error::internal(format!("{}", e), ErrorKind::ParseXml))?;

        let context_refable = match context {
            Some(c) => Refable::Borrowed(c),
            None => Refable::Owned(Context::default()),
        };

        Ok(Reader {
            context: context_refable,
            anchor: Anchor::Root(package),
        })
    }

    /// Construct a new reader for the specified nodeset.
    ///
    /// Relative XPath expressions will then resolve to the first node
    /// in the nodeset.
    ///
    /// The nodeset can be obtained from a `Reader<'d>` with the
    /// `anchor_nodeset` and `anchor_node` methods.
    /// A context can be specified to define custom functions,
    /// variables and namespaces.
    ///
    /// Note: The nodeset can even be empty, which can be used by `FromXml`
    /// implementors to cover the absence of a value in some cases.
    pub fn from_nodeset(nodeset: Nodeset<'d>, context: Option<&'d Context<'d>>) -> Self {
        let context_refable = match context {
            Some(c) => Refable::Borrowed(c),
            None => Refable::Owned(Context::default()),
        };

        Reader {
            context: context_refable,
            anchor: Anchor::Nodeset(nodeset),
        }
    }

    /// Convenience method over `from_nodeset` when there is only one `Node` for
    /// the nodeset.
    pub fn from_node(node: Node<'d>, context: Option<&'d Context<'d>>) -> Self {
        let mut nodeset = Nodeset::new();
        nodeset.add(node);
        Self::from_nodeset(nodeset, context)
    }

    /// Creates a new `Reader` instance by evaluating an XPath expression and
    /// using the result nodeset as anchor nodeset.
    ///
    /// The current context will be passed to the new reader.
    pub fn with_nodeset_eval<'a, X>(&'d self, xpath_expr: X) -> Result<Self, Error>
    where
        X: Into<XPathExpression<'a>>,
    {
        let xpath = xpath_expr.into();
        match self.evaluate(&xpath)? {
            Value::Nodeset(nodeset) => Ok(Reader {
                context: self.context.clone_ref(),
                anchor: Anchor::Nodeset(nodeset),
            }),
            _ => Err(Error::internal(
                format!(
                    "XPath expression did not evaluate to nodeset: '{}'",
                    xpath.to_string()
                ),
                ErrorKind::EvalXPath,
            )),
        }
    }

    /// References the evaluation context of this Reader.
    pub fn context(&'d self) -> &'d Context<'d> {
        self.context.borrow()
    }

    /// Returns the anchor nodeset of the current reader.
    pub fn anchor_nodeset(&'d self) -> Cow<Nodeset<'d>> {
        match self.anchor {
            Anchor::Nodeset(ref nodeset) => Cow::Borrowed(nodeset),
            Anchor::Root(ref package) => {
                let mut nodeset = Nodeset::new();
                let root = package.as_document().root().clone();
                nodeset.add(Node::Root(root));
                Cow::Owned(nodeset)
            }
        }
    }

    /// Returns the first (in document order) node in the anchor nodeset.
    ///
    /// If the anchor nodeset is empty, `None` will be returned.
    pub fn anchor_node(&'d self) -> Option<Node<'d>> {
        match self.anchor {
            Anchor::Nodeset(ref nodeset) => nodeset.document_order_first(),
            Anchor::Root(ref package) => Some(package.as_document().root().clone().into()),
        }
    }

    fn evaluate<'a, X>(&'d self, xpath_expr: X) -> Result<Value<'d>, Error>
    where
        X: Into<XPathExpression<'a>>,
    {
        let xpath_expr = xpath_expr.into();
        let xpath = xpath_expr.parsed()?;
        // TODO: Error message.
        let anchor = self.anchor_node().ok_or_else(|| {
            let xpath_ref: &XPath = xpath.borrow();
            Error::internal(
                format!("Anchor node not found when evaluating: {:?}", xpath_ref),
                ErrorKind::EvalXPath,
            )
        })?;

        // Note: This is very ugly but otherwise does not compile.
        let xpath_ref: &XPath = xpath.borrow();
        xpath_ref
            .evaluate(self.context.borrow(), anchor)
            .map_err(|e| Error::internal(format!("{}", e), ErrorKind::EvalXPath))
    }
}

impl FromXml for String {
    fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error> {
        reader
            .anchor_node()
            .ok_or(Error::custom_msg("Missing (anchor) node."))
            .map(|n| n.string_value())
    }
}

impl FromXml for Option<String> {
    fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error> {
        Ok(reader.anchor_node().and_then(|node| {
            let s = node.string_value();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }))
    }
}

impl<T> FromXml for Vec<T>
where
    T: FromXml,
{
    fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error> {
        reader
            .anchor_nodeset()
            .document_order()
            .iter()
            .map(|node| {
                let reader = Reader::from_node(*node, Some(reader.context()));
                T::from_xml(&reader)
            })
            .collect()
    }
}

macro_rules! from_parse_str {
    ( $( $type:ty ),* ) => {
        $(
            impl FromXml for $type {
                fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error>
                {
                    let s = String::from_xml(reader)?;
                    s.parse::<$type>().map_err(|e| Error::custom_err(e))
                }
            }

            impl FromXml for Option<$type> {
                fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error>
                {
                    if let Some(s) = Option::<String>::from_xml(reader)? {
                        Ok(Some(s.parse::<$type>().map_err(|e| Error::custom_err(e))?))
                    } else {
                        Ok(None)
                    }
                }
            }
        )*
    }
}

from_parse_str!(f32, f64, u8, u16, u32, u64, i8, i16, i32, i64, bool);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xpath_str_reader() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
                     <root><child name="Hello World"/></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();
        assert_eq!(
            reader.evaluate(".//child/@name").unwrap().string(),
            "Hello World".to_string()
        );
    }

    #[test]
    fn string_from_xml() {
        let xml = r#"<?xml version="1.0"?>
                     <root><title>Hello World</title><empty/></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let title = reader.with_nodeset_eval("//title").unwrap();
        assert_eq!(String::from_xml(&title).unwrap(), "Hello World");
        assert_eq!(
            Option::<String>::from_xml(&title).unwrap(),
            Some("Hello World".to_string())
        );

        let empty = reader.with_nodeset_eval("//empty").unwrap();
        assert_eq!(String::from_xml(&empty).unwrap(), "");
        assert_eq!(Option::<String>::from_xml(&empty).unwrap(), None);

        let inexistent = reader.with_nodeset_eval("//inexistent").unwrap();
        assert!(String::from_xml(&inexistent).is_err());
        assert_eq!(Option::<String>::from_xml(&inexistent).unwrap(), None);
    }

    #[test]
    fn num_from_xml() {
        let xml = r#"<?xml version="1.0"?><root><float>-23.85</float><int>42</int></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let float = reader.with_nodeset_eval("//float").unwrap();
        let int = reader.with_nodeset_eval("//int").unwrap();

        assert_eq!(f32::from_xml(&float).unwrap(), -23.85f32);
        assert_eq!(f32::from_xml(&int).unwrap(), 42f32);
        assert_eq!(f64::from_xml(&float).unwrap(), -23.85f64);
        assert_eq!(f64::from_xml(&int).unwrap(), 42f64);

        assert_eq!(u8::from_xml(&int).unwrap(), 42u8);
        assert_eq!(u16::from_xml(&int).unwrap(), 42u16);
        assert_eq!(u32::from_xml(&int).unwrap(), 42u32);
        assert_eq!(u64::from_xml(&int).unwrap(), 42u64);

        assert_eq!(i8::from_xml(&int).unwrap(), 42i8);
        assert_eq!(i16::from_xml(&int).unwrap(), 42i16);
        assert_eq!(i32::from_xml(&int).unwrap(), 42i32);
        assert_eq!(i64::from_xml(&int).unwrap(), 42i64);
    }

    #[test]
    fn num_absent() {
        let xml = r#"<?xml version="1.0"?><root><float>-23.85</float><int>42</int></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let opt1: Option<f32> = reader.read("//float").unwrap();
        let opt2: Option<f32> = reader.read("//ffloat").unwrap();

        assert_eq!(opt1, Some(-23.85f32));
        assert_eq!(opt2, None);
    }

    #[test]
    fn bool_from_xml() {
        let xml = r#"<?xml version="1.0"?><root><t>true</t><f>false</f></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let t = reader.with_nodeset_eval("//t").unwrap();
        let f = reader.with_nodeset_eval("//f").unwrap();

        assert_eq!(bool::from_xml(&t).unwrap(), true);
        assert_eq!(bool::from_xml(&f).unwrap(), false);
    }

    #[test]
    fn vec_existent() {
        let xml = r#"<?xml version="1.0"?><book><tags><tag name="cyberpunk"/><tag name="sci-fi"/></tags></book>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let tags: Vec<String> = reader.read("//book/tags/tag/@name").unwrap();
        assert_eq!(tags, vec!["cyberpunk".to_string(), "sci-fi".to_string()]);
    }

    #[test]
    fn vec_non_existent() {
        let xml = r#"<?xml version="1.0"?><root><t>true</t><f>false</f></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let tags: Vec<String> = reader.read("//book/tags/tag/@name").unwrap();
        assert_eq!(tags, Vec::<String>::new());
    }
}
