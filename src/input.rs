use crate::utils::StringExt;
use crate::errors::LayoutError;
use crate::enum_matches::PangoEnumMatch;

use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;
use pango::{Weight, Style, Variant, Stretch, FontDescription};

const DEFAULT_MAX_FONT_SIZE: i32 = 500;
const DEFAULT_MIN_FONT_SIZE: i32 = 0;

/// The standardised input format for creating a textbox.
pub struct TextBoxInput {
	/// the text to use, formatted in [Pango Markup Language](https://developer.gnome.org/pango/stable/PangoMarkupFormat.html) if desired.
	pub markup: String,
	/// Specify the range of widths and heights available;
	pub dimensions: LayoutDimensions,
	/// The base font description.
	pub font_desc: pango::FontDescription,
	/// The alignment of text within the textbox.
	pub alignment: pango::Alignment,
	/// Specify how to size the fonts
	pub fontsizing: FontSizing
}

/// Specify the range of widths and heights available.
/// All should be in pixels.
///
/// The smallest specified distance will be used preferentially;
/// eg if a width of 100px and a height of 100 or 110px is specified,
/// 110 px will only be used as the height if text will not fit at 100px.
pub enum LayoutDimensions {
	/// fixed width and height
    Static(i32, i32),
    /// Static width but any height in the set.
    StaticWidthFlexHeight(i32, HashSet<i32>),
    /// Any width in the set, but a flexible height.
    FlexWidthStaticHeight(HashSet<i32>, i32),
    /// Both a flexible height and width.
    Flex(HashSet<i32>, HashSet<i32>),
}

impl LayoutDimensions {

	pub fn new(mut width: HashSet<i32>, mut height: HashSet<i32>) -> Self {

		let width_is_static =  width.len() == 1;
		let height_is_static = height.len() == 1;

		match (width_is_static, height_is_static) {
			(true, true) => {
				let w = width.drain().next().unwrap();
				let h = height.drain().next().unwrap();
				LayoutDimensions::Static(w, h)
			},
			(true, false) => {
				let w = width.drain().next().unwrap();
				LayoutDimensions::StaticWidthFlexHeight(w, height)
			},
			(false, true) => {
				let h = height.drain().next().unwrap();
				LayoutDimensions::FlexWidthStaticHeight(width, h)
			},
			(false, false) => {
				LayoutDimensions::Flex(width, height)
			}
		}
	}
}

/// Specify the available font sizes in pts.
#[derive(Debug)]
pub enum FontSizing {
    /// Use only one size.
    Static(i32),
    /// Use the largest font size within the vec that will fit the layout.
    Selection(Vec<i32>),
}

impl FontSizing {
	pub fn from_range(min: Option<i32>, max: Option<i32>) -> Result<Self, LayoutError> {
		let range = match (min, max) {
			(Some(min), None) => {
				(min..=DEFAULT_MAX_FONT_SIZE)
			},
			(None, Some(max)) => {
				(DEFAULT_MIN_FONT_SIZE..=max)
			},
			(None, None) => {
				(DEFAULT_MIN_FONT_SIZE..=DEFAULT_MAX_FONT_SIZE)
			},
			(Some(min), Some(max)) => {
				if max < min {
					return Err(LayoutError::BadFontRange);
				} else {
					(min..=max)
				}
			}
		};

		let v = range.collect();
		Ok(FontSizing::Selection(v))
	}
}



pub trait FromHashMap {
	fn new_from(markup: String, src: &mut HashMap<String, String>) -> Result<TextBoxInput, LayoutError>;
	fn alignment_from_str(s: &str) -> Result<pango::Alignment, LayoutError>;
	fn font_desc_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<FontDescription, LayoutError>;
	fn fontsizing_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<FontSizing, LayoutError>;
	fn layoutdimensions_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<LayoutDimensions, LayoutError>;
	fn ints_from_str(source: &str) -> Result<HashSet<i32>, LayoutError>;
}

impl FromHashMap for TextBoxInput {

