use crate::halo2::*;
use std::fmt;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FQN {
    region: String,
    region_idx: Option<RegionIndex>,
    namespaces: Vec<String>,
    tail: Option<String>,
}

impl fmt::Display for FQN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn clean_string(s: &str) -> String {
            s.trim()
                .replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_")
        }
        write!(f, "{}", clean_string(&self.region))?;
        if let Some(index) = self.region_idx {
            write!(f, "_{}", *index)?;
        }
        if !self.namespaces.is_empty() {
            write!(f, "__{}", clean_string(&self.namespaces.join("__")))?;
        }
        if let Some(name) = &self.tail {
            write!(f, "__{}", clean_string(name))?;
        }
        write!(f, "")
    }
}

impl FQN {
    pub fn new(
        region: &str,
        region_idx: Option<RegionIndex>,
        namespaces: &[String],
        tail: Option<String>,
    ) -> Self {
        Self {
            region: region.to_string(),
            region_idx,
            namespaces: namespaces.to_vec(),
            tail,
        }
    }
}
