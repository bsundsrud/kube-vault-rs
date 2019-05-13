//! # Haystack
//!
//! Search through unknown YAML for specific data shapes using the visitor pattern.
//!
//! Currently only visits [`Mapping`][serde_yaml_mapping] types given a filter-map function.
//!
//! ## Examples
//!
//! ```rust
//! let YAML = r#"
//! a: "foo"
//! nested:
//!   - name: a
//!   - name: b
//! "#;
//! let corpus = Corpus::from_reader(YAML.as_bytes()).unwrap();
//! // Find values for `name` mappings
//! let values = corpus.filter_map_mappings(|m| m.get("name".into()));
//! let values = corpus.filter_map_mappings(|m| {
//!     m.get(&"name".into())
//!         .and_then(|v| Some(v.as_str()?.to_string()))
//! });
//! assert_eq!(vec!["a", "b"], values);
//! ```
//!
//! [serde_yaml_mapping]: https://docs.rs/serde_yaml/0.8.9/serde_yaml/struct.Mapping.html
use failure::Error;
use serde_yaml::Mapping;
use serde_yaml::{self, Error as SerdeError, Value};
use std::io::prelude::*;

#[derive(Debug)]
pub struct Corpus {
    documents: Vec<Value>,
}

impl Corpus {
    /// Create a Corpus from a `Reader`.  Supports multiple documents in stream that are separated with `---`.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Corpus, Error> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;
        let results: Result<Vec<Value>, SerdeError> = buffer
            .split("---")
            .filter(|s| nonempty_document(s))
            .map(|s: &str| serde_yaml::from_reader::<_, Value>(s.as_bytes()))
            .collect();
        Ok(Corpus {
            documents: results?,
        })
    }

    /// Visit all mappings in the Corpus, executing `filter_map` for each mapping.
    ///
    /// Returning `None` from the `filter_map` function will exclude the value from
    /// the resulting `Vec`.
    pub fn filter_map_mappings<FM, T>(&self, filter_map: FM) -> Vec<T>
    where
        FM: Fn(&Mapping) -> Option<T>,
    {
        let mut res = Vec::new();
        for doc in self.documents.iter() {
            filter_mapping_visitor(doc, &mut res, &filter_map);
        }
        res
    }

    /// Visit all mappings, starting from `val` as the root of the document and
    /// applying `filter_map`.
    pub fn filter_map_values_from<FM, T>(val: &Value, filter_map: FM) -> Vec<T>
    where
        FM: Fn(&Value) -> Option<T>,
    {
        let mut res = Vec::new();
        filter_map_value_visitor(val, &mut res, &filter_map);
        res
    }
}

fn nonempty_document(s: &str) -> bool {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter(|&l| l != "")
        .count()
        > 0
}

fn filter_map_value_visitor<FM, T>(val: &Value, acc: &mut Vec<T>, filter_map: &FM)
where
    FM: Fn(&Value) -> Option<T>,
{
    if let Some(v) = filter_map(&val) {
        acc.push(v);
    }
    match val {
        Value::Mapping(m) => {
            for (_k, v) in m {
                filter_map_value_visitor(v, acc, filter_map);
            }
        }
        Value::Sequence(s) => {
            for v in s {
                filter_map_value_visitor(v, acc, filter_map);
            }
        }
        _ => {}
    }
}

fn filter_mapping_visitor<FM, T>(val: &Value, acc: &mut Vec<T>, filter_map: &FM)
where
    FM: Fn(&Mapping) -> Option<T>,
{
    match val {
        Value::Mapping(m) => {
            if let Some(t) = filter_map(&m) {
                acc.push(t);
            }
            for (_k, v) in m {
                filter_mapping_visitor(v, acc, filter_map);
            }
        }
        Value::Sequence(s) => {
            for v in s {
                filter_mapping_visitor(v, acc, filter_map);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod test {
    static CONTENTS: &'static str = r#"---
a: "foo"
nested: 
  - name: a
  - name: b
---
- b: "bar"
- b: baz
- c:
    b: 4
    d: 1
    e: 2
"#;
    use super::Corpus;
    fn get_test_corpus() -> Corpus {
        let res = Corpus::from_reader(CONTENTS.as_bytes());
        assert!(res.is_ok(), "couldn't read bytestring, {:?}", res);
        res.unwrap()
    }

    #[test]
    fn can_read() {
        let corpus = get_test_corpus();
        assert_eq!(corpus.documents.len(), 2, "Didn't find two documents");
        let doc1 = &corpus.documents[0];
        let doc2 = &corpus.documents[1];
        assert!(doc1.is_mapping(), "First document was not a mapping");
        assert!(doc2.is_sequence(), "Second document was not a sequence");
        assert!(
            doc1.as_mapping().unwrap().contains_key(&"a".into()),
            "Key 'a' not in first doc mapping"
        );
        assert_eq!(
            doc2.as_sequence().unwrap().len(),
            3,
            "Second document had unexpected sequence length"
        );
    }

    #[test]
    fn can_visit_mappings() {
        let corpus = get_test_corpus();

        let count: usize = corpus.filter_map_mappings(|_m| Some(1)).iter().sum();
        assert_eq!(count, 7, "unexpected number of mappings");
    }

    #[test]
    fn can_filter_mappings() {
        let corpus = get_test_corpus();
        let b_count: usize = corpus
            .filter_map_mappings(|m| {
                if m.contains_key(&"b".into()) {
                    Some(1)
                } else {
                    None
                }
            })
            .iter()
            .sum();
        assert_eq!(b_count, 3, "unexpected count of 'b' keys");
        let values = corpus.filter_map_mappings(|m| {
            m.get(&"name".into())
                .and_then(|v| Some(v.as_str()?.to_string()))
        });
        assert_eq!(vec!["a", "b"], values);
    }

}
