extern crate cairo;
use cairo::prelude::*;
extern crate pangocairo;
extern crate pango;
use pango::LayoutExt;

use crate::checks::{Markup, FontSize, FontDescriptionExt, DistanceMeasure};


trait LayoutSizingExt {
	fn fits(&self) -> bool;
	fn height_units_surplus(&self) -> i32;
	fn height(&self) -> DistanceMeasure;
	fn width(&self) -> DistanceMeasure;
	fn logical_height(&self) -> DistanceMeasure;
}

impl LayoutSizingExt for pango::Layout {

	fn height(&self) -> DistanceMeasure {
		let h = self.get_height();
		let unscaled_h = h / pango::SCALE;
		DistanceMeasure::new(unscaled_h)
	}

	fn width(&self) -> DistanceMeasure {
		let w = self.get_width();
		let unscaled_w = w / pango::SCALE;
		DistanceMeasure::new(unscaled_w)
	}

	fn logical_height(&self) -> DistanceMeasure {
		let (_logical_width, logical_height) = self.get_pixel_size();
		DistanceMeasure::new(logical_height)
	}


	/// The difference between the logical height of this layout and the intended height:
	/// i.e. the vertical space remaining.
	fn height_units_surplus(&self) -> i32 {
		(self.height().as_px() - self.logical_height().as_px()) as i32
	}

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
		// Ink extents are the size of things as printed,
		// logical extents are those intended to be used for
		// positioning. (Think of a `g` extending below baseline.)
		// I think that these _logical_ extents are what get used
		// by pango in calculating whether to ellipsize.
		// This means a check on ellipsization alone is inadequate:
		// text will be inked beyond the boundaries of the box,
		// even if it's not ellipsized.
		// So we need to check this also.
		
		let intended_height = self.get_height();
		let intended_width = self.get_width();

		let (ink_extents, _logical_extents) = self.get_extents();

		// First step is to check that ink extents start within bounds.
		if (ink_extents.x < 0) | (ink_extents.y < 0) {
			return false;
		}

		// Even if we know ink extents are within bounds on left and top side,
		// we can also encounter trouble if a distance + start point spills over the bounds
		// on right or bottom.		
		let too_high = (ink_extents.height + ink_extents.y) > intended_height;
		let too_wide = (ink_extents.width + ink_extents.x) > intended_width;

		if too_high | too_wide {
			return false;
		}

		true
	}
}

trait LayoutFontExt {
	fn get_base_font_size(&self) -> Result<FontSize, i32>;
	fn change_font_size(&self, new_size: &FontSize) -> Result<(), &'static str>;
} 

impl LayoutFontExt for pango::Layout {
	
	/// Change the base font size of this layout.
	fn change_font_size(&self, new_size: &FontSize) -> Result<(), &'static str> {
		let mut font_desc: pango::FontDescription = self.get_font_description().unwrap_or(Default::default());
		font_desc.change_size(new_size);
		self.set_font_description(&font_desc);
		Ok(())
	}

	/// Get the base font size used in this layout.
	fn get_base_font_size(&self) -> Result<FontSize, i32> {
		let font_desc: pango::FontDescription = self.get_font_description().unwrap_or(Default::default());
		font_desc.fetch_size()
	}

}


trait LayoutStaticExt {
	fn set_static(&self) -> Result<FontSize, &'static str>;
}

impl LayoutStaticExt for pango::Layout {

	fn set_static(&self) -> Result<FontSize, &'static str> {
		let base_font_size = self.get_base_font_size();
		if base_font_size.is_err() {
			let default_font_size: FontSize = Default::default();
			self.change_font_size(&default_font_size).unwrap();
		}
		if !self.fits() {
			return Err("Could not fit text at static font size.");
		}
		Ok(self.get_base_font_size().unwrap())
	}
}

trait LayoutFlexExt {
	fn set_flex(&self) -> Result<FontSize, &'static str>;
	fn find_maximum_font_size(&self) -> FontSize;
	fn font_size_matcher(&self, n: &FontSize) -> std::cmp::Ordering;
}

