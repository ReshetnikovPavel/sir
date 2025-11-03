use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum State {
    #[default]
    Generate,
    Stop,
    // Say,
}
