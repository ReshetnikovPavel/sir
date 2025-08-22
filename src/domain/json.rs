use serde_json::Value;

pub type JsonObject<F = Value> = serde_json::Map<String, F>;

