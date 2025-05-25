use std::fmt::Display;

use log::error;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub fn log_map<E: Display>(e: E) -> Error {
    error!("{}", e);
    Error {
        message: e.to_string(),
    }
}