impl LayoutFlexExt for pango::Layout {

	/// Change font size to `n`; if
	/// the layout then no longer fits, return 
	/// Ordering::Greater, otherwise Ordering::Less.
	fn font_size_matcher(&self, n: &FontSize) -> std::cmp::Ordering {
		self.change_font_size(n).unwrap();
		if self.fits() {
			return std::cmp::Ordering::Less;
		}
		std::cmp::Ordering::Greater
	}

	/// Return the largest base font size which would
	/// still avoid text not fitting the box.
	fn find_maximum_font_size(&self) -> FontSize {

		// If we don't do this, text after the first \n will sometimes disappear?
		// TODO: work out why;
		// doesn't seem to happen in a brute-force search
		// rather than a binary one. Do changes to
		// very large sizes lead pango to throw out lines?

		// The reported text is the same,
		// but the number of lines diminishes.
		self.set_single_paragraph_mode(true);

		let acceptable_font_sizes = FontSize::range();
		let font_sizes_vec = acceptable_font_sizes.collect::<Vec<FontSize>>();

		let search_result = font_sizes_vec.binary_search_by(|i| self.font_size_matcher(i));
		let mut index: i32 = search_result.err().unwrap() as i32;
		// Almost always this is an error representing a value too small;
		// but just in case...
		if !(index == 0) {
			index -= 1;
		}
		let result = &font_sizes_vec[index as usize];
		let mut fs = FontSize::new(result.scaled()).unwrap();
		self.change_font_size(&fs).unwrap();
		self.set_single_paragraph_mode(false);

		// now likely to be too big.

		while !self.fits() {
			let smaller = fs.step_down();
			if !smaller.is_err() {
				fs = smaller.unwrap();
				self.change_font_size(&fs).unwrap();
			}
		}

		fs
	}

	/// Change the text in this layout to the maximum possible size that fits
	fn set_flex(&self) -> Result<FontSize, &'static str> {
		let greatest_possible_size = self.find_maximum_font_size();
		let font_change_result = self.change_font_size(&greatest_possible_size);
		if font_change_result.is_err() {
			return Err("Error adjusting text size");
		}
		return Ok(greatest_possible_size)
	}

}

trait ContextExtension {
	fn pad_top_for<L: LayoutSizingExt>(&self, layout: &L);
}

impl ContextExtension for cairo::Context {

	fn pad_top_for<L: LayoutSizingExt>(&self, layout: &L) {
		let pts_remaining = layout.height_units_surplus();
		let top_padding_pts = pts_remaining / 2;
		self.move_to(0.0, top_padding_pts as f64);
	}
}

pub fn generate_svg(markup: &Markup, width: &DistanceMeasure, height: &DistanceMeasure,
	font_desc: &pango::FontDescription, alignment: &pango::Alignment,
	grow: &bool, ellipsize_mode: &pango::EllipsizeMode)
	-> Result<Vec<u8>, &'static str> {
	let mut writable = Vec::new();
	let surface = cairo::svg::RefWriter::new(width.as_pts().into(), height.as_pts().into(), &mut writable);
	let context = cairo::Context::new(&surface);
	let layout = pangocairo::functions::create_layout(&context).unwrap();

	layout.set_font_description(font_desc);
	layout.set_ellipsize(*ellipsize_mode);
	layout.set_alignment(*alignment);
	layout.set_markup(&markup.to_string());
	layout.set_width(width.as_scaled_pts());
 	layout.set_height(height.as_scaled_pts());

 	let sizer = match grow {
 		false => layout.set_static(),
 		true => layout.set_flex(),
 	};
 	if sizer.is_err() {
 		return Err("Could not set font size");
 	}
 	context.pad_top_for(&layout);
	pangocairo::functions::show_layout(&context, &layout);
 	surface.finish();
 	Ok(writable)
}

#[cfg(test)]
mod tests {

