use std::fmt::Display;

use derive_more::{AsRef, Deref, DerefMut};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, AsRef, Deref, DerefMut, Deserialize)]
#[as_ref(str)]
pub struct Track(pub String);

impl Track {
    pub fn new(name: &impl AsRef<str>) -> Self {
        Self(name.as_ref().trim().to_string())
    }
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}


impl From<String> for Track {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<&str> for Track {
    fn from(value: &str) -> Self {
        Self::new(&value)
    }
}

impl PartialEq<&str> for Track {
    fn eq(&self, other: &&str) -> bool {
        &self.as_ref() == other
    }
}

impl<'a> PartialEq<Track> for &'a str {
    fn eq(&self, other: &Track) -> bool {
        self == &other.as_ref()
    }
}
