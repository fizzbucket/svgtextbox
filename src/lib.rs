//! # svgtextbox
//!
//! `svgtextbox` creates svg images
//! of a particular size containing formatted text.
//!
//! Its most useful feature is the ability to
//! size text automatically to fill up as much space
//! as possible without exceeding the image size, but it
//! also leverages pango's rust bindings to allow for
//! complex textual formatting.
//!
//! It was originally written to aid in the automatic creation of
//! book covers, but might be useful for anyone trying to automatically
//! do complicated text layout in images.
//!
//! It adds very few capabilities to what could be done with pango anyway,
//! but hopefully is substantially easier to use. (It would be nice to never
//! again know the joys of juggling device units, user units, functions labelled get_pixel which return points,
//! and the exact nature of ink and logical extents.)
//!
//! # Examples
//!
//!	```
//! # use svgtextbox::SVGTextBox;
//! extern crate pango;
//! // Markup should be a string in the Pango Text Attribute Markup Language.
//! // (See <https://developer.gnome.org/pango/stable/PangoMarkupFormat.html> for more).
//! // Callers will need to escape this themselves: "<censored>" will, for example, produce
//! // an error because of its unescaped angle brackets. 
//! // This particular example produces two lines, "Hello" and "World,"
//! // where "Hello" is larger and "World" is set in italic and coloured red.
//! let markup = "<span size=\"larger\">Hello</span>\n\
//!				  <span style=\"italic\" foreground=\"red\">World</span>";
//! let width = 500;
//! let height = 500;
//! // generate an image where the text is resized to be as large as possible.
//! let svg = SVGTextBox::new(markup, width, height).unwrap().to_string();
//!
//! // The following will generate an image where the text stays at its original size
//! // and is all set in 10pt sans.
//! let static_svg = SVGTextBox::new("Hello World", width, height)
//!						.unwrap()
//!						.set_font_desc(pango::FontDescription::from_string("Sans 10"))
//!						.set_static()
//!						.to_string();
//!
//! let markup3 = "It is also possible to produce bytes";
//! let vec_svg = SVGTextBox::new(markup3, width, height).unwrap().to_bytes();
//! let markup4 = "Or even a base64-encoded string";
//! let b64_svg = SVGTextBox::new(markup4, width, height).unwrap().to_base64();
//! ```
//!
//! It is possible to format the text layout in various ways beyond using markup.
//! The most important option is how to align it: left, centred, or right.
//!
//! ```
//! # use svgtextbox::SVGTextBox;
//! extern crate pango;
//!
//! let left_aligned = SVGTextBox::new("Left", 100, 100)
//!							.unwrap()
//!							.set_alignment_from_str("left");
//! let also_left = SVGTextBox::new("Left", 100, 100)
//!							.unwrap()
//!							.set_alignment(pango::Alignment::Left);
//! let centre_aligned = SVGTextBox::new("Centre", 100, 100)
//!							.unwrap()
//!							.set_alignment_from_str("centre");
//! let right_aligned = SVGTextBox::new("Right", 100, 100)
//!							.unwrap()
//!							.set_alignment_from_str("right");
//! ```
//! The typeface of the text, together with options like style and weight,
//! can be set by using a `pango::FontDescription`.
//! See the documentation for this for a full description; briefly, this can be set
//! using either a string or by creating a FontDescription and adding attributes.
//! The created font description can be used as shown below. (Note that the size of
//! a font description is only meaningful if `set_static()` is called; otherwise the
//! text size will be changed to fit the image.)
//!
//! ```
//! extern crate pango;
//! // The simplest way to create a font description.
//! let font_eg = "Minion Pro bold italic 10";
//! let minion_pro_font_desc = pango::FontDescription::from_string(font_eg);
//!	
//! let eg = svgtextbox::SVGTextBox::new("This text will be set in bold italic Minion Pro at size 10", 500, 100).unwrap()
//!									.set_font_desc(minion_pro_font_desc)
//!									.set_static();
//!
//! // More complicated font descriptions are possible. 
//!
//! let mut fancy_fd = pango::FontDescription::new();
//! fancy_fd.set_size(10 * pango::SCALE); // nb
//! fancy_fd.set_weight(pango::Weight::Book);
//! // [etc]
//! let fancy = svgtextbox::SVGTextBox::new("Fancy", 100, 100)
//!							.unwrap()
//!							.set_font_desc(fancy_fd);
//! ```
//! # Things to note
//!
//! * Specifying a font description that doesn't exist is not a fatal error. The closest match will be used instead. This could mean things don't look quite as you expect.
//! * I'm pretty sure that there _are_ memory leaks. Fixing them is one of the blockages to making this a public crate. If present, they're minor,
//! 	but still enough using this as a long-running program a bad idea.
//! * Text will not be set to a base size of more than 500 pts.
//! 	There's no particular reason for this number, but some limit was required, and that's high enough to not be a problem generally.


extern crate pango;

mod checks;
mod svgwriter;
use std::str;


pub struct SVGTextBox {
	markup: checks::Markup,
	width: checks::DistanceMeasure,
	height: checks::DistanceMeasure,
	font_desc: pango::FontDescription,
	alignment: pango::Alignment,
	grow: bool,
	ellipsize_mode: pango::EllipsizeMode,
}

impl SVGTextBox {

