extern crate pango;

mod checks;
mod svgwriter;
use std::str;


pub struct SVGTextBox {
	pub markup: checks::Markup,
	pub width: checks::DistanceMeasure,
	pub height: checks::DistanceMeasure,
	pub font_desc: pango::FontDescription,
	pub alignment: pango::Alignment,
	pub grow: bool,
	pub ellipsize_mode: pango::EllipsizeMode,
}

impl SVGTextBox {

	pub fn new(markup: &str, px_width: f32, px_height: f32) -> Result<SVGTextBox, &'static str> {
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

	pub fn set_alignment(&mut self, a: pango::Alignment) -> &mut SVGTextBox {
		self.alignment = a;
		self
	}

	pub fn set_font_desc(&mut self, fd: pango::FontDescription) -> &mut SVGTextBox {
		self.font_desc = fd;
		self
	}

	pub fn set_static(&mut self) -> &mut SVGTextBox {
		self.grow = false;
		self
	}

	pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
		svgwriter::generate_svg(&self.markup, &self.width, &self.height, &self.font_desc, &self.alignment, &self.grow, &self.ellipsize_mode)
	}

	pub fn to_string(&self) -> Result<String, &str> {
		let v = self.to_bytes()?;
		let s = str::from_utf8(&v).expect("String conversion error");
		Ok(s.to_string())
	}

	pub fn to_file<'a>(&self, p: &'a str) -> Result<&'a str, &'static str> {
		let v = self.to_bytes()?;
		std::fs::write(p, v).expect("File write error");
		Ok(p)
	}

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
		SVGTextBox::new("<span>Hello World", 100.0, 100.0).unwrap();
	}

	#[test]
	fn test_good_markup() {
		let tb = SVGTextBox::new("A", 10.0, 10.0).unwrap();
		tb.to_string().unwrap();
	}

	#[test]
	fn test_bytes() {
		let tb = SVGTextBox::new("A", 10.0, 10.0).unwrap();
		tb.to_bytes().unwrap();
	}

	#[test]
	fn test_b64() {
		let tb = SVGTextBox::new("A", 10.0, 10.0).unwrap();
		tb.to_base64().unwrap();
	}

	#[test]
	fn test_option_setting() {
		let mut tb = SVGTextBox::new("A", 10.0, 10.0).unwrap();
		tb.set_alignment(pango::Alignment::Right);
		tb.set_font_desc(pango::FontDescription::from_string("Serif 10"));
		tb.set_static();
		assert_eq!(tb.alignment, pango::Alignment::Right);
		assert_eq!(tb.font_desc, pango::FontDescription::from_string("Serif 10"));
		assert_eq!(tb.grow, false);
	}



}


