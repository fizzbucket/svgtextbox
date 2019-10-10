//! Conversion to rendered output.
//!
//! An implementation of `LayoutSource` can be used to produce a `RenderedTextbox`.
//!
//! # Example
//!
//! ```
//! use pango::{Alignment, FontDescription};
//! use svgtextbox3::layout::{LayoutSource, RenderedTextbox};
//!
//!
//! struct Source {
//!     font_sizes: Vec<i32>,
//!     widths: Vec<i32>,
//!     heights: Vec<i32>,
//!     font_description: FontDescription,
//!     markup: &'static str,
//!     alignment: Alignment,
//! }
//! 
//! impl LayoutSource for Source {
//!     fn possible_font_sizes<'a>(&'a self) -> Box<Iterator<Item=i32> + 'a> {
//!         Box::new(self.font_sizes.iter().cloned())
//!     }
//!     fn possible_widths<'a>(&'a self) -> Box<Iterator<Item=i32> + 'a> {
//!         Box::new(self.widths.iter().cloned())
//!     }
//!     fn possible_heights<'a>(&'a self) -> Box<Iterator<Item=i32> + 'a> {
//!         Box::new(self.heights.iter().cloned())
//!     }
//!     fn font_description(&self) -> &FontDescription {
//!         &self.font_description
//!     }
//!     fn markup(&self) -> &str {
//!         self.markup
//!     }
//!     fn alignment(&self) -> Alignment {
//!         self.alignment
//!     }
//! } 
//!
//! let new = Source {
//!     font_sizes: vec![10240, 20480],
//!     widths: vec![102400, 204800],
//!     heights: vec![102400, 204800],
//!     font_description: FontDescription::new(),
//!     markup: "Hello World",
//!     alignment: Alignment::Center,
//! };
//!
//! let mut rendered = RenderedTextbox::new(&new)
//!     .unwrap();
//! let svg = rendered.to_string();
//! let width = rendered.width;
//! let height = rendered.height;
//!
//! // It's also possible to add a background rectangle to the output, for example to provide
//! // a fill and border
//! let mut attrs = std::collections::HashMap::new();
//! attrs.insert("fill".to_string(), "red".to_string());
//! attrs.insert("stroke".to_string(), "blue".to_string());
//! rendered.insert_background_rect(&attrs).unwrap();
//! ```

use std::collections::BTreeSet;
use crate::errors::SvgTextBoxError;
use std::iter;
use pango::{Layout, EllipsizeMode, WrapMode, FontMapExt};
use pangocairo::FontMap;
use std::cmp::Ordering;
pub use interface::{LayoutSource, RenderedTextbox};


mod interface {
	use crate::errors::SvgTextBoxError;
	use std::collections::{HashMap, BTreeMap};
	use pango::{SCALE, Alignment, FontDescription};
	use super::LayoutManager;

	/// An implementation of this trait can be used to generate a layout
	pub trait LayoutSource {
	    /// All possible font sizes, where the unit is `points * pango::SCALE`
	    /// This should be ordered.
	    fn possible_font_sizes<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a>;
	    /// All possible heights (in order of preference); the unit should be `points * pango::SCALE`
	    fn possible_heights<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a>;
	    /// All possible widths (in order of preference); the unit should be `points * pango::SCALE`
	    fn possible_widths<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a>;
	    /// The font description of the text as a whole
	    fn font_description(&self) -> &FontDescription;
	    /// The text to set
	    fn markup(&self) -> &str;
	    /// The alignment of the text
	    fn alignment(&self) -> Alignment;
	    // the image output width as distinct from the textbox width (defaults to textbox width)
	    fn output_width(&self, layout_width: i32) -> f64 {
	        f64::from(layout_width / SCALE)
	    }
	    /// the image output height as distinct from the textbox height (defaults to textbox height)
	    fn output_height(&self, layout_height: i32) -> f64 {
	        f64::from(layout_height / SCALE)
	    }
	    /// the x-coordinate to place the textbox on the surface (defaults to 0.0)
	    fn output_x(&self) -> f64 {
	        0.0
	    }
	    /// the y-coordinate to place the textbox on the surface (defaults to 0.0)
	    fn output_y(&self) -> f64 {
	        0.0
	    }
	    /// whether to shift the rendered layout on the image surface to vertically center if
	    /// if the layout does not fill the space available (defaults to true)
	    fn centre_output_vertically(&self) -> bool {
	        true
	    }
	}

	/// A rendered layout, with information about its final width and height
	pub struct RenderedTextbox {
	    pub src: String,
	    pub width: f64,
	    pub height: f64,
	}

	impl RenderedTextbox {