	/// Generate a new textbox from the options given.
	///
	/// ```
	/// let mut tb = svgtextbox::SVGTextBox::new("Hello World", 100, 100).unwrap();
	///
	/// // Further options can be given by chaining method calls together.
	/// // For example, to have left-aligned text set in italic Times New Roman:
	///
	/// tb.set_alignment_from_str("left");
	/// tb.set_font_desc(pango::FontDescription::from_string("Times New Roman italic"));
	/// let rendered_svg = tb.to_string();
	///
	/// // Alternatively, these can be combined into one: 
	/// 
	/// let rendered = svgtextbox::SVGTextBox::new("Hello World", 100, 100)
	///						.unwrap()
	///						.set_alignment_from_str("left")
	///						.set_font_desc(pango::FontDescription::from_string("Times New Roman italic"))
	///						.to_string();
	/// ```
	/// # Arguments
	///
	/// * `markup`: the text to use, formatted in [Pango Markup Language](https://developer.gnome.org/pango/stable/PangoMarkupFormat.html) if desired.
	/// * `width`: the width of the eventual image, in pixels.
	/// * `height`: the height of the eventual image, in pixels.
	///
	pub fn new(markup: &str, px_width: i32, px_height: i32) -> Result<SVGTextBox, &'static str> {
		let w = checks::DistanceMeasure::new(px_width);
		let h = checks::DistanceMeasure::new(px_height);
		let m = checks::Markup::new(markup)?;
		let alignment = pango::Alignment::Center;
		let ellipsize_mode = pango::EllipsizeMode::End;
		let font_desc = Default::default();
		let grow = true;

		Ok(SVGTextBox {
			markup: m,
			width: w,
			height: h,
			font_desc: font_desc,
			alignment: alignment,
			grow: grow,
			ellipsize_mode: ellipsize_mode,
		})
	}

	/// Set how text should be aligned.
	/// # Arguments
	/// * `a`: can be any of "left", "centre", "center", and "right". Any other string will result in left-aligned text.
	///
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let mut tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// tb.set_alignment_from_str("centre");
	/// ```
	pub fn set_alignment_from_str(&mut self, a: &str) -> &mut SVGTextBox {
		let alignment = match a {
			"left" => pango::Alignment::Left,
			"centre" | "center" => pango::Alignment::Center,
			"right" => pango::Alignment::Right,
			// might as well not panic
			_ => pango::Alignment::Left
		};
		self.set_alignment(alignment)
	}

	/// Set how text should be aligned, using a `pango::Alignment` directly.
	///
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let mut tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// tb.set_alignment(pango::Alignment::Right);
	/// ```
	pub fn set_alignment(&mut self, a: pango::Alignment) -> &mut SVGTextBox {
		self.alignment = a;
		self
	}

	/// Set a new font description.
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let mut tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// let fd = pango::FontDescription::from_string("Serif");
	/// tb.set_font_desc(fd.clone());
	/// ```
	pub fn set_font_desc(&mut self, fd: pango::FontDescription) -> &mut SVGTextBox {
		self.font_desc = fd;
		self
	}

	/// Do _not_ grow or shrink text, but keep it at its original size.
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// // "Hello World" will grow to fit.
	/// let tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// 
	/// // "Hello World" will be set in 10 point Sans.
	/// let static_tb = SVGTextBox::new("Hello World", 100, 100)
	///						.unwrap()
	///						.set_static();
	/// ```
	pub fn set_static(&mut self) -> &mut SVGTextBox {
		self.grow = false;
		self
	}

	/// Convert `&self` into a Vec<u8> representing the rendered svg file.
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// let result = tb.to_bytes().unwrap();
	/// ```
	pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
		svgwriter::generate_svg(&self.markup, &self.width, &self.height, &self.font_desc, &self.alignment, &self.grow, &self.ellipsize_mode)
	}


	/// Convert `&self` into the rendered svg file, and return as a string.
	/// A convenience method around `to_bytes`:
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// let svg_string = tb.to_string().unwrap();
	/// ```
	pub fn to_string(&self) -> Result<String, &str> {
		let v = self.to_bytes()?;
		let s = str::from_utf8(&v).expect("String conversion error");
		Ok(s.to_string())
	}

	/// Render `&self` and write to a file `p`.
	pub fn to_file<'a>(&self, p: &'a str) -> Result<&'a str, &'static str> {
		let v = self.to_bytes()?;
		std::fs::write(p, v).expect("File write error");
		Ok(p)
	}

	/// Render `&self` and return as a base64-encoded string. Also a
	/// convenience method around `to_bytes`.
	/// ```
	/// # use svgtextbox::SVGTextBox;
	/// let tb = SVGTextBox::new("Hello World", 100, 100).unwrap();
	/// let b64 = tb.to_base64().unwrap();
	/// ```
	pub fn to_base64(&self) -> Result<String, &'static str> {
		let v = self.to_bytes()?;
		Ok(base64::encode(&v))
	}


}




#[cfg(test)]
mod tests {

    use super::*;

	#[test]
	#[should_panic(expected = "Improperly formatted markup")]
	fn test_bad_markup() {
		SVGTextBox::new("<span>Hello World", 100, 100).unwrap();
	}

	#[test]
	fn test_good_markup() {
		let tb = SVGTextBox::new("A", 10, 10).unwrap();
		tb.to_string().unwrap();
	}

	#[test]
	fn test_bytes() {
		let tb = SVGTextBox::new("A", 10, 10).unwrap();
		tb.to_bytes().unwrap();
	}

	#[test]
	fn test_b64() {
		let tb = SVGTextBox::new("A", 10, 10).unwrap();
		tb.to_base64().unwrap();
	}

	#[test]
	fn test_option_setting() {
		let mut tb = SVGTextBox::new("A", 10, 10).unwrap();
		tb.set_alignment(pango::Alignment::Right);
		tb.set_font_desc(pango::FontDescription::from_string("Serif 10"));
		tb.set_static();
		assert_eq!(tb.alignment, pango::Alignment::Right);
		assert_eq!(tb.font_desc, pango::FontDescription::from_string("Serif 10"));
		assert_eq!(tb.grow, false);
	}



}


