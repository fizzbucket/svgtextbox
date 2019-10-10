use crate::errors::SvgTextBoxError;
use crate::xml_fetcher::get_textbox;
use libxml::parser::{Parser, XmlParseError};
use libxml::xpath::Context;
use std::error::Error;
use libxml::tree::{Namespace, Node, Document};
use std::collections::HashMap;
use std::fmt::{self, Display};
use crate::xml_fetcher::TextBoxToRender;
use std::ptr;
use std::env;
use libxslt::stylesheet::Stylesheet;

fn parse(src: &str) -> Result<Document, SvgTextBoxError> {
	let parser = Parser::default();
	let doc = parser.parse_string(src)
		.map_err(|_| SvgTextBoxError::Xml)?;
	Ok(doc)
}

fn get_context(doc: &Document) -> Result<Context, SvgTextBoxError> {
	let context = Context::new(doc)
		.map_err(|_| SvgTextBoxError::Xml)?;
	let root = doc.get_root_element()
		.ok_or(SvgTextBoxError::Xml)?;
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
	Ok(context)
}

fn get_stylesheet(src: &str, n: &str) -> Result<Stylesheet, SvgTextBoxError> {
	let mut path = env::temp_dir();
	path.push(n);
	let p = path.to_str().unwrap();
	std::fs::write(p, src)
		.map_err(|_| SvgTextBoxError::Xml)?;
	let stylesheet = libxslt::parser::parse_file(p)
		.map_err(|_| SvgTextBoxError::Xml)?;
	Ok(stylesheet)
}

fn xml_to_textbox(tb: &mut Node, doc: &Document) -> Result<String, SvgTextBoxError> {
	let attributes = tb.get_properties();
	let attrs = attributes.iter()
		.map(|(k,v)| (k.as_str(), v.as_str()))
		.collect::<HashMap<&str, &str>>();
	let markup_elem = tb.get_first_element_child().expect("m");
	let markup = doc.node_to_string(&markup_elem);
	println!("{:?}", markup);
	let textbox = get_textbox(&markup, attrs).expect("n");
	let textbox_svg = textbox.render().expect("o");
	Ok(textbox_svg)
}


pub fn transform_xml(src: &str) -> Result<String, SvgTextBoxError> {
	let input_stylesheet_source = include_str!("textbox_input.xslt");
	let textbox_stylesheet_source = include_str!("textbox.xslt");
	let mut input_stylesheet = get_stylesheet(input_stylesheet_source, "tbi.xls")?;
	let mut textbox_stylesheet = get_stylesheet(textbox_stylesheet_source, "tb.xls")?;
	
	let d = parse(src)?;
	let mut doc = input_stylesheet.transform(&d)
		.map_err(|_| SvgTextBoxError::Xml)?;
	
	let mut context = get_context(&doc)?;
	let mut textboxes = context.findnodes("//xmlns:textbox", None).unwrap();
	for tb in textboxes.iter_mut() {
		let svg = xml_to_textbox(tb, &doc).expect("c");
		let textbox_doc = parse(&svg).expect("b");
		let textbox_doc = textbox_stylesheet.transform(&textbox_doc)
			.expect("a");
		let mut export_doc = Document::new()
			.map_err(|_| SvgTextBoxError::Xml)?;
		let mut export_node = Node::new("g", None, &export_doc)
			.map_err(|_| SvgTextBoxError::Xml)?;
		let xmlns = Namespace::new("", "http://www.w3.org/2000/svg", &mut export_node)
			.map_err(|_| SvgTextBoxError::Xml)?;
		export_node.set_namespace(&xmlns);
		export_doc.set_root_element(&export_node);
		let children = textbox_doc.get_root_element()
			.unwrap()
			.get_child_elements();
		for mut child in children {
			child.unlink();
			let mut t_node = export_doc.import_node(&mut child)
				.map_err(|_| SvgTextBoxError::Xml)?;
			export_node.add_child(&mut t_node);
		}

		export_node.unlink();
		doc.import_node(&mut export_node);
		tb.add_next_sibling(&mut export_node);
		tb.unlink()
	}

	Ok(doc.to_string(true))
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
        let p = parse(&r).unwrap();
    }
}