		/// Create a new svg image from `src`
	    pub fn new(src: &impl LayoutSource) -> Result<RenderedTextbox, SvgTextBoxError> {
	        let manager = LayoutManager::new(src)?;
	        let layout = manager.get_best_fit()?;
	        let width = src.output_width(layout.get_width());
	        let height = src.output_height(layout.get_height());
	        let x = src.output_x();
	        let vertical_offset = if src.centre_output_vertically() {
	            let (ink_extents, _logical_extents) = layout.get_extents();
        		let surplus_height = layout.get_height() - ink_extents.height;
		        let top_padding = surplus_height / 2;
		        let offset = top_padding - ink_extents.y;
	            f64::from(offset / SCALE)
	        } else {
	                0.0
	        };
        	let y = src.output_y() + vertical_offset;
	        let writable = Vec::new();
	        let surface = cairo::SvgSurface::for_stream(width, height, writable);
	        let context = cairo::Context::new(&surface);
	        context.move_to(x, y);
	        pangocairo::functions::show_layout(&context, &layout);
	        let image_bytes = surface
	            .finish_output_stream()?
	            .downcast::<Vec<u8>>()
	            .map(|v| v.to_vec())?;
	        let svg = std::str::from_utf8(&image_bytes)?;
	        let image = RenderedTextbox {
	        	src: svg.to_string(),
	        	width,
	        	height
	        };
	        Ok(image)
	    }

	    /// Insert a background rectangle into the created svg image.
	    pub fn insert_background_rect(&mut self, attrs: &HashMap<String, String>) -> Result<&mut Self, SvgTextBoxError> {
	        let mut a = attrs.iter()
	        	.map(|(k, v)| (k.as_str(), v.to_string()))
	        	.collect::<BTreeMap<&str, String>>();
	        a.insert("x", "0".to_string());
	        a.insert("y", "0".to_string());
	        a.insert("width", self.width.to_string());
	        a.insert("height", self.height.to_string());
	        let b = a.iter()
	        	.map(|(k, v)| format!("{}=\"{}\"", k, v))
				.collect::<Vec<String>>()
				.join(" ");
			let padding_rect = format!("</defs>\n<g>\n<rect {}/>\n</g>", b);
						
	        self.src = self.src
	        	.replace("</defs>", &padding_rect);
	        Ok(self)
	    }
	}

	impl AsRef<str> for RenderedTextbox {
	    fn as_ref(&self) -> &str {
	        self.src.as_str()
	    }
	}

	impl std::fmt::Display for RenderedTextbox {
	    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	        write!(f, "{}", self.src)
	    }
	}
}

pub(crate) struct LayoutManager {
	dimensions: Vec<(i32, i32)>,
	font_sizes: Vec<i32>,
	base_layout: Layout
}

impl LayoutManager {
	
	pub(crate) fn new(src: &impl LayoutSource) -> Result<LayoutManager, SvgTextBoxError> {
		let fd = src.font_description();
        let possible_font_sizes = src.possible_font_sizes();
        let markup = src.markup();
        let alignment = src.alignment();
        let fontmap = FontMap::get_default()
        	.ok_or(SvgTextBoxError::UnexpectedNone)?;
        let context = fontmap.create_context()
        	.ok_or(SvgTextBoxError::UnexpectedNone)?;
        let layout = Layout::new(&context);
        layout.set_font_description(Some(fd));
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_wrap(WrapMode::Word);
        layout.set_alignment(alignment);
        layout.set_markup(markup);
        let possible_font_sizes = possible_font_sizes
        	.collect::<BTreeSet<i32>>() // want to be sure these are sorted
        	.into_iter()
        	.collect::<Vec<i32>>();
        if possible_font_sizes.is_empty() {
            return Err(SvgTextBoxError::NoValidFontSizes);
        }
        let mut possible_heights = src.possible_heights().peekable();
        let mut possible_widths = src.possible_widths().peekable();
        if possible_heights.peek().is_none() {
        	return Err(SvgTextBoxError::NoValidHeights);
        }
        if possible_widths.peek().is_none() {
        	return Err(SvgTextBoxError::NoValidWidths);
        }
        let possible_dimensions = src.possible_widths()
            .flat_map(move |v|
                iter::repeat(v)
                    .zip(src.possible_heights()))
            .collect::<Vec<(i32, i32)>>();

		Ok(LayoutManager {
			dimensions: possible_dimensions,
			font_sizes: possible_font_sizes,
			base_layout: layout
		})
	}

	pub(crate) fn get_best_fit(self) -> Result<Layout, SvgTextBoxError> {
		for (width, height) in self.dimensions {
			self.base_layout.set_width(width);
			self.base_layout.set_height(height);
			let result = self.base_layout.grow_to_maximum_font_size(&self.font_sizes);
			match result {
				Ok(_) => return Ok(self.base_layout),
				Err(SvgTextBoxError::CouldNotFit) => {},
				Err(e) => return Err(e),
			}
		}
		Err(SvgTextBoxError::CouldNotFit)
	}
}

trait LayoutExtension {
	// the current font size
    fn font_size(&self) -> i32;
    // set a new base font size for this layout
    fn set_font_size(&self, new_font_size: i32);
    /// Whether this layout currently fits
    fn fits(&self) -> bool;
    /// Change this layout's font size to `n`. If it fits, return Ordering::Less.
    /// If it does not, return Ordering::Greater.
    fn change_size_and_check_fits(&self, n: i32) -> Ordering;
    /// Grow this layout to the maximum font size that will fit
    fn grow_to_maximum_font_size(&self, possible_font_sizes: &[i32]) -> Result<(), SvgTextBoxError>;
}

