mod xml_support;
mod layout;
mod fontsizing;
mod pango_interactions;
mod utils;
mod textbox;
mod padded_textbox;

pub use textbox::{TextBox, Length};
pub use padded_textbox::{PaddedTextBox, PaddingSpecification};
pub use pango_interactions::{PangoCompatibleString, FontDescriptionWrapper};
pub use fontsizing::FontSizing;
pub use xml_support::transform_xml;




// struct PaddingSpecification {
//     left: u16,
//     right: u16,
//     top: u16,
//     bottom: u16
// }

// struct HexColour(String);







// macro_rules! errinator {
//     ($name:ident, $msg:expr) => {
//         impl std::fmt::Display for $name {
//             fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 write!(f, "{:?}", self)
//             }
//         }

//         impl Error for $name {
//             fn source(&self) -> Option<&(dyn Error + 'static)> {
//                 None
//             }
//         }
//     };
// }


// pub fn alignment_default() -> pango::Alignment {
//     pango::Alignment::Center
// }


// #[derive(Debug, Clone)]
// enum LayoutError {
//     SizingError,
//     BadConversion,
// }





// errinator!(LayoutError, "Error rendering a layout from textbox");


















// #[derive(Debug, Deserialize)]
// pub struct PaddedTextBox {
//     textbox: TextBox,
//     padding: PaddingSpecification,
//     x: u32,
//     y: u32,
//     border_width: Option<u16>,
//     border_top: Option<HexColour>,
//     border_bottom: Option<HexColour>,
//     #[serde(flatten)]
//     other: HashMap<String, Value>
// }





// fn to_pango_scale<T>(n: T) -> i32
// where
//     i32: std::convert::From<T>,
// {
//     i32::from(n) * pango::SCALE
// }



// #[cfg(test)]
// mod tests {

//     use super::*;

//     #[test]
//     fn test_from_strs() {
//     	let fontsize_cases = [
//     		("100", FontSizing::Static(100)),
//     		("100 103", FontSizing::Flex(vec![100, 101, 102, 103].into_iter().collect())),
//     		("200 100", FontSizing::Flex(vec![200, 100].into_iter().collect())),
//     		("100 200 300", FontSizing::Flex(vec![100, 200, 300].into_iter().collect()))
//     	];

//     	for (i, e) in fontsize_cases.iter() {
//     		assert_eq!(i.parse::<FontSizing>().unwrap(), *e);
//     	}

//     	let length_cases = [
//     		("100", Length::Static(100)),
//     		("100 200", Length::Flex(vec![200, 100].into_iter().collect())),
//     		("100 200 300", Length::Flex(vec![100, 200, 300].into_iter().collect()))
//     	];

//     	for (i, e) in length_cases.iter() {
//     		assert_eq!(i.parse::<Length>().unwrap(), *e);
//     	}
//     }

//     #[test]
//     fn v16() {
//     	let testcases = [
//     		("100", vec![100]),
//     		("100 200", vec![100, 200]),
//     		("100 200 300", vec![100, 200, 300])
//     	];

//     	let bad = [
//     		"100 two hundred",
//     		"65536",
//     	];

//     	for (input, expected_output) in testcases.iter() {
//     		assert_eq!(vec_u16_from_str(input).unwrap(), *expected_output);
//     	}

//     	for (input) in bad.iter() {
//     		assert!(vec_u16_from_str(input).is_err());
//     	}

//     }

//     #[test]
//     fn deser() {
//         let minimal = r#"{
//     		markup: Hello World,
//     		width: 100,
//     		height: [100, 200, 300],
//     		font_size: {min: 10, max: 12},
//     		font_desc: "Sans 10"
//     	}"#;

//         let m: TextBox = serde_yaml::from_str(minimal).unwrap();
//         assert_eq!(m.markup, PangoCompatibleString("Hello World".to_string()));
//         let l = vec![(100, 100), (100, 200), (100, 300)]
//             .iter()
//             .map(|(w, h)| (*w, *h))
//             .collect::<Vec<(i32, i32)>>();
//         assert_eq!(m.possible_layout_dimensions(), l);
//         assert_eq!(m.font_size.to_vec(), vec![10, 11, 12]);
//         assert_eq!(
//             m.font_desc.as_ref().get_family().map(|g| g.to_string()),
//             Some("Sans".to_string())
//         );
//     }

//     #[test]
//     fn possible_dimensions() {
//         let mut testcases = std::collections::HashMap::new();

//         testcases.insert(
//             ("100", "[100, 200, 300]"),
//             vec![(100, 100), (100, 200), (100, 300)],
//         );

//         testcases.insert(
//             ("[100, 200, 300]", "100"),
//             vec![(100, 100), (200, 100), (300, 100)],
//         );

//         testcases.insert(
//             ("[100, 200]", "[100, 200]"),
//             vec![(100, 100), (100, 200), (200, 100), (200, 200)],
//         );

//         testcases.insert(
//             ("[100, 200]", "[100, 200, 300]"),
//             vec![
//                 (100, 100),
//                 (100, 200),
//                 (100, 300),
//                 (200, 100),
//                 (200, 200),
//                 (200, 300),
//             ],
//         );

//         testcases.insert(
//             ("[100, 200, 300]", "[100, 200]"),
//             vec![
//                 (100, 100),
//                 (100, 200),
//                 (200, 100),
//                 (200, 200),
//                 (300, 100),
//                 (300, 200),
//             ],
//         );

//         for ((width, height), target) in testcases.iter() {
//             let input = format!(
//                 "{{markup: Hello World,
// 		    		width: {},
// 		    		height: {},
// 		    		font_size: 10
// 		    	}}",
//                 width, height
//             );
//             let instance: TextBox = serde_yaml::from_str(&input).unwrap();
//             let cmp = target
//                 .iter()
//                 .map(|(w, h)| (*w, *h))
//                 .collect::<Vec<(i32, i32)>>();
//             assert_eq!(instance.possible_layout_dimensions(), cmp);
//         }
//     }

//     #[test]
//     fn test_fontsizing() {
//         let a = ("10", vec![10]);
//         let b = ("[10, 20, 30]", vec![10, 20, 30]);
//         let c = ("{min: 10, max: 20}", (10..=20).collect::<Vec<u16>>());
//         let d = ("{min: 10, max: 20, step: 2}", vec![10, 12, 14, 16, 18, 20]);

//         for (s, expected) in [a, b, c, d].iter() {
//             let f: FontSizing = serde_yaml::from_str(s).unwrap();
//             assert_eq!(&f.to_vec(), expected);
//         }
//     }
// }
