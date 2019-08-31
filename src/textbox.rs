pub use lengths::Length;
pub use tb::TextBox;

mod lengths {

	use std::num::ParseIntError;
	use crate::utils::vec_from_str;
	use serde::{Serialize, Deserialize};
	use std::collections::HashSet;
	use pango::SCALE;
	use std::convert::TryFrom;
	use std::str::FromStr;

	/// Represent either a single length
	/// or a number of possible lengths
	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	#[serde(untagged)]
	pub enum Length {
	    /// a single length
	    Static(u16),
	    /// multiple possible lengths
	    Flex(HashSet<u16>),
	}

	impl Length {

		fn from_str(s: &str) -> Result<Self, ParseIntError> {
			let v = vec_from_str::<u16>(s)?;
			Ok(Length::from_vec(v))
		}

		fn from_vec<T>(v: Vec<T>) -> Self where
			T: Clone,
			u16: From<T>
			{
			let v: Vec<u16> = v.iter()
				.cloned()
				.map(u16::from)
				.collect();

			match v.len() {
				i if i < 1 => Length::default(),
				1 => Length::Static(v[0]),
				_ => Length::Flex(v.into_iter().collect())
			}
		}

		fn to_vec<T>(&self) -> Vec<T> where T: From<u16> + std::cmp::Ord {
			match self {
	            Length::Static(i) => vec![T::from(*i)],
	            Length::Flex(f) => {
	                let mut v = f.iter().map(|n| T::from(*n)).collect::<Vec<T>>();
	                v.sort_unstable();
	                v
	            }
	        }
		}

		/// get a sorted vec of all possible combinations of this and another length
		fn combine(&self, b: &Length) -> Vec<(i32, i32)> {
			let a = self.to_vec();
			let b = b.to_vec().into_iter();
	        let mut combined: Vec<(i32, i32)> = b
	            .flat_map(|h| a.iter().map(move |w| (*w, h)))
	            .collect();
	        combined.sort_unstable();
	        combined
		}

		/// get a sorted vec of all possible combinations of this and another length,
		/// scaled by pango::SCALE
		pub fn combine_and_pango_scale(&self, b: &Length) -> Vec<(i32, i32)> {
			self.combine(b)
				.into_iter()
				.map(|(a, b)| (a * SCALE, b * SCALE))
				.collect()
		}
	}

	impl Default for Length {
	    fn default() -> Self {
	        Length::Static(500)
	    }
	}

	impl TryFrom<&str> for Length {
		type Error = ParseIntError;
		fn try_from(s: &str) -> Result<Self, Self::Error> {
			Length::from_str(s)
		}
	}

	impl TryFrom<String> for Length {
		type Error = ParseIntError;
		fn try_from(s: String) -> Result<Self, Self::Error> {
			Length::from_str(&s)
		}
	}

	impl FromStr for Length {
		type Err = ParseIntError;
    	fn from_str(s: &str) -> Result<Self, Self::Err> {
    		Length::from_str(s)
    	}
	}

	impl <T> From<Vec<T>> for Length where u16: From<T>, T: Clone {
		fn from(v: Vec<T>) -> Length {
			Length::from_vec(v)
		}
	}

	impl From<u16> for Length {
		fn from(i: u16) -> Length {
			Length::Static(i)
		}
	}
}

mod tb {
	use crate::pango_interactions::{FontDescriptionWrapper, PangoAlignmentDef, PangoCompatibleString};
	use crate::fontsizing::FontSizing;
	use serde::Deserialize;
	use super::Length;
	use crate::layout::{LayoutSource, ExtendedLayout, ExportLayout};
	use pango::{FontDescription, Alignment, Layout, SCALE};
	use std::error::Error;
	use std::convert::TryFrom;
	use base64;

	/// A textbox which will have its text expand to fit.
	/// If it has a flexible width or height, the smallest possible
	/// dimensions will be used with the largest possible font size 
	/// that fits those dimensions.
	#[derive(Debug, Deserialize)]
	pub struct TextBox {
		/// the text of this textbox. Can be in Pango markup language.
	    pub markup: PangoCompatibleString,
	   	/// possible widths
	    #[serde(default)]
	    pub width: Length,
	   	/// possible heights
	    #[serde(default)]
	    pub height: Length,
	    #[serde(default)]
	    /// a wrapper around the font description to use
	    pub font_desc: FontDescriptionWrapper,
	    /// the alignment of the text
	    #[serde(with = "PangoAlignmentDef", default = "PangoAlignmentDef::default")]
	   	pub alignment: Alignment,
	   	/// the font sizes to try
	    #[serde(default)]
	    pub font_size: FontSizing,
	}

	impl TextBox {
		pub fn to_svg_image(&self) -> Result<(String, i32, i32), Box<Error>> {
			let layout = Layout::create_layout(self)?;
			let textbox = layout.as_string(None, None, None, None)?;
			let width = layout.get_width() / SCALE;
			let height = layout.get_height() / SCALE;
			Ok((textbox, width, height))
		}

		pub fn to_svg_image_tag(&self, x: i32, y: i32) -> Result<String, Box<Error>> {
			let (svg, width, height) = self.to_svg_image()?;
			let b64 = base64::encode(&svg);
	        let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);
	        let s = format!("<image xmlns:xlink=\"http://www.w3.org/1999/xlink\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" xlink:href=\"{}\"/>",
	            x, y, width, height, prefixed_b64);
	       	Ok(s)
		}

