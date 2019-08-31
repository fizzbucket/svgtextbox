use xml::reader::{ParserConfig, EventReader, XmlEvent};
use xml::writer::{EmitterConfig};
use xml::writer::XmlEvent as WriterXmlEvent;
use xml::attribute::OwnedAttribute;
use std::error::Error;
use std::io::Read;
use std::collections::HashMap;
use crate::{TextBox, PaddedTextBox, PaddingSpecification};

enum GeneratedEvents {
	Divider,
	Break,
	PreservedSpace 
}

impl GeneratedEvents {
	fn to_vec(&self) -> Vec<WriterXmlEvent> {
		match self {
			GeneratedEvents::Divider =>	{
				vec![WriterXmlEvent::start_element("span")
						.attr("font-family", "Spectral")
						.into(),
					WriterXmlEvent::characters("―――"),
					WriterXmlEvent::end_element().into()]},
			GeneratedEvents::Break => vec![WriterXmlEvent::characters("\n")],
			GeneratedEvents::PreservedSpace => vec![WriterXmlEvent::characters(" ")]
		}
	}

	fn from_str(s: &str) -> Option<Self> {
		match s {
			"divider" => Some(GeneratedEvents::Divider),
			"br" => Some(GeneratedEvents::Break),
			"preserved-space" => Some(GeneratedEvents::PreservedSpace),
			_ => None
		}
	}

	fn from_event(e: &XmlEvent) -> Option<Self> {
		match e {
			XmlEvent::StartElement{ref name, ..} => {
				let n = name.local_name.as_str();
				Self::from_str(n)
			},
			XmlEvent::EndElement{ref name, ..} => {
				let n = name.local_name.as_str();
				Self::from_str(n)
			},
			_ => None
		}
	}
}


