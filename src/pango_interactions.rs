pub use pcs::{PangoCompatibleString, PangoCompatibleStringError};
pub use font_desc::FontDescriptionWrapper;
pub use alignment::PangoAlignmentDef;

mod pcs {
	use lazy_static::lazy_static;
	use pango;
	use regex::Regex;
	use std::convert::TryFrom;
	use std::error::Error;
	use serde::{Deserialize};
	use std::fmt::Display;
	use std::str::FromStr;

	lazy_static! {
	    static ref AMPERSAND_REGEX: Regex = Regex::new(r"&(?P<w>\s+)").unwrap();
	}
	static ACCEL_MARKER: char = '\u{00}';
	static NULL_CHAR: char = '\u{0}';
	static UNACCEPTABLE_CHARS: [char; 2] = [ACCEL_MARKER, NULL_CHAR];


	#[derive(Debug)]
	pub enum PangoCompatibleStringError {
		Whitespace,
		BadChar{src: String},
		Reported{src: String, source: Box<dyn Error>}
	}

	impl Display for PangoCompatibleStringError {
	    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	        write!(f, "{:?}", self)
	    }
	}

	impl Error for PangoCompatibleStringError {
	    fn source(&self) -> Option<&(dyn Error + 'static)> {
	        if let PangoCompatibleStringError::Reported{ref source, ..} = self {
	        	Some(source.as_ref())
	        } else {
	        	None
	        }
	    }
	}


	/// Pango needs c-style strings (i.e. without null chars)
	/// and no unescaped ampersands. It will fail if incompatible strings
	/// are passed to various functions. PangoCompatibleString
	/// is simply a wrapper around a string to check these requirements.
	#[derive(Debug, Deserialize, PartialEq)]
	#[serde(try_from = "String")]
	pub struct PangoCompatibleString(String);

	impl PangoCompatibleString {
		fn convert(s: &str) -> Result<Self, PangoCompatibleStringError> {
	        if s.chars().all(|c| c.is_whitespace()) {
	            return Err(PangoCompatibleStringError::Whitespace);
	        }
	        if s.chars().any(|c| UNACCEPTABLE_CHARS.contains(&c)) {
	            return Err(PangoCompatibleStringError::BadChar{src: s.to_string()});
	        }
	        let mut trimmed = s.trim().to_string();
	        // Fix isolated and unambiguous ampersands
	        if trimmed.contains('&') && AMPERSAND_REGEX.is_match(&trimmed) {
	            trimmed = AMPERSAND_REGEX
	                .replace_all(&trimmed, "&amp;$w")
	                .into_owned();
	        }
	        let experimental_parse = pango::parse_markup(&trimmed, ACCEL_MARKER);
	        match experimental_parse {
	            Ok(_) => Ok(PangoCompatibleString(trimmed)),
	            Err(pango_err) => Err(PangoCompatibleStringError::Reported{src: s.to_string(), source: Box::new(pango_err)}),
	        }
		}
	}

	impl Display for PangoCompatibleString {
	    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	        write!(f, "{}", self.as_ref())
	    }
	}

	impl AsRef<str> for PangoCompatibleString {
	    fn as_ref(&self) -> &str {
	        self.0.as_str()
	    }
	}

	impl TryFrom<String> for PangoCompatibleString {
		type Error = PangoCompatibleStringError;
		fn try_from(s: String) -> Result<Self, Self::Error> {
			PangoCompatibleString::convert(&s)
		}
	}

	impl FromStr for PangoCompatibleString {	
	    type Err = PangoCompatibleStringError;
	    fn from_str(s: &str) -> Result<Self, Self::Err> {
	    		PangoCompatibleString::convert(s)
	    }
	}
}

mod font_desc {

	use serde::{Deserialize};
	use pango::FontDescription;
	use std::fmt::{Display, Formatter, Result as FmtResult};
	use super::{PangoCompatibleString, PangoCompatibleStringError};
	use std::str::FromStr;
	use std::convert::TryFrom;

