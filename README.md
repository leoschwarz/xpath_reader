# xpath-reader
[![Current Version](http://meritbadge.herokuapp.com/xpath_reader)](https://crates.io/crates/xpath_reader)

Provides a convenient API to read from XML using XPath queries.

This crate is mostly a wrapper around the crate [sxd_xpath](https://github.com/shepmaster/sxd-xpath).

# Examples
```rust
use xpath_reader::{Context, XpathReader, XpathStrReader};

let xml = r#"<?xml version="1.0"?><book xmlns="books" name="Neuromancer" author="William Gibson"><tags><tag name="cyberpunk"/><tag name="sci-fi"/></tags></book>"#;

let mut context = Context::new();
context.set_namespace("b", "books");

let reader = XpathStrReader::new(xml, &context).unwrap();

let name: String = reader.read("//@name").unwrap();
assert_eq!(name, "Neuromancer".to_string());

let publisher: Option<String> = reader.read_option("//@publisher").unwrap();
let author: Option<String> = reader.read_option("//@author").unwrap();
assert_eq!(publisher, None);
assert_eq!(author, Some("William Gibson".to_string()));

let tags: Vec<String> = reader.read_vec("//b:tags/b:tag/@name").unwrap();
assert_eq!(tags, vec!["cyberpunk".to_string(), "sci-fi".to_string()]);
```