	fn new_from(markup: String, mut src: &mut HashMap<String, String>) -> Result<TextBoxInput, LayoutError> {
		let alignment = match src.remove("alignment") {
			Some(s) => {
				Self::alignment_from_str(&s)?
			},
			None => pango::Alignment::Center,
		};
		let mut font_desc = Self::font_desc_from_hashmap(&mut src)?;
		let fontsizing = Self::fontsizing_from_hashmap(&mut src)?;
		if let FontSizing::Static(i) = fontsizing {
			font_desc.set_size(i * pango::SCALE);
		}
		let dimensions = Self::layoutdimensions_from_hashmap(&mut src)?;
		let compatible_markup = markup.to_pango_compatible()?;

		let tb = TextBoxInput {
			markup: compatible_markup,
			dimensions,
			font_desc,
			alignment,
			fontsizing
		};

		Ok(tb)

	}

	fn alignment_from_str(s: &str) -> Result<pango::Alignment, LayoutError> {
	    match s {
	        "left" => Ok(pango::Alignment::Left),
	        "centre" | "center" => Ok(pango::Alignment::Center),
	        "right" => Ok(pango::Alignment::Right),
	        _ => return Err(LayoutError::CouldNotTransformStrToPangoEnum{msg: s.to_string()}),
	    }
	}

	fn font_desc_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<FontDescription, LayoutError> {
		let mut font_desc = FontDescription::new();
        if let Some(family) = attrs.remove("font-family") {
        	if family.is_non_whitespace() {
        		font_desc.set_family(&family);
        	} else {
        		return Err(LayoutError::BadFontFamily);
        	}
        }
        if let Some(s) = attrs.remove("font-style") {
            let style = Style::from_str(&s)?;
            font_desc.set_style(style);
        };
        if let Some(w) = attrs.remove("font-weight") {
            let weight = Weight::from_str(&w)?;
            font_desc.set_weight(weight);
        };
        if let Some(v) = attrs.remove("font-variant") {
            let variant = Variant::from_str(&v)?;
            font_desc.set_variant(variant);
        };
        if let Some(s) = attrs.remove("font-stretch") {
            let stretch = Stretch::from_str(&s)?;
            font_desc.set_stretch(stretch);
        };
        Ok(font_desc)
	}

	fn fontsizing_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<FontSizing, LayoutError> {
        let size = attr_remove_to_int!("font-size", attrs);
        let min_size = attr_remove_to_int!("min-size", attrs);
        let max_size = attr_remove_to_int!("max-size", attrs);

        match (size, min_size, max_size) {
         	(Some(i), _, _) => {
                Ok(FontSizing::Static(i))
            },
            (None, min, max) => {
            	FontSizing::from_range(min, max)
            }
        }
	}

	fn ints_from_str(source: &str) -> Result<HashSet<i32>, LayoutError> {
        if source.is_empty() {
            return Err(LayoutError::IntsFromString{msg: source.to_string()});
        }

        // split into component ints
        let mut ints: Vec<i32> = Vec::new();
        for substring in source.split_whitespace() {
            let i = substring.parse::<i32>()?;
            match i {
                i if i > 0 => {
                    ints.push(i);
                },
                _ => return Err(LayoutError::IntsFromString{msg: source.to_string()})
            }
        }

		// "x y" can mean either the range x..=y
        // or only x and y.
        // this is disambiguated by interpreting
        // the situation where y > x as indicating
        // the latter
        if ints.len() == 2 {
        	if ints[1] > ints[0] && !(ints[1] == ints[0]) {
        		ints = (ints[0]..=ints[1]).collect();
        	}
        }

        let output_set = HashSet::from_iter(ints);
        Ok(output_set)

	}

