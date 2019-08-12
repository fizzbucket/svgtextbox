use std::collections::HashMap;
use crate::input::TextBoxInput;
use crate::input::{FromHashMap, LayoutDimensions};
use crate::errors::LayoutError;
use crate::textbox::LayoutBuilder;
use std::io::Cursor;
use quick_xml::Writer;
use minidom::Element;
use std::collections::HashSet;

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
/// * &lt;preserved-space/&gt; tags are replaced with a single space
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
    let spaces_added = tags_joined
                            .replace("<br/>", "\n")
                            .replace("<br />", "\n")
                            .replace("<preserved-space/>", " ")
                            .replace("<preserved-space />", " ");

    spaces_added
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
/// the textbox as an image tag suitable for embedding in an svg file.
///
/// The xml element should have a child tag `markup` containing pango-like markup for the textbox.
/// (Note that the markup is only _pango-like_: a new tag `&lt;br/&gt;` is used to indicate a newline, since
/// all whitespace will be normalised unless the `preserve-whitepace` attribute is set.)
/// # Attributes
///
/// ## Compulsory
/// * `width`: the width in pixels of the textbox.
/// * `height`: the height in pixels of the textbox.
///
/// The eventual value of both these distances can be either a single fixed number, a range of possibilities between a minimum and maximum distance,
/// or an arbitrary number of possibilities.
///
/// A single number is specified like so: `width="100"`.
///
/// A range should be minimum and maximum seperated by a space: `width="100 200"` means a range of `100..=200`.
///
/// Specified possibilities should also be seperated by a space: `width="100 150 200"`.
///
/// To indicate two possibilities
/// rather than a range of possibilities, place the maximum value first: `width="200 100"` means `a width of either 200 or 100`,
/// while `width="100 200"` means `a width of any value between 100 and 200`.
///
/// ## Optional
/// * `preserve-whitespace`: if set to any value other than `false`, do _not_ remove insignificant whitespace from the markup.
/// * `font-family`: the family of font to use as the base.
/// * `font-size`: the font size in pts to use. If this is set, the values (if any) of `min-size` and `max-size` are ignored and text
///    sizes will not be increased to fit but instead left static.
/// * `min-size`: the minimum font size in pts, if a range of font sizes are possible. Defaults to `DEFAULT_MIN_FONT_SIZE`.
/// * `max-size`: the maximum font size in pts, if a range of font sizes are possible. Defaults to `DEFAULT_MAX_FONT_SIZE`.
/// * `font-style`: the base style of font (`normal`, `italic`, `oblique`). Defaults to normal.
/// * `font-weight`: the base font weight. Can be either numeric (as in css values) or use names: `bold` and `700` are equivalent. Defaults to 400.
/// * `font-variant`: the font variant to use (`normal` or `smallcaps`). Defaults to normal.
/// * `font-stretch`: the stretch to use (e.g. `condensed`). Defaults to normal.
/// * `alignment`: the alignment of the text (`left`, `center` or `centre`, `right`). Defaults to centre.
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

