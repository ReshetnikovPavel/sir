use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Generate,
    Stop,
    // Say,
}

impl Default for State {
    fn default() -> Self {
        Self::Generate
    }
}
