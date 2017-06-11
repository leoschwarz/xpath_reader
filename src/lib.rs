// Copyright 2017 Leonardo Schwarz <mail@leoschwarz.com>
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

//! Provides a convenient API to read from XML using XPath queries.
//!
//! This crate is mostly a wrapper around the crate [sxd_xpath](https://github.com/shepmaster/sxd-xpath).
//!
//! # Examples
//! ```
//! use xpath_reader::{Context, XpathReader, XpathStrReader};
//!
//! let xml = r#"<?xml version="1.0"?><book xmlns="books" name="Neuromancer" author="William Gibson"><tags><tag name="cyberpunk"/><tag name="sci-fi"/></tags></book>"#;
//!
//! let mut context = Context::new();
//! context.set_namespace("b", "books");
//!
//! let reader = XpathStrReader::new(xml, &context).unwrap();
//!
//! let name: String = reader.read("//@name").unwrap();
//! assert_eq!(name, "Neuromancer".to_string());
//!
//! let publisher: Option<String> = reader.read_option("//@publisher").unwrap();
//! let author: Option<String> = reader.read_option("//@author").unwrap();
//! assert_eq!(publisher, None);
//! assert_eq!(author, Some("William Gibson".to_string()));
//!
//! let tags: Vec<String> = reader.read_vec("//b:tags/b:tag/@name").unwrap();
//! assert_eq!(tags, vec!["cyberpunk".to_string(), "sci-fi".to_string()]);
//! ```

#[macro_use]
extern crate error_chain;
extern crate sxd_document;
extern crate sxd_xpath;

pub mod errors;
pub use self::errors::{FromXmlError, XpathError, XpathErrorKind};

pub mod context;
pub use self::context::Context;

pub mod reader;
pub use self::reader::{FromXml, XpathReader, XpathStrReader, XpathNodeReader};
