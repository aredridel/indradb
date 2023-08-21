use std::{convert::TryFrom, sync::Arc};
use std::ops::Deref;
use std::str::FromStr;
use url::Url;

use crate::errors::{ValidationError, ValidationResult};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A URL
#[derive(Eq, PartialEq, Clone, Debug, Hash, Ord, PartialOrd)]
pub struct Identifier(pub(crate) Arc<String>);

impl Identifier {
    /// Constructs a new identifier.
    ///
    /// # Arguments
    /// * `s`: The identifier value.
    ///
    /// # Errors
    /// Returns a `ValidationError` if the identifier is longer than 255
    /// characters, or has invalid characters.
    pub fn new<S: Into<String>>(s: S) -> ValidationResult<Self> {
        let s = s.into();

        match Url::parse(s.as_str()) {
            Err(_) => Err(ValidationError::InvalidValue),
            Ok(_) => Ok(Self(Arc::new(s)))
        }
    }

    /// Constructs a new identifier, without any checks that it is valid.
    ///
    /// # Arguments
    /// * `s`: The identifier value.
    ///
    /// # Safety
    /// This function is marked unsafe because there's no verification that
    /// the identifier is valid.
    pub unsafe fn new_unchecked<S: Into<String>>(s: S) -> Self {
        Self(Arc::new(s.into()))
    }

    /// Gets a reference to the identifier value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Identifier {
    fn default() -> Self {
        Self(Arc::new("".to_string()))
    }
}

impl Deref for Identifier {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for Identifier {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl TryFrom<String> for Identifier {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (*self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Identifier, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: String = Deserialize::deserialize(deserializer)?;
        let id = unsafe { Identifier::new_unchecked(v) };
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::Identifier;
    use std::str::FromStr;

    #[test]
    fn should_create() {
        assert_eq!(Identifier::new("https://example.org/foo").unwrap().as_str(), "https://example.org/foo");
        assert!(Identifier::new("$").is_err());
    }

    #[test]
    fn should_create_unchecked() {
        unsafe {
            assert_eq!(Identifier::new_unchecked("foo").as_str(), "foo");
            assert_eq!(Identifier::new_unchecked("$").as_str(), "$");
        }
    }

    #[test]
    fn should_try_from_str() {
        assert_eq!(Identifier::try_from("https://example.org/foo".to_string()).unwrap().as_str(), "https://example.org/foo");
        assert!(Identifier::try_from("$".to_string()).is_err());
    }

    #[test]
    fn should_convert_between_identifier_and_string() {
        let id = Identifier::new("https://example.org/foo").unwrap();
        assert_eq!(Identifier::from_str("https://example.org/foo").unwrap(), id);
        assert_eq!(id.as_str(), "https://example.org/foo");
        assert_eq!(id.to_string(), "https://example.org/foo".to_string());
    }
}
