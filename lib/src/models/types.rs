use std::convert::TryFrom;
use std::str::FromStr;

use crate::errors::{ValidationError, ValidationResult};

use serde::{Deserialize, Serialize};

// TODO: rename, now that it's used for property names as well
/// An edge or vertex type.
///
/// Types must be less than 256 characters long, and can only contain letters,
/// numbers, dashes and underscores.
#[derive(Eq, PartialEq, Clone, Debug, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Type(pub String);

impl Type {
    /// Constructs a new type.
    ///
    /// # Arguments
    /// * `t`: The type, which must be less than 256 characters long.
    ///
    /// # Errors
    /// Returns a `ValidationError` if the type is longer than 255 characters,
    /// or has invalid characters.
    pub fn new<S: Into<String>>(s: S) -> ValidationResult<Self> {
        let s = s.into();

        if s.len() > 255 {
            Err(ValidationError::ValueTooLong)
        } else if !s.chars().all(|c| c == '-' || c == '_' || c.is_alphanumeric()) {
            Err(ValidationError::InvalidValue)
        } else {
            Ok(Type(s))
        }
    }

    /// Constructs a new type, without any checks that the name is valid.
    ///
    /// # Arguments
    /// * `t`: The type, which must be less than 256 characters long.
    ///
    /// # Safety
    /// This function is marked unsafe because there's no verification that
    /// the type name is valid.
    pub unsafe fn new_unchecked<S: Into<String>>(s: S) -> Self {
        Type(s.into())
    }
}

impl Default for Type {
    fn default() -> Self {
        Self { 0: "".to_string() }
    }
}

impl FromStr for Type {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl TryFrom<String> for Type {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::Type;
    use std::str::FromStr;

    #[test]
    fn should_fail_for_invalid_types() {
        let long_t = (0..256).map(|_| "X").collect::<String>();
        assert!(Type::new(long_t).is_err());
        assert!(Type::new("$").is_err());
    }

    #[test]
    fn should_convert_str_to_type() {
        assert_eq!(Type::from_str("foo").unwrap(), Type::new("foo").unwrap());
    }
}