	fn layoutdimensions_from_hashmap(attrs: &mut HashMap<String, String>) -> Result<LayoutDimensions, LayoutError> {
		let width = attrs.remove("width").ok_or(
			LayoutError::BadDistanceValues{
				msg: "Width missing".to_string()
			}
		)?;
		let height = attrs.remove("height").ok_or(
			LayoutError::BadDistanceValues{
				msg: "Height missing".to_string()
			}
		)?;
        let width_set = Self::ints_from_str(&width);
        let height_set = Self::ints_from_str(&height);

        match (width_set, height_set) {
        	(Ok(w), Ok(h)) => Ok(LayoutDimensions::new(w, h)),
        	(Err(_), Ok(_)) => Err(LayoutError::BadDistanceValues{msg: format!("Attempted to use an invalid width value ({})", &width)}),
        	(Ok(_), Err(_)) => Err(LayoutError::BadDistanceValues{msg: format!("Attempted to use an invalid height value ({})", &height)}),
        	(Err(_), Err(_)) => Err(LayoutError::BadDistanceValues{msg: format!("Attempted to use invalid width and height values ({}, {})", &width, &height)})

        }
    }
}

#[cfg(test)]
mod tests {
	use super::*;

	fn font_desc_sample_required() -> (String, HashMap<String, String>) {
		let markup = "Hello World".to_string();
		let mut hashmap = HashMap::new();
		hashmap.insert("width".to_string(), "50".to_string());
		hashmap.insert("height".to_string(), "50".to_string());
		(markup, hashmap)
	}

	#[test]
	fn font_specified_size_added_to_description() {
		let (markup, mut hashmap) = font_desc_sample_required();
		hashmap.insert("font-size".to_string(), "20".to_string());
		let tbi = TextBoxInput::new_from(markup, &mut hashmap).unwrap();
		assert_eq!(tbi.font_desc.get_size(), 20 * pango::SCALE);
		match tbi.fontsizing {
			FontSizing::Static(i) => {
				assert_eq!(i, 20);
			},
			_ => panic!()
		}
	}

	#[test]
	fn get_font_desc() {
		let (_markup, mut hashmap) = font_desc_sample_required();
		hashmap.insert("font-family".to_string(), "Times New Roman".to_string());
		hashmap.insert("font-weight".to_string(), "700".to_string());
		hashmap.insert("font-style".to_string(), "italic".to_string());
		let fd = TextBoxInput::font_desc_from_hashmap(&mut hashmap).unwrap();
		assert_eq!(fd.get_family().unwrap(), "Times New Roman");
        assert_eq!(fd.get_weight(), Weight::Bold);
        assert_eq!(fd.get_style(), Style::Italic);
	}

	#[test]
	fn font_family_bad_strings() {
		let (_markup, mut hashmap) = font_desc_sample_required();
        hashmap.insert("font-family".to_string(), "\u{0}".to_string());
        let fd = TextBoxInput::font_desc_from_hashmap(&mut hashmap);
        assert!(fd.is_err());
        hashmap.insert("font-family".to_string(), "\u{00}".to_string());
        let fd = TextBoxInput::font_desc_from_hashmap(&mut hashmap);
        assert!(fd.is_err());
	}


    #[test]
    fn test_get_alignment() {
        let left = TextBoxInput::alignment_from_str("left").unwrap();
        let centre = TextBoxInput::alignment_from_str("centre").unwrap();
        let center = TextBoxInput::alignment_from_str("center").unwrap();
        let right = TextBoxInput::alignment_from_str("right").unwrap();
        let bad = TextBoxInput::alignment_from_str("bad");
        assert!(bad.is_err());
        assert_eq!(left, pango::Alignment::Left);
        assert_eq!(right, pango::Alignment::Right);
        assert_eq!(centre, pango::Alignment::Center);
        assert_eq!(center, pango::Alignment::Center);
    }