fn markup_to_str(m: Vec<XmlEvent>) -> Result<String, Box<Error>> {

	let mut markup = Vec::new();
	let mut writer = EmitterConfig::new()
		.write_document_declaration(false)
		.perform_indent(false)
		.create_writer(&mut markup);

	for event in m {
		match event {
			XmlEvent::StartDocument{..} => {},
			XmlEvent::StartElement{..} if GeneratedEvents::from_event(&event).is_some() => {
				let subevents = GeneratedEvents::from_event(&event).unwrap();
				for e in subevents.to_vec() {
					writer.write(e)?;
				}
			},
			XmlEvent::EndElement {..} if GeneratedEvents::from_event(&event).is_some() => {},
			XmlEvent::StartElement{name, attributes, ..} => {
				let e2 = XmlEvent::StartElement{name, attributes, namespace: xml::namespace::Namespace::empty()};
				if let Some(e) = e2.as_writer_event() {
					writer.write(e)?;
				}
			}
			_ => {
				if let Some(e) = event.as_writer_event() {
					writer.write(e)?;
				}
			}
		}
	}

	let s = std::str::from_utf8(&markup)?;
	Ok(s.to_string())
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum TextBoxAttr<'a> {
	// note that ordering is significant here to enable
	// more specific settings to override general ones
	Width(&'a str),
	Alignment(&'a str),
	Height(&'a str),
	// nb we want these two to be overridden by
	// an explicit font size
	MinFontSize(&'a str),
	MaxFontSize(&'a str),
	FontSize(&'a str),
	FontDesc(&'a str),
	FontFamily(&'a str),
	FontVariant(&'a str),
	FontStyle(&'a str),
	FontWeight(&'a str),
}

impl <'a> TextBoxAttr<'a> {

	fn from_strs(key: &'a str, value: &'a str) -> Option<Self> {
		use TextBoxAttr::*;
		match key {
			"width" => Some(Width(value)),
			"alignment" => Some(Alignment(value)),
			"height" => Some(Height(value)),
			"font_desc" => Some(FontDesc(value)),
			"font_size" => Some(FontSize(value)),
			"font_family" => Some(FontFamily(value)),
			"font_variant" => Some(FontVariant(value)),
			"font_style" => Some(FontStyle(value)),
			"font_weight" => Some(FontWeight(value)),
			"min_font_size" => Some(MinFontSize(value)),
			"max_font_size" => Some(MaxFontSize(value)),
			_ => None
		}
	}
}


fn handle_textbox<R: Read>(parser: &mut EventReader<R>, attributes: &[OwnedAttribute] ) -> Result<Vec<XmlEvent>, Box<Error>> {
	
	let mut markup_events = Vec::new();

	loop {
		let next = parser.next()?;
		match next {
			XmlEvent::EndDocument => {break},
			XmlEvent::EndElement{ref name} if name.local_name.as_str() == "textbox" => {
				break;
			},
			_ => markup_events.push(next)
		}
	}
	
	let markup = markup_to_str(markup_events)?;
	let mut textbox = TextBox::new(&markup)?;


	let mut other_attrs: HashMap<&str, &str> = HashMap::new();
	let mut textbox_attrs: Vec<TextBoxAttr> = Vec::new();
	
	for attr in attributes.iter() {
		let name = attr.name.local_name.as_str();
		let value = attr.value.as_str();
		let a = TextBoxAttr::from_strs(name, value);
		if a.is_some() {
			textbox_attrs.push(a.expect("n12"));
		} else {
			other_attrs.insert(name, value);
		}
	}

	textbox_attrs.sort_unstable();
	
	for a in textbox_attrs {
		match a {
			TextBoxAttr::Width(s) => textbox.set_width(s)?,
			TextBoxAttr::Alignment(s) => textbox.set_alignment(s)?,
			TextBoxAttr::Height(s) => textbox.set_height(s)?,
			TextBoxAttr::FontDesc(s) => textbox.set_font_desc(s)?,
			TextBoxAttr::FontSize(s) => textbox.set_font_size(s)?,
			TextBoxAttr::FontFamily(s) => textbox.set_font_family(s)?,
			TextBoxAttr::FontVariant(s) => textbox.set_font_variant(s)?,
			TextBoxAttr::FontStyle(s) => textbox.set_font_style(s)?,
			TextBoxAttr::FontWeight(s) => textbox.set_font_weight(s)?,
			TextBoxAttr::MinFontSize(s) => textbox.set_min_font_size(s.parse::<u16>()?)?,
			TextBoxAttr::MaxFontSize(s) => textbox.set_max_font_size(s.parse::<u16>()?)?,
		};
	}

	let x = other_attrs["x"].parse::<i32>()?;
	let y = other_attrs["y"].parse::<i32>()?;

	let result = if !other_attrs.contains_key("padding") {
		textbox.to_svg_image_tag(x, y)
	} else {
		let p = other_attrs.remove("padding").unwrap();
		let padding = PaddingSpecification::from_str(p)?;
		let oa = other_attrs.iter()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect();
		let padded = PaddedTextBox {
			textbox,
			padding,
			other: oa
		};
		padded.to_svg_image_tag(x, y)
	}?;

	let mut parser = EventReader::from_str(&result);
	let mut out = Vec::new();
	loop {
		let next = parser.next()?;
		match next {
			XmlEvent::StartDocument{..} => {},
			XmlEvent::EndDocument => break,
			_ => out.push(next)
		}
	}
	
	Ok(out)
}

pub fn transform_xml(src: &str) -> Result<String, Box<Error>> {
	
	let mut parser = ParserConfig::new()
		.trim_whitespace(true)
		.whitespace_to_characters(true)
		.ignore_comments(true)
		.create_reader(src.as_bytes());

	let mut out = Vec::new();
	let mut writer = EmitterConfig::new().create_writer(&mut out);
	loop {
		let next = parser.next()?;
		match next {
			XmlEvent::EndDocument => {
				break;
			},
			XmlEvent::StartElement{ref name, ref attributes, ..} => {
				if &name.local_name != "textbox" {
					next.as_writer_event()
						.map(|e| writer.write(e)
									.or_else(Err));
				} else {
					let textbox_events = match handle_textbox(&mut parser, attributes) {
						Ok(o) => o,
						Err(e) => return Err(e)
					};
					for e in textbox_events {
						e.as_writer_event()
						 .map(|e| writer.write(e)
						 	.or_else(Err));
					}
				}
			},
			_ => {next.as_writer_event()
					.map(|e| writer.write(e)
						.or_else(Err));
			}
		}
	}

	let s = std::str::from_utf8(&out)?;
	Ok(s.to_string())

	// // we now have an svg file with possibly many base64 embeds and lots of repetition.
	// // For cleanliness's sake, we can pass it to librsvg and tidy it up.
	// let out_bytes = glib::Bytes::from(&out);
	// let input_stream = gio::MemoryInputStream::new_from_bytes(&out_bytes);

	// let base_file: Option<&gio::File> = None;
	// let cancellable: Option<&gio::Cancellable> = None;
	// let handle = librsvg::Loader::new()
	// 	.read_stream(&input_stream, base_file, cancellable)?;
 //    let renderer = librsvg::CairoRenderer::new(&handle);

 //    let intrinsic_dimensions = renderer.intrinsic_dimensions();
 //    let width = intrinsic_dimensions.width.expect("pasd").get_unitless();
 //    let height = intrinsic_dimensions.height.expect("asdf").get_unitless();
    
 //    let mut writable = Vec::new();
 //    let surface = cairo::SvgSurface::for_stream(width, height, writable);
 //    let context = cairo::Context::new(&surface);

 //    renderer
 //        .render_document(
 //            &context,
 //            &cairo::Rectangle {
 //                x: 0.0,
 //                y: 0.0,
 //                width: f64::from(width),
 //                height: f64::from(height),
 //            },
 //        )
 //        .unwrap();

 //    let o = surface
 //        .finish_output_stream()
 //        .map_err(|_e| std::fmt::Error)?;
 //    let v = match o.downcast::<Vec<u8>>() {
 //        Ok(v) => Ok(v.to_vec()),
 //        Err(_e) => Err(std::fmt::Error),
 //    }?;
	// let s = std::str::from_utf8(&v)?;
	// Ok(s.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn xml() {
		let e = r#"
		<svg width="200" height="400" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
			<textbox x="0" y="0" width="200" height="200" max_font_size="50">
				<markup>
					Hello World
				</markup>
			</textbox>
			<textbox x="0" y="200" width="200" height="200" padding="10" style="fill:red;" max_font_size="50">
				<markup>
					Hello World
				</markup>
			</textbox>
		</svg>"#;
		let p = transform_xml(e).expect("a");

		std::fs::write("hmmm.svg", p).expect("b");
	}
}
