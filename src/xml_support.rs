use crate::errors::SvgTextBoxError;
use libxml::parser::{Parser};
use libxml::xpath::Context;
use libxml::tree::{Node, Document, Namespace};
use std::collections::HashMap;
use crate::textbox::{TextBox};
use serde_json::{Value, json};
use std::env;

struct ConvertedTextBox {
	prefix: String,
	image: String,
	x: u16,
	y: u16,
	width: f64,
	height: f64
}

fn convert_textbox_src(tb: &Node, doc: &Document) -> Result<ConvertedTextBox, SvgTextBoxError> {
	println!("converting...");
	let mut attributes = tb.get_properties();
	let markup_elem = tb.get_first_element_child()
		.ok_or(SvgTextBoxError::MissingMarkup)?;
	let markup = doc.node_to_string(&markup_elem);
	if markup == "<markup/>" {
		return Err(SvgTextBoxError::MissingMarkup);
	}

	let mut map: HashMap<&str, Value> = HashMap::new();
	map.insert("markup", Value::String(markup));
	println!("inserted markup");


	let width = attributes.remove("width");
	let min_width = attributes.remove("min-width")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(100))?;
	let max_width = attributes.remove("max-width")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(1000))?;
	let width_step = attributes.remove("width-step")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(10))?;

	let width = width
		.map(|v| Value::String(v))
		.unwrap_or({
			json! ({
				"min": min_width,
				"max": max_width,
				"step": width_step
			})
		});
	map.insert("width", width);

	let height = attributes.remove("height");
	let min_height = attributes.remove("min-height")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(100))?;
	let max_height = attributes.remove("max-height")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(1000))?;
	let height_step = attributes.remove("height-step")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(10))?;
	let height = height
		.map(|v| Value::String(v))
		.unwrap_or({
			json! ({
				"min": min_height,
				"max": max_height,
				"step": height_step
			})
		});
	map.insert("height", height);

	let font_size = attributes.remove("font-size");
	let min_font_size = attributes.remove("min-font-size")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(1))?;
	let max_font_size = attributes.remove("max-font-size")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(100))?;
	let font_size_step = attributes.remove("font-size-step")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(1))?;
	let font_size = font_size
		.map(|v| Value::String(v))
		.unwrap_or({
			json! ({
				"min": min_font_size,
				"max": max_font_size,
				"step": font_size_step
			})
		});
	map.insert("font-size", font_size);

	let padding = attributes.remove("padding");
	let padding_left = attributes.remove("padding-left")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;
	let padding_right = attributes.remove("padding-right")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;
	let padding_top = attributes.remove("padding-top")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;
	let padding_bottom = attributes.remove("padding_bottom")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;

	let padding = padding
		.map(|v| Value::String(v))
		.unwrap_or({
			json! ({
				"left": padding_right,
				"right": padding_left,
				"bottom": padding_bottom,
				"top": padding_top
			})
		});
	map.insert("padding", padding);

	if let Some(a) = attributes.remove("alignment") {
		map.insert("alignment", Value::String(a));
	}

	if let Some(f) = attributes.remove("font-desc") {
		map.insert("font-desc", Value::String(f));
	}

	let x = attributes.remove("x")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;
	let y = attributes.remove("y")
		.map(|i| i.parse::<u16>())
		.unwrap_or(Ok(0))?;

	let prefix = attributes.remove("__id")
		.unwrap_or("textbox-00".to_string());

	for (k, v) in attributes.iter() {
		map.insert(k.as_str(), Value::String(v.to_string()));
	}

	let serialized = serde_json::to_string_pretty(&map)?;
	println!("{}", &serialized);

	let tb: TextBox = serde_json::from_str(&serialized)?;

	println!("built tb");
	println!("{:?}", tb);	

	let textbox_standalone_svg = tb.to_svg_image()?;
	
	let out = ConvertedTextBox {
		x,
		y,
		prefix,
		image: textbox_standalone_svg.src,
		width: textbox_standalone_svg.width,
		height: textbox_standalone_svg.height,
	};
	Ok(out)
}


fn find_textboxes(doc: &Document) -> Result<Vec<Node>, SvgTextBoxError> {
	let mut context = Context::new(&doc)
		.map_err(|_| SvgTextBoxError::Xml)?;
	let root = doc.get_root_element()
		.ok_or(SvgTextBoxError::XmlNoRoot)?;
	let namespaces = root.get_namespace_declarations();
	for ns in namespaces {
		let mut prefix = ns.get_prefix();
		if prefix.is_empty() {
			prefix = "xmlns".to_string();
		}
		let href = ns.get_href();
		context.register_namespace(&prefix, &href)
			.map_err(|_| SvgTextBoxError::Xml)?;
	}
	let textboxes = context.findnodes("//xmlns:textbox", None)
		.map_err(|_| SvgTextBoxError::Xml)?;
	Ok(textboxes)
}


/// transform `textbox` elements within xml markup
pub fn transform_xml(src: &str) -> Result<String, SvgTextBoxError> {
	let parser = Parser::default();
	let doc = parser.parse_string(src)?;
	let input_stylesheet_source = include_str!("textbox_input.xslt");
	let mut path = env::temp_dir();
	path.push("tbi.xls");
	let p = path.to_str().unwrap();
	std::fs::write(p, input_stylesheet_source)
		.map_err(|_| SvgTextBoxError::Xml)?;
	let mut input_stylesheet = libxslt::parser::parse_file(p)
		.map_err(|_| SvgTextBoxError::Xml)?;

	let mut doc = input_stylesheet.transform(&doc)
		.map_err(|_| SvgTextBoxError::XsltError)?;

	for mut node in find_textboxes(&doc)?.into_iter() {
		let tb = convert_textbox_src(&node, &doc);
		match tb {
			Ok(tb) => {
				let mut n = Node::new("image", None, &doc)
					.map_err(|_| SvgTextBoxError::Xml)?;
				let b64 = base64::encode(&tb.image);
    			let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);
    			n.set_property("x", &format!("{}", tb.x))
    				.map_err(|_| SvgTextBoxError::Xml)?;
    			n.set_property("y", &format!("{}", tb.y))
    			    .map_err(|_| SvgTextBoxError::Xml)?;
    			n.set_property("width", &format!("{}", tb.width))
    			    .map_err(|_| SvgTextBoxError::Xml)?;
    			n.set_property("height", &format!("{}", tb.height))
    			    .map_err(|_| SvgTextBoxError::Xml)?;
    			let xlink = Namespace::new("xlink", "http://www.w3.org/1999/xlink", &mut n)
    				.map_err(|_| SvgTextBoxError::Xml)?;
    			n.set_property_ns("href", &prefixed_b64, &xlink)
    			    .map_err(|_| SvgTextBoxError::Xml)?;
				node.add_next_sibling(&mut n)
					.map_err(|_| SvgTextBoxError::Xml)?;
				node.unlink();
			},
			Err(SvgTextBoxError::MissingMarkup) => {
				node.unlink();
			},
			Err(e) => return Err(e)
		}
	}
	Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_xml() {
        let e = r#"
			<svg width="200" height="400" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
				<textbox x="0" y="0" width="200" height="200">
					<markup>
						Hello World
					</markup>
				</textbox>
				<textbox x="0" y="200" width="200" height="200" padding-top="10" style="fill:red;">
					<markup>
						<span style="italic">Hello</span><preserved-space/>World
						<br/><divider/><br/>
						Newline
					</markup>
				</textbox>
			</svg>"#;
        let r = transform_xml(e).unwrap();
        println!("{}", r);
        panic!();
    }
}