use std::collections::{BTreeMap, HashMap};

use crate::core::controller::constantes::*;

#[derive(Debug, Default)]
pub struct Annotations(HashMap<String, String>);

impl Annotations {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .get(&(KUBESLEEPER_ANNOTATION_PREFIX.to_string() + key))
            .map(String::as_str)
    }
}

impl From<&BTreeMap<String, String>> for Annotations {
    fn from(btree: &BTreeMap<String, String>) -> Self {
        let map = btree
            .iter()
            .filter(|(k, _)| k.starts_with(KUBESLEEPER_ANNOTATION_PREFIX))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Annotations(map)
    }
}
