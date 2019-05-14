use crate::errors::LayoutError;
use regex::Regex;

const MAX_MARKUP_LEN: i32 = 1000;
const ACCEL_MARKER: char = '\u{00}';
const NULL_CHAR: char = '\u{0}';




/// Remove the value for $key from $sources (a hashmap) if it
/// exists and parse it as a positive integer.
macro_rules! attr_remove_to_int {
    ($key:expr, $sources:expr) => (
        match $sources.remove($key) {
            Some(v) => {
                let i: i32 = v.parse()?;
                if i > 0 {
                	Some(i)
                } else {
                	return Err(LayoutError::CouldNotTransformStrToPangoEnum{msg: v})
                }
            },
            None => None,
        })
}


pub trait StringExt {
	fn is_non_whitespace(&self) -> bool;
	fn to_pango_compatible(&self) -> Result<String, LayoutError>;
}

impl StringExt for String {
	fn is_non_whitespace(&self) -> bool {
		let non_whitespace = self.chars().any(|c| !c.is_whitespace());
		if self.is_empty() | !non_whitespace {
			return false
		}
		let bad_chars = self.contains(NULL_CHAR) | self.contains(ACCEL_MARKER);
		if bad_chars {
			return false
		}
		true
	}

	fn to_pango_compatible(&self) -> Result<String, LayoutError> {
	    if !self.is_non_whitespace() {
	    	return Err(LayoutError::StringNotPangoCompatible{msg: self.to_string()});
	    }

	    let mut trimmed = self.trim().to_string();
	    let too_long = (trimmed.len() as i32) > MAX_MARKUP_LEN;
	    if too_long {
	        return Err(LayoutError::StringTooLong {msg: self.to_string()});
    	}
	    // Fix isolated and unambiguous ampersands
	    if trimmed.contains('&') {
	        let isolated_ampersand = Regex::new(r"&(?P<w>\s+)").unwrap();
	        if isolated_ampersand.is_match(&trimmed) {
	            let n = isolated_ampersand
	                .replace_all(&trimmed, "&amp;$w")
	                .to_string();
	            trimmed = n;
	        }
	    }
    	let experimental_parse = pango::parse_markup(&trimmed, ACCEL_MARKER);
	    match experimental_parse {
	        Ok(_) => Ok(trimmed),
	        Err(_pango_err) => Err(LayoutError::StringNotPangoCompatible{msg: trimmed})
	    }
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_markup() {
        let r = "Hello \u{0}".to_string().to_pango_compatible();
        assert!(r.is_err());
    }

    #[test]
    fn test_empty_markup() {
        let r = "".to_string().to_pango_compatible();
        assert!(r.is_err());
    }

    #[test]
    fn test_all_whitespace_markup() {
        let r = "   \n    ".to_string().to_pango_compatible();
        assert!(r.is_err());
    }

    #[test]
    fn test_escaped_ampersand_markup() {
        "Trouble &amp; Strife".to_string().to_pango_compatible().unwrap();
    }

    #[test]
    fn test_isolated_ampersand_markup() {
        let r = "Trouble & Strife".to_string().to_pango_compatible().unwrap();
        assert_eq!(r, "Trouble &amp; Strife".to_string());
    }

    #[test]
    fn test_unisolated_ampersand_markup() {
        let r = "Trouble &amp Strife".to_string().to_pango_compatible();
        assert!(r.is_err());
    }

    #[test]
    fn test_unescaped_angle_brackets_markup() {
        let r = "<censored>".to_string().to_pango_compatible();
        assert!(r.is_err());
    }

    #[test]
    fn test_incomplete_span_markup() {
        let r = "<span>Trouble et Strife".to_string().to_pango_compatible();
        assert!(r.is_err());
    }
}