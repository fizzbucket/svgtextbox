use std::collections::HashMap;
use crate::input::TextBoxInput;
use crate::input::FromHashMap;
use crate::errors::LayoutError;
use crate::textbox::LayoutBuilder;
use std::io::Cursor;
use quick_xml::Writer;
use minidom::Element;


pub trait XMLElementWriter {
    /// Write an xml element to a string
    fn write(&self) -> Result<String, LayoutError>;
}

impl XMLElementWriter for Element {
    fn write(&self) -> Result<String, LayoutError> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        self.write_to_inner(&mut writer)?;
        Ok(String::from_utf8(writer.into_inner().into_inner()).unwrap())
    }
}

pub trait XMLElementIterSearch where Self: Sized {
    /// Find the first child matching `name`, ignoring namespaces.
    fn iter_search(&self, name: &str) -> Option<&Self>;
}

impl XMLElementIterSearch for Element {
    fn iter_search(&self, name: &str) -> Option<&Self> {
        let mut target_element: Option<&Element> = None;
        for child in self.children() {
            if child.name() == name {
                target_element = Some(child);
                break;
            }
        }
        target_element
    }
}

/// Markup often has insignificant whitespace between tags or, as a result of
/// indentation, before text strings. This whitespace is treated as significant by
/// Pango, but often it really shouldn't be.
/// This function removes whitespace in markup in the following manner:
/// * _all_ whitespace is normalised to non-continguous spaces.
/// * spaces immediately after a tag opening angle bracket or before a tag closing angle bracket are removed.
/// * &lt;br/&gt; tags are replaced with a newline character.
///
/// ```xml
/// <span>
///    Indented text
/// </span>
/// <span size="smaller">
///     More text <br/>
///     Etc
/// </span>
/// ```
/// would be normalised to
///
/// ```xml
/// <span>Indented text</span><span size="smaller">More text
/// Etc</span>
/// ```
pub fn trim_insignificant_whitespace(s: &str) -> String {
	let non_whitespace = s.split_whitespace();
    let joined_whitespace = non_whitespace
                                .map(|s| &*s)
                                .collect::<Vec<&str>>()
                                .join(" ");
    let tags_joined = joined_whitespace
                        .replace("> ", ">")
                        .replace(" <", "<");
    let newlines_added = tags_joined
                            .replace("<br/>", "\n")
                            .replace("<br />", "\n");
    newlines_added
}

fn get_markup(src_elem: &Element, attrs: &mut HashMap<String, String>) -> Result<String, LayoutError> {

	let mut markup = match src_elem.iter_search("markup") {
            Some(m) => m.write()?,
            None => return Err(LayoutError::XMLCouldNotFindMarkup)
        };
    
    match attrs.remove("preserve-whitespace") {
        None => {
            markup = trim_insignificant_whitespace(&markup);
        },
        Some(s) => {
            if s == "false" {
                markup = trim_insignificant_whitespace(&markup);
            }
        }
    }
    Ok(markup)
} 