/// Generate a textbox with a background.
///
/// This is a wrapper which allows setting a background, fill,
/// padding, seperators after etc around a produced textbox.
///
/// In `from_element_to_element` attributes not used are simply passed through to the resulting `image` tag.
/// This wrapper, however, collects the attributes `padding`, `x` and `y.` All are compulsory.
/// The optional attributes "border-top" and "border-bottom" [= hex of border colour], as well as "border-width" = [int]
/// will also be used to draw a line functioning as a border if present.
/// TODO: add seperator line.
/// Any other attributes
/// not used in the production of a textbox will be applied to the `rect` element instead; these might include, for example,
/// `style` or `fill`, to be applied to the `rect`.
///
/// Imagine a tag like this:
/// ```xml
/// <textbox x="0" y="0" width="100" height="100" style="fill: red;" padding="10">
/// <markup>Hello!</markup>
/// </textbox>
/// ```
/// The idea is that instead of this:
/// ```xml
/// <image x="0" y="0" width="100" height="100" style="fill: red;" padding="10px" xlink:href="[data]"><image/>
/// ```
/// You would get this:
/// ```xml
/// <g>
/// <rect x="0" y="0" width="100" height="100" style="fill: red;">
/// <image x="10" y="10" width="80" height="80" xlink:href="[data]"></image>
/// </g>
/// ```
/// In other words, the new attribute `padding` is used to shrink the textbox produced,
/// which is then positioned at a location on top of a new `rect` element, created using the `style` attribute,
/// such that the specified padding in its original box is achieved.
/// The effect is to give a background to the textbox.
/// 
/// Note that the advantage of doing it like this is that the background can be expanded to fit if the
/// the textbox has variable dimensions. The colour of text itself is not changed, but this can easily be set in
/// the markup itself.
///
/// ## Interpreting `padding`
/// Padding can only be set as a pixel value, using a plain number to describe it.
/// 
/// Syntactically, it mimics css: that is, it can be specified using one, two, three or four positive values.
/// * One value: apply the same padding to all four sides.
/// * Two values: apply the first to the top and bottom, the second to the right and left.
/// * Three values: apply the first to the top, the second to the left and right, and the third to the bottom.
/// * Four values: apply in clockwise order: top, right, bottom, left.
///
pub fn from_backgrounded_element_to_element_group(src_elem: &Element) -> Result<Element, LayoutError> {
    let mut attrs = HashMap::new();
    for (k, v) in src_elem.attrs() {
        attrs.insert(k.to_string(), v.to_string());
    }

    let markup = get_markup(src_elem, &mut attrs).unwrap();
    let mut input = TextBoxInput::new_from(markup, &mut attrs).unwrap();

    let padding = match attrs.remove("padding") {
        Some(p) => p,
        None => return Err(LayoutError::XMLRequiredAttributeMissing{msg: "padding".to_string()})
    };
    let x: i32 = match attrs.remove("x") {
        Some(i) => i.parse()?,
        None => return Err(LayoutError::XMLRequiredAttributeMissing{msg: "x".to_string()})
    };
    let y: i32 = match attrs.remove("y") {
        Some(i) => i.parse()?,
        None => return Err(LayoutError::XMLRequiredAttributeMissing{msg: "y".to_string()})
    };
    let ps = PaddingSpecification::new(&padding)?;
    let textbox_x = x + ps.left;
    let textbox_y = y + ps.top;

    let get_width = |w| w - (ps.left + ps.right);
    let get_height = |h| h - (ps.top + ps.bottom);


    let new_dimensions = match input.dimensions {
            LayoutDimensions::Static(width, height) => {
                LayoutDimensions::Static(get_width(width), get_height(height))
            },
            LayoutDimensions::StaticWidthFlexHeight(width, heights) => {
                let h: HashSet<i32> = heights.iter().map(|x| get_height(*x)).collect();
                LayoutDimensions::StaticWidthFlexHeight(get_width(width), h)
            },
            LayoutDimensions::FlexWidthStaticHeight(widths, height) => {
                let w: HashSet<i32> = widths.iter().map(|x| get_width(*x)).collect();
                LayoutDimensions::FlexWidthStaticHeight(w, get_height(height))
            },
            LayoutDimensions::Flex(widths, heights) => {
                let w: HashSet<i32> = widths.iter().map(|x| get_width(*x)).collect();
                let h: HashSet<i32> = heights.iter().map(|x| get_height(*x)).collect();
                LayoutDimensions::Flex(w, h)
            }
    };

    input.dimensions = new_dimensions;
    let textbox_output = LayoutBuilder::get_layout_output(&input)?;


    let b64 = base64::encode(&textbox_output.rendered);
    let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);

    let mut output_image = Element::builder("image").build();
    output_image.set_attr("width", textbox_output.width);
    output_image.set_attr("height", textbox_output.height);
    output_image.set_attr("xlink:href", prefixed_b64);
    output_image.set_attr("x", textbox_x);
    output_image.set_attr("y", textbox_y);

    let mut output_rect = Element::builder("rect").build();
    let output_width = textbox_output.width + ps.left + ps.right;
    let output_height = textbox_output.height + ps.top + ps.bottom;
    output_rect.set_attr("width", output_width);
    output_rect.set_attr("height", output_height);
    output_rect.set_attr("x", x);
    output_rect.set_attr("y", y);
    
    let border_width = attrs.remove("border-width");
    let border_top = attrs.remove("border-top");
    let border_bottom = attrs.remove("border-bottom");

    for (k, v) in attrs {
        output_rect.set_attr(k.to_string(), v.to_string());
    }

    let mut output_group = Element::builder("g").build();
    output_group.append_child(output_rect);
    output_group.append_child(output_image);

    if let Some(colour) = border_top {
        let mut top_border = Element::builder("line").build();
        top_border.set_attr("stroke", colour);
        top_border.set_attr("x1", x);
        top_border.set_attr("x2", x + output_width);
        top_border.set_attr("y1", y);
        top_border.set_attr("y2", y);
        if let Some(w) = &border_width {
            top_border.set_attr("stroke-width", w);
        }
        output_group.append_child(top_border);
    };
    if let Some(colour) = border_bottom {
        let mut bottom_border = Element::builder("line").build();
        bottom_border.set_attr("stroke", colour);
        bottom_border.set_attr("x1", x);
        bottom_border.set_attr("x2", x + output_width);
        bottom_border.set_attr("y1", y + output_height);
        bottom_border.set_attr("y2", y + output_height);
        if let Some(w) = &border_width {
            bottom_border.set_attr("stroke-width", w);
        }
        output_group.append_child(bottom_border);
    };
    Ok(output_group)
}



