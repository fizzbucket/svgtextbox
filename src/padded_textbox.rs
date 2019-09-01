pub use padding::PaddingSpecification;
pub use pt::PaddedTextBox;

mod padding {

	use std::error::Error;
	use std::convert::TryFrom;
	use std::fmt::Display;
	use pango::SCALE;
	use serde::Deserialize;

	#[derive(Debug)]
	pub struct PaddingError;
	
	impl Display for PaddingError {
	    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	        write!(f, "{:?}", self)
	    }
	}

	impl Error for PaddingError {
	    fn source(&self) -> Option<&(dyn Error + 'static)> {None}
	}

	/// padding around this textbox
	#[derive(Debug, Deserialize)]
	pub struct PaddingSpecification {
	    pub left: u16,
	    pub right: u16,
	    pub top: u16,
	    pub bottom: u16
	}

	impl PaddingSpecification {
		
		fn new<T>(top: T, right: T, bottom: T, left: T) -> Self where u16: From<T> {
			PaddingSpecification {
				top: u16::from(top),
				right: u16::from(right),
				bottom: u16::from(bottom),
				left: u16::from(left)
			}
		}

		fn from_vec<T>(v: Vec<T>) -> Result<Self, PaddingError> where T: Clone + Copy, u16: From<T> {
			match v[..] {
				[s] => Ok(PaddingSpecification::new(s, s, s, s)),
				[top_and_bottom, right_and_left] => Ok(PaddingSpecification::new(top_and_bottom, right_and_left, top_and_bottom, right_and_left)),
				[top, right_and_left, bottom] => Ok(PaddingSpecification::new(top, right_and_left, bottom, right_and_left)),
				[top, right, bottom, left] => Ok(PaddingSpecification::new(top, right, bottom, left)),
				_ => Err(PaddingError),
			}
		}

		pub(crate) fn from_str(s: &str) -> Result<Self, PaddingError> {
			let v = s.split_whitespace()
                 .map(|s| s.parse::<u16>())
                 .collect::<Result<Vec<u16>, std::num::ParseIntError>>()
                 .map_err(|_| PaddingError)?;
			PaddingSpecification::from_vec(v)
		}

		pub(crate) fn horizontal_padding(&self) -> i32 {
			i32::from(self.left + self.right)
		}

		pub(crate) fn vertical_padding(&self) -> i32 {
			i32::from(self.top + self.bottom)
		}

		pub(crate) fn scaled_horizontal_padding(&self) -> i32 {
			self.horizontal_padding() * SCALE
		}

		pub(crate) fn scaled_vertical_padding(&self) -> i32 {
			self.vertical_padding() * SCALE
		}
	}

	impl TryFrom<String> for PaddingSpecification {
		type Error = PaddingError;
		fn try_from(s: String) -> Result<Self, Self::Error> {
			PaddingSpecification::from_str(&s)
		}
	}

	impl Display for PaddingSpecification {
	    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	        write!(f, "{} {} {} {}", self.top, self.right, self.bottom, self.left)
	    }
	}

	impl <T> TryFrom<Vec<T>> for PaddingSpecification
		where T: Clone + Copy, u16: From<T> {
		type Error = PaddingError;
		fn try_from(v: Vec<T>) -> Result<Self, Self::Error> {
			PaddingSpecification::from_vec(v)
		}
	}
}


mod pt {

	use serde::Deserialize;
	use pango::{SCALE, FontDescription, Alignment, Layout};
	use std::collections::HashMap;
	use crate::textbox::TextBox;
	use crate::layout::{ExtendedLayout, ExportLayout, LayoutSource};
	use std::error::Error;
	use super::PaddingSpecification;


	/// A textbox set into a rectangle with padding around it.
	/// Any key-value pair in *other* will be set as an attribute of the rectangle.
	#[derive(Debug, Deserialize)]
	pub struct PaddedTextBox {
	    pub textbox: TextBox,
	    pub padding: PaddingSpecification,
	    #[serde(flatten)]
	    pub other: HashMap<String, String>
	}

