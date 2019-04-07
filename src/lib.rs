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
//! // Markup should be a string in the Pango Text Attribute Markup Language.
//! // (See <https://developer.gnome.org/pango/stable/PangoMarkupFormat.html> for more).
//! // This particular example produces two lines, "Hello" and "World,"
//! // where "Hello" is larger and "World" is set in italic and coloured red.
//! let markup = "<span size=\"larger\">Hello</span>\n\
//!				  <span style=\"italic\" foreground=\"red\">World</span>"
//!				 .to_string();
//! let width = 500;
//! let height = 500;
//! let font = "Serif 12";
//! // generate an image where the text is resized to be as large as possible.
//! let svg = svgtextbox::SVGTextBox::new(markup, width, height, font).to_string();
//!
//! // The following will generate an image where the text stays at its original size
//! // and is all set in 10pt sans.
//! let markup2 = "Hello World".to_string();
//! let static_svg = svgtextbox::SVGTextBox::new(markup2, width, height, "Sans 10")
//!						.static_text_size().to_string();
//!
//! let markup3 = "It is also possible to produce a vec<u8>".to_string();
//! let vec_svg = svgtextbox::SVGTextBox::new(markup3, width, height, "Sans 10").to_bytes();
//! let markup4 = "Or even a base64-encoded string".to_string();
//! let b64_svg = svgtextbox::SVGTextBox::new(markup4, width, height, "Sans 10").to_base64();
//! ```
//!
//! It is possible to format the text layout in various ways beyond using markup.
//! The most important option is how to align it: left, centred, or right.
//!
//! ```
//! extern crate pango;
//!
//! let left_aligned = svgtextbox::SVGTextBox::new("Left".to_string(), 100, 100, "Sans 10")
//!							.set_alignment_from_str("left")
//!							.to_string();
//! let also_left = svgtextbox::SVGTextBox::new("Left".to_string(), 100, 100, "Sans 10")
//!							.set_alignment(pango::Alignment::Left)
//!							.to_string();
//! let centre_aligned = svgtextbox::SVGTextBox::new("Centre".to_string(), 100, 100, "Sans 10")
//!							.set_alignment_from_str("centre")
//!							.to_string();
//! let right_aligned = svgtextbox::SVGTextBox::new("Right".to_string(), 100, 100, "Sans 10")
//!							.set_alignment_from_str("right")
//!							.to_string();
//! ```
//! Although the typeface of the text is normally set on creation using a simple text string,
//! behind the scenes this is simply passed on to `pango::FontDescription::from_string()` to get a `pango::FontDescription`.
//! See the documentation for this for a full description.
//!
//! ```
//! let font_eg = "Minion Pro bold italic 10";
//! let other_font_eg = "Gill Sans, Gill Sans MT, 12";
//! ```
//! If necessary, it is possible to use a more complicated `pango::FontDescription` by passing it in:
//! ```
//! extern crate pango;
//!
//! let mut fancy_fd = pango::FontDescription::new();
//! fancy_fd.set_size(10 * pango::SCALE); // nb
//! fancy_fd.set_weight(pango::Weight::Book);
//! // [etc]
//! let fancy = svgtextbox::SVGTextBox::new("Fancy".to_string(), 100, 100, "")
//!							.set_font_desc(fancy_fd)
//!							.to_string();
//! ```
//! # Things to note
//!
//! * Specifying a font description that doesn't exist is not a fatal error. The closest match will be used instead. This could mean things don't look quite as you expect.
//! * Note that although an `SVGTextBox` should have a width and height defined in pixels, and will produce that in the end, under the hood the calculations are in pts.
//!		This doesn't normally matter to an end user, but do be aware that units might not be what you expect.
//!		For example, as can be seen above, calls to change the font size are passed on to pango; the unit expected there is the size in pts * `pango::SCALE`.
//! * I'm pretty sure that there _are_ memory leaks. Fixing them is one of the blockages to making this a public crate. If present, they're minor,
//! 	but still enough using this as a long-running program a bad idea.
//! * Text will not be set to a base size of more than 10000pts.
//! 	There's no particular reason for this number, but some limit was required. Unless you're making not posters but very oversized billboards, this really should be enough, since
//! 	it's ~350 cm.
use std::fs;
use std::str;

extern crate pango;
use pango::LayoutExt;
extern crate cairo;
use cairo::prelude::*;
extern crate pangocairo;


pub mod utils {

