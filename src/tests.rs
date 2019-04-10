use std::fs;
use super::*;
use proptest::prelude::*;


fn example_textbox() -> Result<SVGTextBox, &'static str> {
	SVGTextBox::new("Hello World".to_string(), 100, 100, "Serif 12")
}

fn layout_from_tb(tb: SVGTextBox) -> pango::Layout {
	let mut writable = Vec::new();
	let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	let context = cairo::Context::new(&surface);
	let layout = tb.get_layout(&context).unwrap();
	layout
}

#[test]
fn test_static_text_size() {
	let eg = example_textbox().unwrap().static_text_size().grow.unwrap();
	assert_eq!(eg, false);
	assert_eq!(example_textbox().unwrap().grow.unwrap_or(true), true)
}

#[test]
fn test_to_file() {
	let r = example_textbox().unwrap().to_file("test.svg");
	r.unwrap();
	fs::remove_file("test.svg").unwrap();
}

#[test]
fn check_get_max_size() {
	let layout = layout_from_tb(example_textbox().unwrap());
	let max_size = layout.max_font_size();
	// A layout at max_size still fits:
	layout.change_font_size(max_size).unwrap();
	assert!(layout.fits());
	// But text one pt larger is too big:
	layout.change_font_size(max_size + pango::SCALE).unwrap();
	assert!(!layout.fits());
}

#[test]
fn check_layout_change_size() {
	let layout = layout_from_tb(example_textbox().unwrap());
	let original_size = layout.get_base_font_size();
	assert_eq!(original_size, 12 * pango::SCALE);
	layout.change_font_size(11 * pango::SCALE).unwrap();
	assert!(original_size != layout.get_base_font_size());
	assert_eq!(11 * pango::SCALE, layout.get_base_font_size());
}



fn _check_layout_sizing(layout: pango::Layout) {
	let max = layout.max_font_size();
	layout.change_font_size(max).unwrap();

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
}


#[test]
fn check_layout_sizing_1() {
	let mut tb = SVGTextBox::new("SOME PUBLISHER".to_string(), 900, 900, "Spectral").unwrap();
	tb.set_alignment_from_str("centre");
	let layout_1 = layout_from_tb(tb);
	_check_layout_sizing(layout_1);
}

#[test]
fn check_layout_sizing_2() {
	let mut tb2 = SVGTextBox::new("SOME BOOK\n――\nSOME MANY NAMED AUTHOR".to_string(), 2000, 1200, "Spectral").unwrap();
	tb2.set_alignment_from_str("centre");
	let layout = layout_from_tb(tb2);
	_check_layout_sizing(layout);

}

#[test]
#[should_panic(expected = "Possible HTML entity not ending with colon.")]
fn test_bad_ampersands_in_markup() {
	SVGTextBox::new("Trouble &amp Strife".to_string(), 1000, 1000, "Serif 12").unwrap().to_bytes();
}

#[test]
fn test_escaped_ampersand_in_markup() {
	let tb = SVGTextBox::new("Trouble &amp; Strife".to_string(), 1000, 1000, "Serif 12").unwrap();
	assert_eq!(tb.markup, "Trouble &amp; Strife".to_string());
}

#[test]
fn test_unescaped_ampersand_in_markup() {
	let tb2 = SVGTextBox::new("Trouble & Strife".to_string(), 1000, 1000, "Serif 12").unwrap();
	assert_eq!(&tb2.markup, "Trouble &amp; Strife");
}


#[test]
fn test_dodgy_markup() {
	let r = SVGTextBox::new("\\".to_string(), 1000, 1000, "Serif 12");
	if !r.is_err() {
		let x = r.unwrap();
		x.to_bytes();
	}
	let e = SVGTextBox::new("Hello \u{0}".to_string(), 1000, 1000, "Serif 12");
	if !e.is_err() {
		let y = e.unwrap();
		y.to_bytes();
	}

}


proptest! {
    #[test]
    fn no_crash(s in "[^&]") {
        let r = SVGTextBox::new(s, 1000, 1000, "Serif 12");
        if !r.is_err() {
        	let x = r.unwrap().to_bytes();
        }
    }

    #[test]
    fn no_crash_2(s in "[^&]") {
        let r = SVGTextBox::new("Hello World".to_string(), 1000, 1000, &s);
        if !r.is_err() {
        	let x = r.unwrap().to_bytes();
        }
    }
}


