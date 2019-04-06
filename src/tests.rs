use std::fs;
use super::*;

use crate::utils::pango_scale;
use crate::utils::px_to_pts;


#[test]
fn test_px_to_pts() {
	let r = px_to_pts(100);
    assert_eq!(r, 75.0);
}

#[test]
fn test_pango_scale() {
	let r = pango_scale(100);
    assert_eq!(r, 100 * pango::SCALE);
}

fn example_textbox(width: i32, height: i32) -> SVGTextBox {
	SVGTextBox::new("Hello World".to_string(), width, height, "Serif 12")
}

fn get_layout(width: i32, height: i32) -> pango::Layout {
	let tb = example_textbox(width, height);
	let mut writable = Vec::new();
	let surface = cairo::svg::RefWriter::new(tb.width as f64, tb.height as f64, &mut writable);
	let context = cairo::Context::new(&surface);
	tb.get_layout(&context).unwrap()
}

#[test]
fn test_new_box() {
	let eg = example_textbox(100, 100);
	assert_eq!(eg.markup, "Hello World");
	assert_eq!(eg.width, px_to_pts(100) as i32);
	assert_eq!(eg.height, px_to_pts(100) as i32);
	assert_eq!(eg.font_desc, pango::FontDescription::from_string("Serif 12"));
	assert_eq!(eg.alignment, None);
	assert_eq!(eg.grow, None);

}

#[test]
fn test_set_font_desc() {
	let fd = pango::FontDescription::from_string("Times New Roman");
	example_textbox(100, 100).set_font_desc(fd);
}

#[test]
fn test_alignment_from_str() {
	let eg = example_textbox(100, 100).set_alignment_from_str("left").alignment.unwrap();
	let eg_center = example_textbox(100, 100).set_alignment_from_str("center").alignment.unwrap();
	let eg_centre = example_textbox(100, 100).set_alignment_from_str("centre").alignment.unwrap();
	let eg_right = example_textbox(100, 100).set_alignment_from_str("right").alignment.unwrap();
	let eg_bad = example_textbox(100, 100).set_alignment_from_str("bad").alignment.unwrap();

	assert_eq!(eg, pango::Alignment::Left);
	assert_eq!(eg_center, pango::Alignment::Center);
	assert_eq!(eg_centre, pango::Alignment::Center);
	assert_eq!(eg_right, pango::Alignment::Right);
	assert_eq!(eg_bad, pango::Alignment::Left);
}

#[test]
fn test_alignment() {
	let eg = example_textbox(100, 100).set_alignment(pango::Alignment::Right).alignment.unwrap();
	assert_eq!(eg, pango::Alignment::Right);
}

#[test]
fn test_static_text_size() {
	let eg = example_textbox(100, 100).static_text_size().grow.unwrap();
	assert_eq!(eg, false);

}

#[test]
fn test_to_bytes() {
	let _eg = example_textbox(100, 100).to_bytes();
}

#[test]
fn test_to_string() {
	let _eg = example_textbox(100, 100).to_string();
}

#[test]
fn test_to_file() {
	let r = example_textbox(100, 100).to_file("test.svg");
	r.unwrap();
	fs::remove_file("test.svg").unwrap();
}

#[test]
fn test_to_base64() {
	let _eg = example_textbox(100, 100).to_base64();
}

#[test]
fn test_layout_size() {
	let layout = get_layout(100, 100);
	let r = layout.max_font_size();
	assert_eq!(r, 22528);
}

#[test]
fn test_layout_change_font_size() {
	let layout = get_layout(100, 100);
	assert_eq!(layout.get_font_description().unwrap().get_size(), pango_scale(12));
	let new_size = pango_scale(13);
	layout.change_font_size(new_size);
	assert_eq!(layout.get_font_description().unwrap().get_size(), new_size);
}