/// Generate a textbox from an xml element, then return a new element which contains
/// the textbox as an image suitable for embedding in an svg file.
pub fn from_element_to_element(src_elem: &Element) -> Result<Element, LayoutError> {

	let mut attrs = HashMap::new();
	for (k, v) in src_elem.attrs() {
		attrs.insert(k.to_string(), v.to_string());
	}

	let markup = get_markup(src_elem, &mut attrs)?;
	
	let input = TextBoxInput::new_from(markup, &mut attrs)?;
	let output = LayoutBuilder::get_layout_output(&input)?;

	let b64 = base64::encode(&output.rendered);
    let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);

    let mut output_element = Element::builder("image").build();
    output_element.set_attr("width", output.width);
    output_element.set_attr("height", output.height);
    output_element.set_attr("xlink:href", prefixed_b64);

    for (k, v) in attrs {
        output_element.set_attr(k.to_string(), v.to_string());
    }
    Ok(output_element)

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_to_element() {
        let e_in: Element = r#"<textbox x="0" y="100" width="100" height="100" font-family="Helvetica"><markup>Hello!</markup></textbox>"#.parse().unwrap();
        let e_out = from_element_to_element(&e_in).unwrap();
        assert_eq!(e_out.attr("x").unwrap(), "0");
        assert_eq!(e_out.attr("width").unwrap(), "100");
        assert_eq!(e_out.attr("height").unwrap(), "100");
        assert!(e_out.attr("font-family").is_none());
        assert!(e_out.attr("xlink:href").is_some());
        
        assert_eq!(e_out.children().next(), None);
    }

    #[test]
    fn dodgy_element() {
        let dodgy: Element = "<textbox width=\"100\" height=\"100\"><markup><mysterytag>Hello!</mysterytag></markup></textbox>".parse().unwrap();
        assert!(from_element_to_element(&dodgy).is_err());
    }



    #[test]
    fn test_element_write() {
        let in_text = "<span kind=\"test\">Hello World</span>";
        let xml: Element = in_text.parse().unwrap();
        let out_text = xml.write().unwrap();
        assert_eq!(in_text, out_text);
    }

    #[test]
    fn test_element_iter_search() {
        let x = r#"\
            <element>
                <adult>
                    Boo!
                </adult>
                 <child>
                    Hello World!
                 </child>
                 <child>
                    Goodbye!
                </child>
             </element>"#;
        let xml: Element = x.parse().unwrap();
        let result = xml.iter_search("child").expect("Found no child");
        assert_eq!(result.texts().next().unwrap().trim(), "Hello World!");
        assert_eq!(result.name(), "child");
    }

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_insignificant_whitespace("\tA"), "A");
        assert_eq!(trim_insignificant_whitespace("<span>\tA</span>"), "<span>A</span>");
        assert_eq!(trim_insignificant_whitespace("<span>A B</span>"), "<span>A B</span>");
        assert_eq!(trim_insignificant_whitespace("<span>A <br/> B</span>"), "<span>A\nB</span>");
        assert_eq!(trim_insignificant_whitespace("<span>\n\tOuter\n\t<span>\n\t\tInner\n\t</span>\n</span>"), "<span>Outer<span>Inner</span></span>");
    }


    #[test]
    fn test_other_attrs_preserved() {
        let e: Element = r#"<textbox x="0" y="100" width="100" height="100" font-family="Roman"><markup>Hello!</markup></textbox>"#.parse().unwrap();
        let out_e = from_element_to_element(&e).unwrap();
        assert_eq!(out_e.attr("x").unwrap(), "0");
        assert_eq!(out_e.attr("y").unwrap(), "100");
        assert!(out_e.attr("font-family").is_none());
    }

    #[test]
    fn test_whitespace_preservation() {
        let no_preserve: Element = "<textbox width=\"100\" height=\"100\"><markup>\t Hello!</markup></textbox>".parse().unwrap();
        let no_preserve_explicit: Element = "<textbox width=\"100\" height=\"100\" preserve-whitespace=\"false\"><markup>\t Hello!</markup></textbox>".parse().unwrap();
        let preserve: Element = "<textbox width=\"100\" height=\"100\" preserve-whitespace=\"true\"><markup>\t Hello!</markup></textbox>".parse().unwrap();

        let markup_fetch = |e: &Element| {
        	let mut attrs = HashMap::new();
        	for (k, v) in e.attrs() {
        		attrs.insert(k.to_string(), v.to_string());
        	}
        	let out = get_markup(e, &mut attrs).unwrap();
        	out
        };

        assert_eq!(markup_fetch(&no_preserve), "<markup>Hello!</markup>");
        assert_eq!(markup_fetch(&no_preserve_explicit), "<markup>Hello!</markup>");
        assert_eq!(markup_fetch(&preserve), "<markup>\t Hello!</markup>");
    }

    #[test]
    fn test_element_missing_requireds() {
        let missing_required: Element = r#"<textbox width="100"><markup>Hello!</markup></textbox>"#.parse().unwrap();
        let mr = from_element_to_element(&missing_required);
        assert!(mr.is_err());

        let missing_markup: Element = r#"<textbox width="100" height="100"></textbox>"#.parse().unwrap();
        let mm = from_element_to_element(&missing_markup);
        assert!(mm.is_err());

    }


}