impl LayoutExtension for Layout {
    
    fn fits(&self) -> bool {
        // Pango has a mystery habit of dropping lines
        // off the end if you let it.
        // so we check what the index of the char closest
        // to the bottom right is: as far as I can tell,
        // this gets you to the last utf8 byte index;
        let (_inside, last_char_index, _trailing) =
            self.xy_to_index(self.get_width(), self.get_height());

        // in an ideal world, we would just compare this last_char_index
        // to the total character count
        // and make sure that they were the same.
        // but the character count is _not_ the utf8 bytes count.
        // We have to get this from the text itself:
        let text_string = self.get_text().expect("No text");
        let dropped_chars = last_char_index != (text_string.len() as i32 - 1);
        !(self.is_ellipsized() || dropped_chars)
    }

    fn change_size_and_check_fits(&self, size: i32) -> Ordering {
        self.set_font_size(size);
        if self.fits() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    fn grow_to_maximum_font_size(&self, v: &[i32]) -> Result<(), SvgTextBoxError> {
        // this search will always return an error representing
        // the index of where in `possible_font_sizes` a notional
        // successful result would have been found -- i.e the point
        // at which preceding font sizes would fit and at which succeeding
        // font sizes would not
        let search_result = v.binary_search_by(|n| self.change_size_and_check_fits(*n));
        let index = search_result.err()
        	.ok_or(SvgTextBoxError::UnexpectedNone)?;
        // if this is at zero, no possible size would fit
        if index == 0 {
            return Err(SvgTextBoxError::CouldNotFit);
        }
        // in any other case, the last value which does fit must be that immediately preceding:
        let last_fit = index - 1;
        let r = v.get(last_fit)
        	.ok_or(SvgTextBoxError::UnexpectedNone)?;
        self.set_font_size(*r);
        Ok(())
    }

    fn font_size(&self) -> i32 {
        self.get_font_description().unwrap_or_default().get_size()
    }

    fn set_font_size(&self, new_font_size: i32) {
        let mut fd = self.get_font_description().unwrap_or_default();
        fd.set_size(new_font_size);
        self.set_font_description(Some(&fd));
    }
}

#[cfg(test)]
mod tests {
	use super::*;
	use pango::SCALE;

	#[test]
	fn new_rendered_textbox() {
		unimplemented!();
	}

	#[test]
	fn insert_background_rect() {
		unimplemented!();
	}

	#[test]
	fn rendered_textbox_display() {
		let tb = RenderedTextbox{
			src: "Test".to_string(),
			width: 100.0,
			height: 100.0
		};

		assert_eq!(tb.to_string(), tb.src);
		assert_eq!(tb.as_ref(), tb.src);
	}

	#[test]
	fn new_layout_manager() {
		unimplemented!();
	}

	#[test]
	fn get_best_fit() {
		unimplemented!();
	}

    fn create_layout_for_testing() -> Layout {
        let fontmap = pangocairo::FontMap::get_default().expect("Could not get pango fontmap");
        let context = fontmap
            .create_context()
            .expect("Could not create pango font context");
        let layout = Layout::new(&context);
        layout.set_markup("Hello World");
        layout.set_width(300 * SCALE);
        layout.set_height(150 * SCALE);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_wrap(pango::WrapMode::Word);
        layout
    }

	#[test]
	fn layout_fits() {
		let l = create_layout_for_testing();
		l.set_font_size(10 * SCALE);
		assert!(l.fits());
		let (fits, nofits): (Vec<i32>, Vec<i32>) = (22..30)
			.map(|n| n * SCALE)
			.partition(|w| {
				l.set_width(*w);
				l.fits()
			});
		for nf in nofits.iter().cloned() {
			for f in fits.iter().cloned() {
				assert!(nf < f);
			}
		}
	}


	#[test]
	fn layout_grow_to_maximum_font_size() {
		let l = create_layout_for_testing();
		let sizes = (50..60).map(|i| i * SCALE).collect::<Vec<i32>>();
		l.grow_to_maximum_font_size(&sizes).unwrap();
		assert_eq!(l.font_size(), 57344);
		l.grow_to_maximum_font_size(&vec![10 * SCALE]).unwrap();
		assert_eq!(l.font_size(), 10 * SCALE);
		let e = l.grow_to_maximum_font_size(&vec![70 * SCALE]);
		assert!(e.is_err());
		let x = l.change_size_and_check_fits(70 * SCALE);
		assert_eq!(x, Ordering::Greater);
		let y = l.change_size_and_check_fits(10 * SCALE);
		assert_eq!(y, Ordering::Less);
	}

	#[test]
	fn layout_fontsizing() {
        let layout = create_layout_for_testing();
        for i in (10..100).step_by(10) {
        	layout.set_font_size(i * SCALE);
        	assert_eq!(layout.font_size(), i * SCALE);
        }
	}

}


