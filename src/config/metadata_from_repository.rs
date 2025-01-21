use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataFromRepository {
	pub tags: Option<BTreeSet<String>>,
}