	/// Converts px to the equivalent in pts.
	/// # Examples
	///
	/// ```
	/// let five_px = 5;
	/// assert_eq!(3.75, svgtextbox::utils::px_to_pts(five_px));
	/// ```
	pub fn px_to_pts(px: i32) -> f64 {
		px as f64 * 0.75
	}

	/// Scales n by pango::SCALE.
	/// # Examples
	/// ```
	/// let unscaled_font_size = 10;
	/// let scaled = svgtextbox::utils::pango_scale(unscaled_font_size);
	/// assert_eq!(scaled, 10240);
	/// ```
	pub fn pango_scale(n: i32) -> i32 {
		n * pango::SCALE
	}

	/// Just a wrapper around `pango::FontDescription::from_string` to prevent
	/// needing to import for callers.
	/// ```
	/// let a = svgtextbox::utils::pango_font_description_from_str("Sans 10");
	/// let b = pango::FontDescription::from_string("Sans 10");
	/// assert_eq!(a, b)
	/// ```
	pub fn pango_font_description_from_str(fd: &str) -> pango::FontDescription {
		pango::FontDescription::from_string(fd)
	}
}

pub struct SVGTextBox {
	pub markup: String,
	pub width: i32,
	pub height: i32,
	pub font_desc: pango::FontDescription,
	pub alignment: Option<pango::Alignment>,
	pub grow: Option<bool>
}

impl SVGTextBox {


	/// Generate a new textbox from the options given.
	///
	/// ```
	/// let mut tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans 12");
	/// // this sets certain options;
	/// // note that some are changed:
	///	assert_eq!(tb.markup, "Hello World");
	///	assert_eq!(tb.width, svgtextbox::utils::px_to_pts(100) as i32);
	///	assert_eq!(tb.height, svgtextbox::utils::px_to_pts(100) as i32);
	///	assert_eq!(tb.font_desc, svgtextbox::utils::pango_font_description_from_str("Sans 12"));
	///	assert_eq!(tb.alignment, None);
	///	assert_eq!(tb.grow, None);
	///
	/// // Further options can be given by chaining method calls together.
	/// // For example, to have left-aligned text:
	///
	/// tb.set_alignment_from_str("left");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Left);
	/// let rendered_svg = tb.to_string();
	///
	/// // Alternatively, these can be combined into one: 
	/// 
	/// let rendered = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans")
	///						.set_alignment_from_str("left")
	///						.to_string();
	/// ```
	/// # Arguments
	///
	/// * `markup`: the text to use, formatted in [Pango Markup Language](https://developer.gnome.org/pango/stable/PangoMarkupFormat.html) if desired.
	/// * `width`: the width of the eventual image, in pixels.
	/// * `height`: the height of the eventual image, in pixels.
	/// * `font_desc_str`: a string to be passed to `pango::FontDescription::new_from_string` in order to generate a `FontDescription`.
	///
	pub fn new(markup: String, width: i32, height: i32, font_desc_str: &str) -> SVGTextBox {
		
		// need to operate in pts on an svg surface
		let pt_width = utils::px_to_pts(width);
		let pt_height = utils::px_to_pts(height);
	
		SVGTextBox {
			markup: markup,
			width: pt_width as i32,
			height: pt_height as i32,
			font_desc: utils::pango_font_description_from_str(font_desc_str),
			alignment: None,
			grow: None
		}
	}

	/// Set a new font description.
	/// ```
	/// let mut tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// let fd = pango::FontDescription::from_string("Serif");
	/// tb.set_font_desc(fd);
	/// ```
	pub fn set_font_desc(&mut self, f: pango::FontDescription) -> &mut SVGTextBox {
		self.font_desc = f;
		self
	}