struct PaddingSpecification {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

impl PaddingSpecification {

    fn from_vec(mut v: Vec<i32>) -> Result<PaddingSpecification, LayoutError> {
        
        let ps = match v.len() {
            1 => {
                let all = v.remove(0);
                PaddingSpecification {
                    top: all,
                    right: all,
                    bottom: all,
                    left: all,
                }
            },
            2 => {
                let top_bottom = v.remove(0);
                let left_right = v.remove(0);

                PaddingSpecification {
                    top: top_bottom,
                    right: left_right,
                    bottom: top_bottom,
                    left: left_right,
                }
            },
            3 => {
                let top = v.remove(0);
                let right_left = v.remove(0);
                let bottom = v.remove(0);

                PaddingSpecification {
                    top: top,
                    right: right_left,
                    bottom: bottom,
                    left: right_left,
                }
            },
            4 => {
                PaddingSpecification {
                    top: v.remove(0),
                    right: v.remove(0),
                    bottom: v.remove(0),
                    left: v.remove(0),
                }
            },
            _ => return Err(LayoutError::Generic),
        };
        Ok(ps)
    }

    fn new(src: &str) -> Result<PaddingSpecification, LayoutError> {
        let mut ints: Vec<i32> = Vec::new();
        for s in src.split_whitespace() {
            let parsed = s.parse::<i32>()?;
            ints.push(parsed);
        }
        Ok(Self::from_vec(ints)?)
    }
}

/// Given an xml string potentially containing textbox elements,
/// convert all of these as part of a new svg image, preserving
/// non-textbox elements.
pub fn from_subelements_to_svg_image(xml: &str) -> Result<String, LayoutError> {
    let mut root: Element = xml.parse()?;
    let width = match root.attr("width") {
        Some(x) => x.to_owned(),
        None => return Err(LayoutError::XMLRequiredAttributeMissing{msg: String::from("width")})
    };
    let height = match root.attr("height") {
        Some(x) => x.to_owned(),
        None => return Err(LayoutError::XMLRequiredAttributeMissing{msg: String::from("height")})
    };
    let mut replacement_root = Element::builder("svg")
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("xmlns:xlink", "http://www.w3.org/1999/xlink")
        .attr("width", "100%")
        .attr("viewBox", &format!("0 0 {} {}", width, height))
        .attr("version", "1.1")
        .build();
    for elem in root.children_mut() {
        match elem.name() {
            "textbox" => {
                let replacement_elem = match elem.attr("padding") {
                    Some(_) => from_backgrounded_element_to_element_group(elem)?,
                    None => from_element_to_element(elem)?,
                };
                replacement_root.append_child(replacement_elem);
            },
            _ => {
                replacement_root.append_child(elem.clone());
            }
        }
    }
    replacement_root.write()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subelements() {
        let eg = 
            r#"<?xml version="1.0" encoding="utf-8"?>
            <svg width="2000" height="4000">
                <span>Before</span>
                <textbox x="500" y="0" width="2000" height="2000">
                    <markup>
                        Hello World
                    </markup>
                </textbox>
                <span>Inbetween</span>
                <textbox x="500" y="2500" padding="100" height="1500" width="2000" fill="blue">
                    <markup>
                        I'm back
                    </markup>
                </textbox>
                <span>After</span>
            </svg>
            "#;
        let result = from_subelements_to_svg_image(eg).unwrap();
        let mut parsed_result: Element = result.parse().unwrap();
        assert_eq!(parsed_result.attr("viewBox").unwrap(), "0 0 2000 4000");
        
        let mut kids = parsed_result.children_mut();
        let first_span = kids.next().unwrap();
        let first_textbox = kids.next().unwrap();
        let second_span = kids.next().unwrap();
        let second_textbox_g = kids.next().unwrap();
        let third_span = kids.next().unwrap();

        assert_eq!(first_span.name(), "span");
        assert_eq!(first_textbox.name(), "image");
        assert_eq!(second_span.name(), "span");
        assert_eq!(second_textbox_g.name(), "g");
        assert_eq!(third_span.name(), "span");

        assert_eq!(first_span.texts().next().unwrap(), "Before");
        assert_eq!(second_span.texts().next().unwrap(), "Inbetween");
        assert_eq!(third_span.texts().next().unwrap(), "After");

        assert_eq!(first_textbox.attr("width").unwrap(), "2000");
        assert_eq!(first_textbox.attr("height").unwrap(), "2000");
        assert_eq!(first_textbox.attr("x").unwrap(), "500");
        assert_eq!(first_textbox.attr("y").unwrap(), "0");

        assert_eq!(second_textbox_g.name(), "g");
        let mut g_kids = second_textbox_g.children();
        let second_textbox_rect = &g_kids.next().unwrap();
        let second_textbox = &g_kids.next().unwrap();

        assert_eq!(second_textbox_rect.name(), "rect");
        assert_eq!(second_textbox_rect.attr("width").unwrap(), "2000");
        assert_eq!(second_textbox_rect.attr("height").unwrap(), "1500");
        assert_eq!(second_textbox_rect.attr("x").unwrap(), "500");
        assert_eq!(second_textbox_rect.attr("y").unwrap(), "2500");
        assert_eq!(second_textbox_rect.attr("fill").unwrap(), "blue");

        assert_eq!(second_textbox.name(), "image");
        assert_eq!(second_textbox.attr("width").unwrap(), "1800");
        assert_eq!(second_textbox.attr("height").unwrap(), "1300");
        assert_eq!(second_textbox.attr("x").unwrap(), "600");
        assert_eq!(second_textbox.attr("y").unwrap(), "2600");

    }


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
    fn test_padding_from_vec() {
        let single = PaddingSpecification::from_vec(vec![1]).unwrap();
        let double = PaddingSpecification::from_vec(vec![1, 2]).unwrap();
        let triple = PaddingSpecification::from_vec(vec![1, 2, 3]).unwrap();
        let quadruple = PaddingSpecification::from_vec(vec![1, 2, 3, 4]).unwrap();
        let empty = PaddingSpecification::from_vec(Vec::new());
        let too_long = PaddingSpecification::from_vec(vec![1, 2, 3, 4, 5]);

        assert!(empty.is_err());
        assert!(too_long.is_err());

        assert_eq!(single.top, 1);
        assert_eq!(single.right, 1);
        assert_eq!(single.bottom, 1);
        assert_eq!(single.left, 1);

        assert_eq!(double.top, 1);
        assert_eq!(double.right, 2);
        assert_eq!(double.bottom, 1);
        assert_eq!(double.left, 2);
        
        assert_eq!(triple.top, 1);
        assert_eq!(triple.right, 2);
        assert_eq!(triple.bottom, 3);
        assert_eq!(triple.left, 2);

        assert_eq!(quadruple.top, 1);
        assert_eq!(quadruple.right, 2);
        assert_eq!(quadruple.bottom, 3);
        assert_eq!(quadruple.left, 4);
    }

    #[test]
    fn adding_border() {
        let top_bordered: Element = "<textbox padding=\"0\" x=\"0\" y=\"0\" font-size=\"10\" width=\"100\" height=\"100\" border-top=\"#000000\"><markup>With border</markup></textbox>".parse().unwrap();
        let bottom_bordered: Element = "<textbox padding=\"0\" x=\"0\" y=\"0\" font-size=\"10\" width=\"100\" height=\"100\" border-bottom=\"#000000\"><markup>With border</markup></textbox>".parse().unwrap();
        let with_stroke_width: Element = "<textbox padding=\"0\" x=\"0\" y=\"0\" font-size=\"10\" width=\"100\" height=\"100\" border-bottom=\"#000000\" border-width=\"20\"><markup>With border</markup></textbox>".parse().unwrap();

        let a_ = from_backgrounded_element_to_element_group(&top_bordered).unwrap();
        let a = a_.iter_search("line").unwrap();
        let b_ = from_backgrounded_element_to_element_group(&bottom_bordered).unwrap();
        let b = b_.iter_search("line").unwrap();
        let c_ = from_backgrounded_element_to_element_group(&with_stroke_width).unwrap();
        let c = c_.iter_search("line").unwrap();

        assert_eq!(a.attr("x1").unwrap(), "0");
        assert_eq!(a.attr("x2").unwrap(), "100");
        assert_eq!(a.attr("y1").unwrap(), "0");
        assert_eq!(a.attr("y2").unwrap(), "0");

        assert_eq!(b.attr("x1").unwrap(), "0");
        assert_eq!(b.attr("x2").unwrap(), "100");
        assert_eq!(b.attr("y1").unwrap(), "100");
        assert_eq!(b.attr("y2").unwrap(), "100");

        assert_eq!(c.attr("stroke-width").unwrap(), "20");

    }


    #[test]
    fn with_fill() {
        let elem: Element = "<textbox padding=\"10\" x=\"0\" y=\"0\" font-size=\"10\" width=\"100\" height=\"100\" fill=\"red\"><markup>Hello World</markup></textbox>".parse().unwrap();
        let result = from_backgrounded_element_to_element_group(&elem).unwrap();
        let rect = result.iter_search("rect").unwrap();
        assert_eq!(rect.attr("fill").unwrap(), "red");
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
        assert_eq!(trim_insignificant_whitespace("<span>A<preserved-space/><span style=\"italic\">and</span><preserved-space />B</span>"), "<span>A <span style=\"italic\">and</span> B</span>");

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