    use super::*;

	// utils

	fn from_markup(markup: &str, width: f64, height: f64) -> pango::Layout {
		let mut writable = Vec::new();
		let surface = cairo::svg::RefWriter::new(width, height, &mut writable);
		let context = cairo::Context::new(&surface);
		let layout = pangocairo::functions::create_layout(&context).unwrap();
		layout.set_markup(markup);
		layout.set_width(width as i32 * pango::SCALE);
		layout.set_height(height as i32 * pango::SCALE);
		layout
	}

	// contextextension

	struct LayoutSizingExtensionMock {
		height_units_surplus: i32,
	}

	impl LayoutSizingExt for LayoutSizingExtensionMock {
		fn fits(&self) -> bool {
			true
		}

		fn height(&self) -> DistanceMeasure {
			DistanceMeasure::new(0)
		}

		fn width(&self) -> DistanceMeasure {
			DistanceMeasure::new(0)
		}

		fn logical_height(&self) -> DistanceMeasure {
			DistanceMeasure::new(0)
		}

		fn height_units_surplus(&self) -> i32 {
			self.height_units_surplus
		}
	}

	#[test]
	fn test_pad_top() {
		let mock_surplus_height = 10;

		let mut writable = Vec::new();
		let surface = cairo::svg::RefWriter::new(100.0, 100.0, &mut writable);
		let context = cairo::Context::new(&surface);
		let layout_mock = LayoutSizingExtensionMock {height_units_surplus: mock_surplus_height};
		context.pad_top_for(&layout_mock);
		let (x, y) = context.get_current_point();
		assert_eq!(x, 0.0);
		assert_eq!(y, (mock_surplus_height / 2) as f64);
	}

	// layoutsizing ext

	#[test]
	fn test_layout_sizing() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		assert!(layout.fits());
		let extents = layout.get_extents();
		let (ink_extents, _logical_extents) = extents;

		let (width, height) = layout.get_pixel_size();
		assert!(width < layout.get_width());
		assert!(height < layout.get_height());

		assert!(!layout.is_ellipsized());
		assert!(ink_extents.x >= 0);
		assert!(ink_extents.y >= 0);

		assert!(ink_extents.height <= layout.get_height());
		assert!(ink_extents.width <= layout.get_width());
		assert!((ink_extents.x + ink_extents.width) < layout.get_width());
		assert!((ink_extents.y + ink_extents.height) < layout.get_height());