	/// Set how text should be aligned.
	/// # Arguments
	/// * `a`: can be any of "left", "centre", "center", and "right". Any other string will result in left-aligned text.
	///
	/// ```
	/// let mut tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// tb.set_alignment_from_str("centre");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Center);
	/// tb.set_alignment_from_str("center");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Center);
	/// tb.set_alignment_from_str("left");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Left);
	/// tb.set_alignment_from_str("right");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Right);
	/// tb.set_alignment_from_str("bad");
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Left);
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
	/// let mut tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// tb.set_alignment(pango::Alignment::Center);
	/// assert_eq!(tb.alignment.unwrap(), pango::Alignment::Center);
	/// ```
	pub fn set_alignment(&mut self, a: pango::Alignment) -> &mut SVGTextBox {
		self.alignment = Some(a);
		self
	}

	/// Do _not_ grow or shrink text, but keep it at its original size.
	/// ```
	/// // "Hello World" will grow to fit.
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// 
	/// // "Hello World" will be set in 10 point Sans.
	/// let static_tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans 10")
	///						.static_text_size();
	/// ```
	pub fn static_text_size(&mut self) -> &mut SVGTextBox {
		self.grow = Some(false);
		self
	}

	/// Get a pango context from a Cairo one.
	fn get_pango_context(&self, context: &cairo::Context) -> Result<pango::Context, &str> {
		let pango_context = pangocairo::functions::create_context(context);
		match pango_context {
			Some(pango_context) => Ok(pango_context),
			_ => Err("Could not create pango context.")
		}
	}

	/// Get a pango layout from the context with all our choices set.
	/// This is public only to make it possible to give examples for `impl LayoutExtension`
	/// and probably shouldn't be used directly, really.
	pub fn get_layout(&self, context: &cairo::Context) -> Result<pango::Layout, &str> {
		let pango_context = self.get_pango_context(context)?;
		let layout = pango::Layout::new(&pango_context);

		let alignment = self.alignment.unwrap_or(pango::Alignment::Left);

		layout.set_markup(&self.markup);
		layout.set_font_description(&self.font_desc);
		layout.set_width(utils::pango_scale(self.width));
	 	layout.set_height(utils::pango_scale(self.height));
	    layout.set_ellipsize(pango::EllipsizeMode::End);
	    layout.set_alignment(alignment);
	    layout.set_wrap(pango::WrapMode::Word);
	    Ok(layout)
	}

	/// Convert `&self` into a Vec<u8> representing the rendered svg file.
	/// ```
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// let result = tb.to_bytes().unwrap();
	/// ```
	pub fn to_bytes(&self) -> Result<Vec<u8>, &str> {
		let mut writable = Vec::new();
		let surface = cairo::svg::RefWriter::new(self.width as f64, self.height as f64, &mut writable);
		let context = cairo::Context::new(&surface);
		let layout = self.get_layout(&context)?;

		let grow = self.grow.unwrap_or(true);

	    if grow {
		    let max_font_size = layout.max_font_size();
		    layout.change_font_size(max_font_size);
		} else {
			// There could be no size at all set.
			let current_size = layout.get_base_font_size();
			if current_size == 0 {
				layout.change_font_size(utils::pango_scale(10));
			}
		}

		// pts despite function name.
	    let (_, layout_current_height_pt) = layout.get_pixel_size();
	    let pts_remaining = self.height as f64 - layout_current_height_pt as f64;
	    let top_padding_pts = pts_remaining / 2.0;

		context.move_to(0.0, top_padding_pts);
		pangocairo::functions::show_layout(&context, &layout);
	    surface.finish();

	    Ok(writable)
	}

	/// Convert `&self` into the rendered svg file, and return as a string.
	/// A convenience method around `to_bytes`:
	/// ```
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// let svg_string = tb.to_string().unwrap();
	/// let b = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans").to_bytes().unwrap();
	// 	assert_eq!(b, svg_string.as_bytes())
	/// ```
	pub fn to_string(&self) -> Result<String, &str> {
		let v = self.to_bytes().expect("Failed to convert to bytes");
		let as_str = str::from_utf8(&v).expect("Failed to convert to string.");
		Ok(as_str.to_string())
	}

	/// Render `&self` and write to a file.
	pub fn to_file(&self, p: &str) -> std::io::Result<()> {
		let svg = self.to_bytes().expect("Failed to convert to bytes");
		fs::write(p, svg)?;
		Ok(())
	}

	/// Render `&self` and return as a base64-encoded string. Also a
	/// convenience method around `to_bytes`.
	/// ```
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans");
	/// let b64 = tb.to_base64().unwrap();
	/// ```
	pub fn to_base64(&self) -> Result<String, &str> {
		let as_bytes = self.to_bytes()?;
		Ok(base64::encode(&as_bytes))
	}

}	


pub trait LayoutExtension {
	fn change_font_size(&self, new_size: i32);
	fn max_font_size(&self) -> i32;
	fn get_base_font_size(&self) -> i32;
	fn fits(&self) -> bool;
}

impl LayoutExtension for pango::Layout {