		pub fn new(markup: &str) -> Result<Self, Box<Error>> {
			let p = markup.parse::<PangoCompatibleString>()?;
			Ok(TextBox {
				markup: p,
				width: Length::default(),
				height: Length::default(),
				font_desc: FontDescriptionWrapper::default(),
				alignment: PangoAlignmentDef::default(),
				font_size: FontSizing::default()
			})
		}

		pub fn set_width<'a, T>(&'a mut self, w: T) -> Result<&'a mut TextBox, Box<Error>>
			where Length: TryFrom<T>,
			<Length as TryFrom<T>>::Error: Error + 'static {
			let l = Length::try_from(w)?;
			self.width = l;
			Ok(self)
		}

		pub fn set_height<'a, T>(&'a mut self, h: T) -> Result<&'a mut TextBox, Box<Error>>
			where Length: TryFrom<T>,
			<Length as TryFrom<T>>::Error: Error + 'static{
			let l = Length::try_from(h)?;
			self.height = l;
			Ok(self)
		}

		pub fn set_font_desc<'a>(&'a mut self, s: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let fd = FontDescriptionWrapper::convert(s)?;
			self.font_desc = fd;
			Ok(self)
		}

		pub fn set_font_size<'a>(&'a mut self, s: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let fs = s.parse::<FontSizing>()?;
			self.font_size = fs;
			Ok(self)
		}

		pub fn set_font_family<'a>(&'a mut self, f: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			self.font_desc = self.font_desc.set_family(f);
			Ok(self)
		}

		pub fn set_alignment<'a>(&'a mut self, a: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let alignment = match a {
				"left" | "Left" => Alignment::Left,
				"right" | "Right" => Alignment::Right,
				"center" | "Center" | "centre" | "Centre" => Alignment::Center,
				_ => Alignment::Center
			};
			self.alignment = alignment;
			Ok(self)
		}

		pub fn set_font_variant<'a>(&'a mut self, v: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let v = match v {
				"normal" | "Normal" => pango::Variant::Normal,
				"smallcaps" | "SmallCaps" | "smallCaps" | "Smallcaps" => pango::Variant::SmallCaps,
				_ => pango::Variant::Normal
			};
			self.font_desc = self.font_desc.set_variant(v);
			Ok(self)
		}

		pub fn set_font_style<'a>(&'a mut self, s: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let s = match s {
				"normal" | "Normal" => pango::Style::Normal,
				"oblique" | "Oblique" => pango::Style::Oblique,
				"italic" | "Italic" => pango::Style::Italic,
				_ => pango::Style::Normal
			};
			self.font_desc = self.font_desc.set_style(s);
			Ok(self)
		}

		pub fn set_font_weight<'a>(&'a mut self, w: &'a str) -> Result<&'a mut TextBox, Box<Error>> {
			let w = match w {
			    "thin" | "Thin" => pango::Weight::Thin,
			    "ultralight" | "Ultralight" => pango::Weight::Ultralight,
			    "light" | "Light" => pango::Weight::Light,
			    "semilight" | "Semilight" => pango::Weight::Semilight,
			    "book" | "Book" => pango::Weight::Book,
			    "normal" | "Normal" => pango::Weight::Normal,
			    "medium" | "Medium" => pango::Weight::Medium,
			    "semibold" | "Semibold" => pango::Weight::Semibold,
			    "bold" | "Bold" => pango::Weight::Bold,
			    "ultrabold" | "Ultrabold" => pango::Weight::Ultrabold,
			    "heavy" | "Heavy" => pango::Weight::Heavy,
			    "ultraheavy" | "Ultraheavy" => pango::Weight::Ultraheavy,
			    _ => pango::Weight::Normal
			};
			self.font_desc = self.font_desc.set_weight(w);
			Ok(self)
		}

		pub fn set_min_font_size<'a>(&'a mut self, s: u16) -> Result<&'a mut TextBox, Box<Error>> {
			let fs = self.font_size.clone();
			let min_set = fs.set_min_size(s);
			self.font_size = min_set;
			Ok(self)	
		}

		pub fn set_max_font_size<'a>(&'a mut self, s: u16) -> Result<&'a mut TextBox, Box<Error>> {
			let fs = self.font_size.clone();
			let max_set = fs.set_max_size(s);
			self.font_size = max_set;
			Ok(self)	
		}
	}

	impl LayoutSource for TextBox {
		fn possible_font_sizes(&self) -> Vec<i32> {
			self.font_size.to_pango_scaled_vec()
		}
		fn possible_dimensions(&self) -> Vec<(i32, i32)> {
			self.width.combine_and_pango_scale(&self.height)
		}
		fn font_description(&self) -> &FontDescription {
			self.font_desc.as_ref()
		}
		fn markup(&self) -> &str {
			self.markup.as_ref()
		}
		fn alignment(&self) -> Alignment {
			self.alignment
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_textbox_builder() {
		let markup = "Hello World";
		let mut t = TextBox::new(markup).unwrap();
		t.set_width("100".to_string()).unwrap();
		t.set_width("100 200".to_string()).unwrap();
		t.set_width(100).unwrap();
		t.set_width(vec![100_u16]).unwrap();
	}
}