		let layout2 = from_markup("Hello World", 10.0, 10.0);
		assert!(!layout2.fits());
	}

	#[test]
	fn test_layout_surplus_height() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		assert_eq!(layout.height_units_surplus(), 84);
		let layout2 = from_markup("Hello World", 100.0, 16.0);
		assert_eq!(layout2.height_units_surplus(), 0);
	}

	// layout font extension

	#[test]
	fn test_change_font_size() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		let font_size: FontSize = Default::default();
		let new_font_size = FontSize::new(25 * pango::SCALE).unwrap();
		assert!(!(font_size == new_font_size));
		let r = new_font_size.scaled();
		layout.change_font_size(&new_font_size).unwrap();
		assert_eq!(layout.get_base_font_size().unwrap().scaled(), r);
	}

	#[test]
	fn test_get_unset_base_size() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		let b = layout.get_base_font_size();
		assert!(b.is_err());
	}

	#[test]
	fn test_get_set_base_size() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		let mut fd = pango::FontDescription::new();
		fd.set_size(pango::SCALE);
		layout.set_font_description(&fd);
		let b = layout.get_base_font_size().unwrap();
		assert_eq!(pango::SCALE, b.scaled());
	}


	// layout static


	#[test]
	fn static_layout_no_font_size_set() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		layout.set_static().unwrap();
		assert!(layout.get_base_font_size().unwrap().scaled() != 0);
		let default_font_size: FontSize = Default::default();
		assert_eq!(layout.get_base_font_size().unwrap().scaled(), default_font_size.scaled());
		assert_eq!(layout.get_character_count(), "Hello World".len() as i32);
	}

	#[test]
	fn static_layout_font_size_set() {
		let layout = from_markup("Hello World", 100.0, 100.0);
		layout.set_font_description(&pango::FontDescription::from_string("Sans 12"));
		layout.set_static().unwrap();
		assert_eq!(layout.get_character_count(), "Hello World".len() as i32);
	}

	#[test]
	#[should_panic(expected="Could not fit text at static font size.")]
	fn static_layout_font_size_set_too_big() {
		let layout = from_markup("Hello World", 10.0, 10.0);
		layout.set_font_description(&pango::FontDescription::from_string("Sans 12"));
		layout.set_static().unwrap();
	}

	// layout flex

	fn _layout(markup: Markup, width: f64, height: f64) -> pango::Layout {
		let mut writable = Vec::new();
		let surface = cairo::svg::RefWriter::new(width, height, &mut writable);
		let context = cairo::Context::new(&surface);
		let layout = pangocairo::functions::create_layout(&context).unwrap();
		layout.set_markup(&markup.to_string());
		layout.set_font_description(&pango::FontDescription::from_string("Spectral"));
		layout.set_width(width as i32 * pango::SCALE);
		layout.set_height(height as i32 * pango::SCALE);
		layout.set_alignment(pango::Alignment::Center);
		layout
	}


	fn _check_layout_sizing(layout: pango::Layout) {
		
		let original_char_count = layout.get_character_count();
		let original_line_count = layout.get_line_count();
		let original_text = layout.get_text();

		layout.set_flex().unwrap();

		let extents = layout.get_extents();
		let (ink_extents, _logical_extents) = extents;

		let (width, height) = layout.get_pixel_size();
		assert!(width < layout.get_width());
		assert!(height < layout.get_height());

		assert!(!layout.is_ellipsized());
		assert!(ink_extents.x >= 0);
		assert!(ink_extents.y >= 0);

		assert!(ink_extents.height <= layout.get_height());
		assert!(ink_extents.width <= layout.get_width());
		assert!((ink_extents.x + ink_extents.width) < layout.get_width());
		assert!((ink_extents.y + ink_extents.height) < layout.get_height());
		
		assert_eq!(layout.get_character_count(), original_char_count);
		println!("Orig: {:?}", original_line_count);
		println!("Final: {:?}", layout.get_line_count());
		
		assert_eq!(layout.get_text(), original_text);
		let fewer_lines = original_line_count > layout.get_line_count();
		assert!(!fewer_lines);
	}


	#[test]
	fn ink_extents() {
		let s = "SOME PUBLISHER";
		let m = Markup::new(s).unwrap();
		let width = 900.0;
		let height = 900.0;
		let layout = _layout(m, width, height);
		_check_layout_sizing(layout);
	}

	// this is an important one...
	#[test]
	fn line_count_increases_from_minimal_size() -> Result<(), &'static str> {
		let s = "SOME BOOK\n――\nSOME MANY NAMED AUTHOR";
		let m = Markup::new(s).unwrap();
		let width = 2000.0;
		let height = 1200.0;
		let layout = _layout(m, width, height);
		let original_line_count = layout.get_line_count();
		layout.set_flex()?;
		let fewer_lines = original_line_count > layout.get_line_count();
		assert!(!fewer_lines);
		Ok(())
	}


	// generate svg

	#[test]
	fn test_generate_svg() {
		let m = Markup::new("Hello World").unwrap();
		let width = DistanceMeasure::new(2000);
		let height = DistanceMeasure::new(1200);
		let font_desc = pango::FontDescription::new();
		let alignment = pango::Alignment::Center;
		let grow = true;
		let ellipsize_mode = pango::EllipsizeMode::End;
		let _svg_bytes = generate_svg(&m, &width, &height, &font_desc, &alignment, &grow, &ellipsize_mode).unwrap();

	}

}