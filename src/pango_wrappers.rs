use pango::{Alignment, FontDescription};
use crate::errors::SvgTextBoxError;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use std::fmt;
use std::ops::Deref;
use std::ffi::CString;

macro_rules! wrapper {
	($name:ident, $wrapped:ty, $visitor:ident) => {

		#[derive(Debug, Clone)]
		pub struct $name(pub $wrapped);


		impl From<$wrapped> for $name {
			fn from(s: $wrapped) -> Self {
				$name(s)
			}
		}

		impl Deref for $name {
			type Target = $wrapped;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		struct $visitor;

		impl<'de> Visitor<'de> for $visitor {
			type Value = $name;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("a string")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
				where E: de::Error
			{
				v.parse::<$name>()
					.map_err(de::Error::custom)
			}
		}

		impl<'de> Deserialize<'de> for $name {
		    fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
		    where
		        D: Deserializer<'de>,
		    {
		        deserializer.deserialize_str($visitor)
		    }
		}

		impl Serialize for $name {
		    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		    where
		        S: Serializer,
		    {
		        let s = self.to_string();
		        serializer.serialize_str(&s)
		    }
		}

		impl PartialEq for $name {
			fn eq(&self, other: &Self) -> bool {
        		self.to_string() == other.to_string()
    		}
		}
	};
}

wrapper!(AlignmentWrapper, Alignment, AlignmentWrapperVisitor);
wrapper!(FontDescriptionWrapper, FontDescription, FontDescriptionWrapperVisitor);

impl FromStr for AlignmentWrapper {
	type Err = SvgTextBoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left" => Ok(AlignmentWrapper(Alignment::Left)),
            "centre" | "center" => Ok(AlignmentWrapper(Alignment::Center)),
            "right" => Ok(AlignmentWrapper(Alignment::Right)),
            _ => Err(SvgTextBoxError::InvalidAlignment)
        }
    }
}

impl FromStr for FontDescriptionWrapper {
    type Err = SvgTextBoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c = CString::new(s)?;
        let f = FontDescription::from_string(c.to_str()?);
        Ok(FontDescriptionWrapper(f))
    }
}

impl Default for FontDescriptionWrapper {
    fn default() -> Self {
        FontDescriptionWrapper(FontDescription::new())
    }
}

impl Default for AlignmentWrapper {
    fn default() -> Self {
        AlignmentWrapper(Alignment::Center)
    }
}

impl fmt::Display for AlignmentWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self.deref() {
            Alignment::Center => "center",
            Alignment::Left => "left",
            Alignment::Right => "right",
            Alignment::__Unknown(_) => "unkown"
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for FontDescriptionWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.deref();
        write!(f, "{}", fd.to_string())
    }
}
