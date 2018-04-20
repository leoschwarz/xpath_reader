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

use errors::{Error, XpathError};
use std::borrow::{Borrow, Cow};
use sxd_document::Package;
use sxd_document::parser::parse as sxd_parse;
use sxd_xpath::{Context, Factory, Value, XPath};
use sxd_xpath::nodeset::{Node, Nodeset};

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
    fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error>;
}

enum Anchor<'d> {
    Nodeset(Nodeset<'d>),
    Root(Package),
}

// TODO: Is there a standard type for this in Rust, like Cow but without
//       the clone requirement.
enum Refable<'a, T: 'a> {
    Owned(T),
    Borrowed(&'a T),
}

impl<'a, T> Borrow<T> for Refable<'a, T> {
    fn borrow(&self) -> &T {
        match self {
            &Refable::Owned(ref v) => &v,
            &Refable::Borrowed(v) => v,
        }
    }
}

impl<'a, T> Refable<'a, T> {
    fn clone_ref(&'a self) -> Self {
        Refable::Borrowed(self.borrow())
    }
}

/// Reads a XML tree using XPath queries.
///
/// In the most basic case the XML tree equates a complete XML document,
/// however using the `relative` and `from_node` methods it's also
/// possible to create `Reader` instances relative to a specific anchor
/// nodeset.
pub struct Reader<'d> {
    context: Refable<'d, Context<'d>>,
    // TODO
    factory: Factory,
    anchor: Anchor<'d>,
}

impl<'d> Reader<'d> {
    /// Construct a new reader for the specified XML document.
    ///
    /// A context can be specified to define functions, variables and namespaces.
    pub fn from_str(xml: &str, context: Option<&'d Context<'d>>) -> Result<Self, Error> {
        let package = sxd_parse(xml).map_err(|e| Error::ParseXml(e.1[0]))?;

        let context_refable = match context {
            Some(c) => Refable::Borrowed(c),
            None => Refable::Owned(Context::default()),
        };

        Ok(Reader {
            context: context_refable,
            factory: Factory::default(),
            anchor: Anchor::Root(package),
        })
    }

    pub fn from_node(node: Node<'d>, context: Option<&'d Context<'d>>) -> Self {
        let mut nodeset = Nodeset::new();
        nodeset.add(node);

        let context_refable = match context {
            Some(c) => Refable::Borrowed(c),
            None => Refable::Owned(Context::default()),
        };

        Reader {
            context: context_refable,
            factory: Factory::default(),
            anchor: Anchor::Nodeset(nodeset),
        }
    }

    pub fn context(&'d self) -> &'d Context<'d> {
        self.context.borrow()
    }

    /// Read the result of the xpath expression into a value of type `V`.
    pub fn read<V>(&'d self, xpath_expr: &str) -> Result<V, Error>
    where
        V: FromXml,
    {
        let reader = self.relative(xpath_expr)?;
        V::from_xml(&reader)
    }

    /// Returns the anchor node of the current XML tree.
    ///
    /// If there are multiple nodes the first node in document order
    /// will be returned.
    pub fn anchor_node(&'d self) -> Option<Node<'d>> {
        match self.anchor {
            Anchor::Nodeset(ref nodeset) => nodeset.document_order_first(),
            Anchor::Root(ref package) => Some(package.as_document().root().clone().into()),
        }
    }

    /// Returns the anchor node set of the current reader.
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

    // TODO: Revise this method.
    // Evaluates an XPath query, takes the first returned node (in document order) and creates
    // a new `XpathNodeReader` with that node at its root.
    pub fn relative(&'d self, xpath_expr: &str) -> Result<Self, Error> {
        let nodeset = match self.evaluate(xpath_expr)? {
            Value::Nodeset(nodeset) => nodeset,
            _ => {
                return Err(Error::Xpath(XpathError::NotNodeset(xpath_expr.into())));
            }
        };
        Ok(Reader {
            context: self.context.clone_ref(),
            // TODO
            factory: Factory::default(),
            anchor: Anchor::Nodeset(nodeset),
        })
    }

    fn evaluate(&'d self, xpath_expr: &str) -> Result<Value<'d>, Error> {
        let xpath = build_xpath(&self.factory, xpath_expr)?;
        // TODO: Error message.
        let anchor = self.anchor_node()
            .ok_or_else(|| Error::NodeNotFound("".into()))?;

        xpath
            .evaluate(self.context.borrow(), anchor)
            .map_err(|e| Error::Xpath(e.into()))
    }
}

fn build_xpath(factory: &Factory, xpath_expr: &str) -> Result<XPath, Error> {
    factory
        .build(xpath_expr)
        .map_err(|e| Error::Xpath(e.into()))?
        .ok_or(Error::Xpath(XpathError::Empty))
}

impl FromXml for String {
    fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error> {
        reader
            .anchor_node()
            .ok_or(Error::MissingAnchor)
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
                    s.parse::<$type>().map_err(|e| Error::ParseValue(Box::new(e)))
                }
            }

            impl FromXml for Option<$type> {
                fn from_xml<'d>(reader: &'d Reader<'d>) -> Result<Self, Error>
                {
                    if let Some(s) = Option::<String>::from_xml(reader)? {
                        Ok(Some(s.parse::<$type>().map_err(|e| Error::ParseValue(Box::new(e)))?))
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

        let title = reader.relative("//title").unwrap();
        assert_eq!(String::from_xml(&title).unwrap(), "Hello World");
        assert_eq!(
            Option::<String>::from_xml(&title).unwrap(),
            Some("Hello World".to_string())
        );

        let empty = reader.relative("//empty").unwrap();
        assert_eq!(String::from_xml(&empty).unwrap(), "");
        assert_eq!(Option::<String>::from_xml(&empty).unwrap(), None);

        let inexistent = reader.relative("//inexistent").unwrap();
        assert!(String::from_xml(&inexistent).is_err());
        assert_eq!(Option::<String>::from_xml(&inexistent).unwrap(), None);
    }

    #[test]
    fn num_from_xml() {
        let xml = r#"<?xml version="1.0"?><root><float>-23.85</float><int>42</int></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let float = reader.relative("//float").unwrap();
        let int = reader.relative("//int").unwrap();

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

        let t = reader.relative("//t").unwrap();
        let f = reader.relative("//f").unwrap();

        assert_eq!(bool::from_xml(&t).unwrap(), true);
        assert_eq!(bool::from_xml(&f).unwrap(), false);
    }

    #[test]
    fn vec_existent() {
        let xml = r#"<?xml version="1.0"?><book><tags><tag name="cyberpunk"/><tag name="sci-fi"/></tags></book>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let tags = reader.read::<Vec<String>>("//book/tags/tag/@name").unwrap();
        assert_eq!(tags, vec!["cyberpunk".to_string(), "sci-fi".to_string()]);
    }

    #[test]
    fn vec_non_existent() {
        let xml = r#"<?xml version="1.0"?><root><t>true</t><f>false</f></root>"#;
        let reader = Reader::from_str(xml, None).unwrap();

        let tags = reader.read::<Vec<String>>("//book/tags/tag/@name").unwrap();
        assert_eq!(tags, Vec::<String>::new());
    }
}
