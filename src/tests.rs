use std::fs;
use super::*;

use crate::utils::pango_scale;
use crate::utils::px_to_pts;
use crate::LayoutExtension;



fn example_textbox() -> SVGTextBox {
	SVGTextBox::new("Hello World".to_string(), 100, 100, "Serif 12")
}

#[test]
fn test_static_text_size() {
	let eg = example_textbox().static_text_size().grow.unwrap();
	assert_eq!(eg, false);
	assert_eq!(example_textbox().grow.unwrap_or(true), true)

}

#[test]
fn test_to_file() {
	let r = example_textbox().to_file("test.svg");
	r.unwrap();
	fs::remove_file("test.svg").unwrap();
}

#[test]
fn check_layout_sizing() {
	let mut tb = SVGTextBox::new("SOME PUBLISHER".to_string(), 900, 900, "Spectral");
	tb.set_alignment_from_str("centre");
	let mut writable = Vec::new();
	let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	let context = cairo::Context::new(&surface);
	let layout = tb.get_layout(&context).unwrap();
	let max = layout.max_font_size();
	layout.change_font_size(max);
	
	assert!(!layout.is_ellipsized());
	let (ink_extents, logical_extents) = layout.get_extents();
	assert!(ink_extents.x >= 0);
	assert!(ink_extents.y >= 0);

	assert!(ink_extents.height <= layout.get_height());
	assert!(ink_extents.width <= layout.get_width());
	assert!((ink_extents.x + logical_extents.width) < layout.get_width());
	assert!((ink_extents.y + logical_extents.height) < layout.get_height());
}