	/// Whether this layout fits within a box of
	/// `layout.get_width()` x `layout.get_height()`.
	/// This means that the text is not ellipsized
	/// and no text or part of text goes outside the box.
	fn fits(&self) -> bool {
		// The simplest check is whether pango
		// has already decided this doesn't fit.
		if self.is_ellipsized() {
			return false;
		}
		// But Pango's interpretation of this is not ours,
		// since we're imposing the idea of a bounding box.
		// Ink extents are the size of things as printed
		// logical extents are those intended to be used for
		// positioning. (Think of a `g` extending below baseline.)
		// I think that these _logical_ extents are what get used
		// by pango in calculating whether to ellipsize.
		// But we actually want to be sure that the ink extents, not the
		// logical extents, don't go beyond the bounds of our box.
		// making a check on ellipsization something that sometimes
		// fails: text will be inked beyond the boundaries of the box,
		// even if it's not ellipsized.
		// So we need to check this also.
		
		let intended_height =  self.get_height();
		let intended_width = self.get_width();

		let (ink_extents, logical_extents) = self.get_extents();

		let x_negative = ink_extents.x < 0;
		let y_negative = ink_extents.y < 0;

		if x_negative | y_negative {
			return false;
		}
		// this is highly unlikely to be a problem, since ink dimensions 
		// are always almost less than logical, but might as well check. 
		let too_high = ink_extents.height > intended_height;
		let too_wide = ink_extents.width > intended_width;
		if too_high | too_wide {
			return false;
		}

		// We can also encounter trouble if, for example, the logical height is, when added to
		// the start point given by ink_extents.y, greater than the total intended height.
		let total_height = ink_extents.y + logical_extents.height;
		let total_width = ink_extents.x + logical_extents.width;
		if (total_height > intended_height) | (total_width > intended_width) {
			return false;
		}
		true
	}

	/// Change the base font size of this layout.
	///
	/// ```
	/// use pango::LayoutExt;
	/// use svgtextbox::LayoutExtension;
	///
	/// // get a layout
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans 10");
	/// let mut writable = Vec::new();
	/// let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	/// let context = cairo::Context::new(&surface);
	/// let layout = tb.get_layout(&context).unwrap();
	///
	/// let original_size = layout.get_base_font_size();
	/// assert_eq!(original_size, 10 * pango::SCALE);
	/// layout.change_font_size(11 * pango::SCALE);
	/// assert!(original_size != layout.get_base_font_size());
	/// assert_eq!(11 * pango::SCALE, layout.get_base_font_size());
	/// ```
	fn change_font_size(&self, new_size: i32) {
		let mut font_desc = self.get_font_description().unwrap();
		font_desc.set_size(new_size);
		self.set_font_description(&font_desc);
	}

	/// Return the largest base font size which would
	/// still avoid text not fitting the box.
	///
	/// ```
	/// use pango::LayoutExt;
	/// use svgtextbox::LayoutExtension;
	///
	/// // get a layout
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans 10");
	/// let mut writable = Vec::new();
	/// let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	/// let context = cairo::Context::new(&surface);
	/// let layout = tb.get_layout(&context).unwrap();
	///
	/// let max_size = layout.max_font_size();
	/// // A layout at max_size still fits:
	/// layout.change_font_size(max_size);
	/// assert!(layout.fits());
	/// // But text one pt larger is:
	/// layout.change_font_size(max_size + pango::SCALE);
	/// assert!(!layout.fits());
	/// ```
	fn max_font_size(&self) -> i32 {
		// can't get a binary search to work properly,
		// but this is quick enough anyway...
	    let font_pts_range = 1..10001;
	    let mut ideal = 0;
	    for i in font_pts_range {
	    	self.change_font_size(utils::pango_scale(i));
	    	if !self.fits() {
	    		break;
	    	} else {
	    		ideal = i;
	    	}
	    }
	    utils::pango_scale(ideal)
	}

	/// Get the base font size used in this layout.
	/// ```
	/// use pango::LayoutExt;
	/// use svgtextbox::LayoutExtension;
	///
	/// // get a layout
	/// let tb = svgtextbox::SVGTextBox::new("Hello World".to_string(), 100, 100, "Sans 10");
	/// let mut writable = Vec::new();
	/// let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	/// let context = cairo::Context::new(&surface);
	/// let layout = tb.get_layout(&context).unwrap();
	///
	/// assert_eq!(layout.get_base_font_size(), 10 * pango::SCALE)
	fn get_base_font_size(&self) -> i32 {
		self.get_font_description().unwrap().get_size()
	}
}



#[cfg(test)]
mod tests;