	/// A wrapper around pango's FontDescription
	#[derive(Debug, Deserialize)]
	#[serde(try_from = "String")]
	pub struct FontDescriptionWrapper(FontDescription);

	impl FontDescriptionWrapper {
		pub fn convert(s: &str) -> Result<FontDescriptionWrapper, PangoCompatibleStringError> {
			let s = s.parse::<PangoCompatibleString>()?;
	        let p = FontDescription::from_string(s.as_ref());
	        Ok(FontDescriptionWrapper(p))
		}

		pub fn set_family(&mut self, s: &str) -> Self {
			let FontDescriptionWrapper(fd) = self;
			fd.set_family(s);
			FontDescriptionWrapper(fd.clone())
		}

		pub fn set_variant(&mut self, v: pango::Variant) -> Self {
			let FontDescriptionWrapper(fd) = self;
			fd.set_variant(v);
			FontDescriptionWrapper(fd.clone())
		}

		pub fn set_style(&mut self, s: pango::Style) -> Self {
			let FontDescriptionWrapper(fd) = self;
			fd.set_style(s);
			FontDescriptionWrapper(fd.clone())
		}

		pub fn set_weight(&mut self, w: pango::Weight) -> Self {
			let FontDescriptionWrapper(fd) = self;
			fd.set_weight(w);
			FontDescriptionWrapper(fd.clone())
		}
	}


	impl Default for FontDescriptionWrapper {
	    fn default() -> Self {
	        FontDescriptionWrapper(FontDescription::new())
	    }
	}

	impl Display for FontDescriptionWrapper {
	    fn fmt(&self, f: &mut Formatter) -> FmtResult {
	        write!(f, "{}", self.0.to_string())
	    }
	}

	impl AsRef<FontDescription> for FontDescriptionWrapper {
	    fn as_ref(&self) -> &FontDescription {
	        &self.0
	    }
	}

	impl TryFrom<String> for FontDescriptionWrapper {
		type Error = PangoCompatibleStringError;
		fn try_from(s: String) -> Result<Self, Self::Error> {
			FontDescriptionWrapper::convert(&s)
		}
	}

	impl FromStr for FontDescriptionWrapper {	
	    type Err = PangoCompatibleStringError;
	    fn from_str(s: &str) -> Result<Self, Self::Err> {
	    	FontDescriptionWrapper::convert(s)
	    }
	}
}

mod alignment {

	use serde::{Deserialize};
	use pango::Alignment;

	#[derive(Debug, Deserialize)]
	#[serde(remote = "Alignment", )]
	pub enum PangoAlignmentDef {
	    Left,
	    Center,
	    Right,
	    __Unknown(i32),
	}

	impl PangoAlignmentDef {
		pub fn default() -> Alignment {
			Alignment::Center
		}
	}
}

#[cfg(test)]
mod pcs_tests {
	use super::*;

	#[test]
	fn name() {
		unimplemented!();
	}
}

#[cfg(test)]
mod font_desc_tests {
	use super::*;

	#[test]
	fn name() {
		unimplemented!();
	}
}

#[cfg(test)]
mod alignment_tests {
	use super::*;
	use pango::Alignment;
	use serde::Deserialize;

	#[derive(Deserialize)]
	struct TestingAlignment {
	    #[serde(with = "PangoAlignmentDef", default="PangoAlignmentDef::default")]
		alignment: Alignment
	}

	#[test]
	fn test_serde() {
		let i = "{alignment: Left}";
		let o: TestingAlignment = serde_yaml::from_str(&i).unwrap();
		assert_eq!(o.alignment, Alignment::Left);
		let i = "{}";
		let o: TestingAlignment = serde_yaml::from_str(&i).unwrap();
		assert_eq!(o.alignment, PangoAlignmentDef::default());

	}
}





