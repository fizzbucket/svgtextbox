//! # svgtextbox
//!
//! `svgtextbox` creates svg images
//! of a particular size containing formatted text.
//!
//! Its most useful feature is the ability to
//! size text automatically to fill up as much space
//! as possible without exceeding the image size, but it
//! also leverages pango's rust bindings to allow for
//! complex textual formatting.
//!
//! It was originally written to aid in the automatic creation of
//! book covers, but might be useful for anyone trying to automatically
//! do complicated text layout in images.
//!
//! It adds very few capabilities to what could be done with pango anyway,
//! but hopefully is substantially easier to use.
//!
//! # Examples
//!
//! ```
//! # use svgtextbox::{LayoutDimensions, FontSizing, TextBoxInput, LayoutBuilder};
//! # use std::collections::HashSet;
//!
//! // Expand a textbox to fit text
//!
//! // Have a width of 100 px, but a height of either 25 or 50px;
//! let possible_heights: HashSet<i32> = vec![25, 50].into_iter().collect();
//! let dimensions = LayoutDimensions::StaticWidthFlexHeight(100, possible_heights);
//! // Use a fixed font size of 24pts.
//! let fontsizing = FontSizing::Static(24);
//!
//! let input = TextBoxInput {
//!		markup: "Hello World".to_string(),
//!		dimensions,
//!		alignment: pango::Alignment::Left,
//!		fontsizing,
//!		font_desc: pango::FontDescription::new()
//! };
//!
//! let rendered_layout = LayoutBuilder::get_layout_output(&input).unwrap();
//!
//! // The textbox as a rendered svg. 
//! let layout_bytes = rendered_layout.rendered;
//! // The height of the layout as it was rendered.
//! let height = rendered_layout.height;
//! // Because the text would not fit at 25px, the textbox used a height of 50px instead.
//! assert_eq!(height, 50);
//!
//! // Make text size larger to fit a textbox of 200 x 200px:
//!
//! let dimensions = LayoutDimensions::Static(200, 200);
//! // The text will be set in the largest of these sizes that fits:
//! let fontsizing = FontSizing::Selection(vec![10, 20, 30, 40, 50, 60]);
//! let input = TextBoxInput {
//!		markup: "Hello World".to_string(),
//!		dimensions,
//!		alignment: pango::Alignment::Left,
//!		fontsizing,
//!		font_desc: pango::FontDescription::new()
//! };
//! let rendered_layout = LayoutBuilder::get_layout_output(&input).unwrap();
//!```
//! Another method to specify a textbox is as an xml element.
//! This should have the following form:
//! ```xml
//! <textbox width="100" height="100">
//!     <markup>
//!         [pango-like markup]
//!     </markup>
//! </textbox>
//! ```
//! The output will be, in string form:
//! ```xml
//! <image width="100" height="100" xlink:href="[textbox as base64]"></image>
//! ```
//!
//! Further control is enabled by adding optional attributes to the `textbox` element:
//!
//! * `alignment`: the alignment to use: (one of left, centre, right)
//! * `font-family`: the name of the font family to use.
//! * `font-weight`: the weight of the font to use.
//! * `font-style`: the font style to use.
//! * `font-variant` 
//! * `font-stretch`
//! * `font-size`
//! * `font-sizes`
//! * `min-font-size`
//! * `max-font-size`
//! * `preserve-whitespace`
//! 
//! Any other attributes will be passed through to the output element.
//!
//! # XML usage example
//! ```
//! # use svgtextbox::from_element_to_element;
//! extern crate minidom;
//! use minidom::Element;
//!
//! // Because xml is likely to have meaningless indentation, all whitespace is normalised.
//! // We include a <br/> element to indicate a newline that should be preserved.
//! // If we were to include the attribute "preserve-whitespace"="true", whitespace would be preserved,
//! // but the <br/> element would cause an error.
//! let example_xml = r#"
//! 	<textbox x="0" width="300" height="300" font-family="Open Sans">
//! 		<markup>
//! 			<span font-weight="bold" font-size="larger"> Title </span>
//! 			<br/>
//! 			A paragraph of following text at a normal weight and size.
//! 		</markup>
//!		</textbox>
//! 	"#;
//! let elem_in: Element = example_xml.parse().unwrap();
//! let elem_out = from_element_to_element(&elem_in).unwrap();
//! assert_eq!(elem_out.attr("x").unwrap(), "0");
//! assert!(elem_out.attr("xlink:href").is_some());
//! ```

#[macro_use]
mod utils;
#[macro_use]
mod enum_matches;
mod layout;

mod errors;
mod output;
mod input;
mod textbox;
mod xml_support;

pub use xml_support::from_element_to_element;
pub use input::{TextBoxInput, LayoutDimensions, FontSizing};
pub use textbox::LayoutBuilder;
pub use output::LayoutOutput;
