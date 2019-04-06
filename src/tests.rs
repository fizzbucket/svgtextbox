use std::fs;
use super::*;

use crate::utils::pango_scale;
use crate::utils::px_to_pts;



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
