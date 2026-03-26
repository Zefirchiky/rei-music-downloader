use std::fmt::Display;

use derive_more::{AsRef, Deref, DerefMut};
use serde::Deserialize;

use crate::Artist;

#[derive(Debug, Clone, PartialEq, AsRef, Deref, DerefMut, Deserialize)]
#[as_ref(str)]
pub struct Track(pub String);

impl Track {
    pub fn new(name: &impl AsRef<str>) -> Self {
        Self(name.as_ref().trim().to_string())
    }
    
    pub fn try_extract(&self) -> Option<(Artist, Track)> {
        let (a, t) = self.split_once(" - ")?;
        Some((a.into(), t.into()))
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
