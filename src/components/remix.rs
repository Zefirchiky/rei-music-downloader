use std::fmt::Display;

use derive_more::{AsRef, Deref, DerefMut};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, AsRef, Deref, DerefMut, Deserialize)]
#[as_ref(str)]
pub struct Remix(String);

impl Remix {
    pub fn new(name: &impl AsRef<str>) -> Self {
        Self(name.as_ref().trim().to_string())
    }
}

impl Display for Remix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}


impl From<String> for Remix {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<&str> for Remix {
    fn from(value: &str) -> Self {
        Self::new(&value)
    }
}

impl PartialEq<&str> for Remix {
    fn eq(&self, other: &&str) -> bool {
        &self.as_ref() == other
    }
}

impl<'a> PartialEq<Remix> for &'a str {
    fn eq(&self, other: &Remix) -> bool {
        self == &other.as_ref()
    }
}