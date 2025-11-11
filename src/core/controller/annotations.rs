use std::collections::{BTreeMap, HashMap};

use crate::core::controller::constantes::*;

#[derive(Debug, Default)]
pub struct Annotations(HashMap<String, String>);

impl Annotations {
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.0.clone()
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0
            .get(&(KUBESLEEPER_ANNOTATION_PREFIX.to_string() + key))
    }

    pub fn set(&mut self, key: &str, value: String) {
        self.0
            .insert(KUBESLEEPER_ANNOTATION_PREFIX.to_string() + key, value);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