	impl PaddedTextBox {

		pub fn to_svg_image(&self) -> Result<(String, i32, i32), Box<Error>> {

			let layout = Layout::create_layout(self)?;
        	let textbox_width = layout.get_width() / SCALE;
        	let textbox_height = layout.get_height() / SCALE;

			let width = f64::from(textbox_width + self.padding.horizontal_padding());
			let height = f64::from(textbox_height + self.padding.vertical_padding());

			let x = f64::from(self.padding.left);
			let y = f64::from(self.padding.top);

			let image = layout.as_string(Some(width), Some(height), Some(x), Some(y))?;
			
			let parser = xml::reader::EventReader::from_str(&image);
			let mut out = Vec::new();
			let mut writer = xml::writer::EventWriter::new(&mut out);

			let ws = width.to_string();
			let hs = height.to_string();

			let mut padding_rect_builder = xml::writer::events::XmlEvent::start_element("rect")
				.attr("width", &ws)
				.attr("height", &hs);

			let shm: Vec<(&str, &str)> = self.other.iter().map(|(k,v)| (k.as_str(), v.as_str())).collect();

			for (k, v) in shm {
				if !(k == "x" || k == "y") {
					padding_rect_builder = padding_rect_builder.attr(k, v);
				}
			}

			let padding_rect = xml::writer::XmlEvent::from(padding_rect_builder);
			let mut events = parser.into_iter().collect::<Result<Vec<xml::reader::XmlEvent>, xml::reader::Error>>()?;

			let end_of_defs = events.iter().position(|e| {
				if let xml::reader::XmlEvent::EndElement{name} = e {
					if name.local_name.as_str() == "defs" {
						return true
					}
				};
				false
			});

			let end_of_defs = end_of_defs.expect("e");

			let after_defs = events.split_off(end_of_defs+1);

			for event in events {
				if let Some(w) = event.as_writer_event() {
					writer.write(w)?;
				}
			}

			writer.write(padding_rect)?;
			writer.write(xml::writer::XmlEvent::from(xml::writer::XmlEvent::end_element()))?;

			for event in after_defs {
				if let Some(w) = event.as_writer_event() {
					writer.write(w)?;
				}
			}

			let s = std::str::from_utf8(&out)?;
			Ok((s.to_string(), width as i32, height as i32))
		}

		pub fn to_svg_image_tag(&self, x:i32, y:i32) -> Result<String, Box<Error>> {
			let (svg, width, height) = self.to_svg_image()?;

			let b64 = base64::encode(&svg);
	        let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);
	        let s = format!("<image xmlns:xlink=\"http://www.w3.org/1999/xlink\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" xlink:href=\"{}\"/>",
	            x, y, width, height, prefixed_b64);
	       	Ok(s)
		}
	}



	impl LayoutSource for PaddedTextBox {

		fn possible_font_sizes(&self) -> Vec<i32> {
			self.textbox.possible_font_sizes()
		}

		fn font_description(&self) -> &FontDescription {
			self.textbox.font_description()
		}

		fn markup(&self) -> &str {
			self.textbox.markup()
		}

		fn alignment(&self) -> Alignment {
			self.textbox.alignment()
		}

		fn possible_dimensions(&self) -> Vec<(i32, i32)> {
			let textbox_dims = self.textbox.possible_dimensions();
			let padding_horizontal = self.padding.scaled_horizontal_padding();
			let padding_vertical = self.padding.scaled_vertical_padding();
			textbox_dims.into_iter()
				.map(|(w, h)| (w - padding_horizontal, h - padding_vertical))
				.collect()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn paddedtextbox() {
		let src = r##"{
			textbox: {
				markup: "Hello World",
				width: 100,
				height: 100,
			},
			padding: {
				top: 10,
				left: 10,
				bottom: 10,
				right: 10,
			},
			fill: red,
			stroke: blue,
		}"##;
		let p: PaddedTextBox = serde_yaml::from_str(src).unwrap();
		let (s, _w, _h) = p.to_svg_image().unwrap();
		println!("{}", s);
	}
}
