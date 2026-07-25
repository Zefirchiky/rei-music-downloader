use std::fmt::{Debug, Display};

use derive_more::{AsRef, Deref, DerefMut};
use serde::Deserialize;

pub trait Component: Debug + Display + AsRef<str> + From<String> + for<'a> From<&'a str> + for<'a> PartialEq<&'a str> + PartialEq<Self> {}

#[macro_export]
macro_rules! create_component {
    ($name:ident) => {        
        #[derive(Debug, Clone, PartialEq, AsRef, Deref, DerefMut, Deserialize)]
        #[as_ref(str)]
        pub struct $name(pub String);
        
        impl $name {
            pub fn new(name: &impl AsRef<str>) -> Self {
                Self(name.as_ref().trim().to_string())
            }
        }

        impl Component for $name {}
        
        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(&value)
            }
        }
        
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(&value)
            }
        }
        
        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                &self.as_ref() == other
            }
        }
        
        impl<'a> PartialEq<$name> for &'a str {
            fn eq(&self, other: &$name) -> bool {
                self == &other.as_ref()
            }
        }
    };
}

create_component!(Artist);

impl Display for Artist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

create_component!(RemixArtist);

impl Display for RemixArtist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} (Remix Artist)", &self.0))
    }
}

#[derive(Debug, Clone)]
pub enum ArtistOrRemixArtist {
    Artist(Artist),
    RemixArtist(RemixArtist),
}

impl AsRef<str> for ArtistOrRemixArtist {
    fn as_ref(&self) -> &str {
        match self {
            Self::Artist(a) => a.as_ref(),
            Self::RemixArtist(ra) => ra.as_ref(),
        }
    }
}

impl Display for ArtistOrRemixArtist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Self::Artist(a) => a.to_string(),
            Self::RemixArtist(ra) => ra.to_string(),
        })
    }
}

create_component!(Title);

impl Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

create_component!(Remix);

impl Display for Remix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_uppercase())
    }
}
