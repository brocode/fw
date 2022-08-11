use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
  pub tags: Option<BTreeSet<String>>,
}
