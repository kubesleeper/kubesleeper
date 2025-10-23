use std::collections::{BTreeMap, HashMap};

pub const KUBESLEEPER_ANNOTATION_PREFIX: &str = "kubesleeper";

pub fn extract_kube_annoations(
    raw_annotations: Option<&BTreeMap<String, String>>,
) -> HashMap<String, String> {
    raw_annotations
        .unwrap_or(&BTreeMap::new())
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix(&format!("{}/", KUBESLEEPER_ANNOTATION_PREFIX))
                .map(|new_key| (new_key.to_string(), v.clone()))
        })
        .collect()
}