    #[test]
    fn test_get_layout_dimensions() {
    	let mut hashmap: HashMap<String, String> = HashMap::new();
    	let no_set = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap);
    	assert!(no_set.is_err());
    	hashmap.insert("width".to_string(), "100".to_string());
    	let one_set = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap);
    	assert!(one_set.is_err());
    	hashmap.insert("width".to_string(), "100".to_string());
    	hashmap.insert("height".to_string(), "100".to_string());
    	let two_set = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap).expect("Two set");
    	match two_set {
    		LayoutDimensions::Static(100, 100) => {},
    		_ => panic!()
    	}
    	hashmap.insert("width".to_string(), "100".to_string());
    	hashmap.insert("height".to_string(), "200 100".to_string());
    	let flex_set = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap).expect("flex set");
    	match flex_set {
    		LayoutDimensions::StaticWidthFlexHeight(100, v) => {
    			let h = HashSet::from_iter(vec![200, 100].into_iter());
    			assert_eq!(v, h);
    		},
    		_ => panic!()
    	}
    	hashmap.insert("width".to_string(), "100".to_string());
    	hashmap.insert("height".to_string(), "100 200".to_string());
    	let flex_range = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap).expect("flex set");
    	match flex_range {
    		LayoutDimensions::StaticWidthFlexHeight(100, v) => {
    			let r: Vec<i32> = (100..=200).collect();
    			let h = HashSet::from_iter(r.into_iter());
    			assert_eq!(v, h);
    		},
    		_ => panic!()
    	}


    	hashmap.insert("width".to_string(), "100".to_string());
    	hashmap.insert("height".to_string(), "bad".to_string());
    	let bad_input = TextBoxInput::layoutdimensions_from_hashmap(&mut hashmap);
    	assert!(bad_input.is_err());
    }


    #[test]
    fn test_get_intsfromstr() {

    	let single_parsed = TextBoxInput::ints_from_str("10").unwrap();
    	let range_parsed = TextBoxInput::ints_from_str("10 20").unwrap();
    	let inverted_range_parsed = TextBoxInput::ints_from_str("20 10").unwrap();
    	
    	let zero_parsed = TextBoxInput::ints_from_str("0");
    	assert!(zero_parsed.is_err());
    	let non_int_parsed = TextBoxInput::ints_from_str("Hello");
    	assert!(non_int_parsed.is_err());
    	let includes_not_int_parsed = TextBoxInput::ints_from_str("20 10 Hello");
    	assert!(includes_not_int_parsed.is_err());
    	let includes_zero_parsed = TextBoxInput::ints_from_str("20 10 0");
    	assert!(includes_zero_parsed.is_err());

    	assert!((single_parsed.len() == 1) & single_parsed.contains(&10));
    	for i in 10..=20 {
    		assert!(range_parsed.contains(&i));
    	}
    	assert!((inverted_range_parsed.len() == 2) && inverted_range_parsed.contains(&20) && inverted_range_parsed.contains(&20));

   	}

    #[test]
    fn test_get_fontsizing() {

        let mut hashmap: HashMap<String, String> = HashMap::new();


        macro_rules! match_or_panic {
            ($fontsizing:ident, $p:pat) => (
                match $fontsizing {
                    $p => {},
                    _ => panic!(),
                }
            )
        }



    	let none_set = TextBoxInput::fontsizing_from_hashmap(&mut hashmap).unwrap();
        match_or_panic!(none_set, FontSizing::Selection(_));

    	hashmap.insert("font-size".to_string(), "10".to_string());
        let static_set = TextBoxInput::fontsizing_from_hashmap(&mut hashmap).unwrap();
        match_or_panic!(static_set, FontSizing::Static(10));

    	hashmap.insert("font-size".to_string(), "10".to_string());
    	hashmap.insert("min-size".to_string(), "8".to_string());
        let fs = TextBoxInput::fontsizing_from_hashmap(&mut hashmap).unwrap();
        match_or_panic!(fs, FontSizing::Static(10));

    	hashmap.insert("min-size".to_string(), "8".to_string());
    	let x = TextBoxInput::fontsizing_from_hashmap(&mut hashmap).unwrap();
    	match_or_panic!(x, FontSizing::Selection(_));

    	hashmap.insert("max-size".to_string(), "8".to_string());
    	let x = TextBoxInput::fontsizing_from_hashmap(&mut hashmap).unwrap();
    	match_or_panic!(x, FontSizing::Selection(_));

    }

    #[test]
    fn test_new_from() {
		let markup = "Hello & World".to_string();
		let mut hashmap = HashMap::new();
		hashmap.insert("alignment".to_string(), "left".to_string());
		hashmap.insert("width".to_string(), "100".to_string());
		hashmap.insert("height".to_string(), "100".to_string());
		hashmap.insert("font-size".to_string(), "12".to_string());
		let input = TextBoxInput::new_from(markup, &mut hashmap).unwrap();
		assert_eq!(input.markup, "Hello &amp; World");
		match input.fontsizing {
			FontSizing::Static(12) => {},
			_ => panic!(),
		}
		match input.dimensions {
			LayoutDimensions::Static(100, 100) => {},
			_ => panic!(),
		}
		assert_eq!(input.font_desc.get_size(), 12 * pango::SCALE);
    }

    #[test]
    fn test_fontsizing_from_range() {
		let ten = Some(10);
		let twenty = Some(20);

		let specified_range = FontSizing::from_range(ten, twenty).unwrap();
		let min_specified = FontSizing::from_range(ten, None).unwrap();
		let max_specified = FontSizing::from_range(None, twenty).unwrap();
		let none_specified = FontSizing::from_range(None, None).unwrap();
		let wrong_order = FontSizing::from_range(twenty, ten);
		assert!(wrong_order.is_err());

		let min_default_max: Vec<i32> = (10..=DEFAULT_MAX_FONT_SIZE).collect();
		let min_and_max: Vec<i32> = (10..=20).collect();
		let default_min_max: Vec<i32> = (DEFAULT_MIN_FONT_SIZE..=20).collect();
		let default_default: Vec<i32> = (DEFAULT_MIN_FONT_SIZE..=DEFAULT_MAX_FONT_SIZE).collect();

		if let FontSizing::Selection(v) = specified_range {
			assert_eq!(v, min_and_max);
		} else {
			panic!()
		};

		if let FontSizing::Selection(v) = min_specified {
			assert_eq!(v, min_default_max);
		} else {
			panic!()
		};

		if let FontSizing::Selection(v) = max_specified {
			assert_eq!(v, default_min_max);
		} else {
			panic!()
		};

		if let FontSizing::Selection(v) = none_specified {
			assert_eq!(v, default_default);
		} else {
			panic!()
		};
    }

    #[test]
    fn test_new_layoutdimensions() {
    	let single = HashSet::from_iter(vec![10].into_iter());
    	let double = HashSet::from_iter(vec![10, 20].into_iter());
    
    	let two_single = LayoutDimensions::new(single.clone(), single.clone());
    	let width_single = LayoutDimensions::new(single.clone(), double.clone());
    	let height_single = LayoutDimensions::new(double.clone(), single.clone());
    	let two_double = LayoutDimensions::new(double.clone(), double.clone());

    	if let LayoutDimensions::Static(width, height) = two_single {
    		assert_eq!(width, 10);
    		assert_eq!(height, 10);
    	} else {
    		panic!();
    	};

    	if let LayoutDimensions::StaticWidthFlexHeight(width, height) = width_single {
    		assert_eq!(width, 10);
    		assert!((height.len() == 2) & height.contains(&10) & height.contains(&20));
    	} else {
    		panic!();
    	};

    	if let LayoutDimensions::FlexWidthStaticHeight(width, height) = height_single {
    		assert!((width.len() == 2) & width.contains(&10) & width.contains(&20));
    		assert_eq!(height, 10);
    	} else {
    		panic!();
    	};

    	if let LayoutDimensions::Flex(width, height) = two_double {
    		assert!((width.len() == 2) & width.contains(&10) & width.contains(&20));
    		assert!((height.len() == 2) & height.contains(&10) & height.contains(&20));
    	} else {
    		panic!();
    	}
    }

}


