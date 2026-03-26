use std::fmt::Display;

use derive_more::{AsRef, Deref, DerefMut};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, AsRef, Deref, DerefMut, Deserialize)]
#[as_ref(str)]
pub struct Artist(String);

impl Artist {
    pub fn new(name: &impl AsRef<str>) -> Self {
        Self(name.as_ref().trim().to_string())
    }
}

impl Display for Artist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Artist {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<&str> for Artist {
    fn from(value: &str) -> Self {
        Self::new(&value)
    }
}
