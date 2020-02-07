use serde::de::{self, Visitor, MapAccess, SeqAccess};
use std::fmt;
use crate::layout::{RenderedTextbox, LayoutSource};
use lazy_static::lazy_static;
use pango::{Alignment, FontDescription, SCALE};
use regex::Regex;
use serde::{Deserialize, Serialize, Deserializer};
use std::collections::{BTreeSet, HashMap};
use std::convert::TryFrom;
use std::default::Default;
use std::fmt::Display;
use std::num::NonZeroU16;
use std::num::ParseIntError;
use crate::errors::SvgTextBoxError;
use std::ops::Deref;

pub use crate::pango_wrappers::{AlignmentWrapper, FontDescriptionWrapper};

/// a container to hold different groups of measurement units
#[derive(Debug, Serialize, Clone)]
pub enum UnitContainer {
    AsSet(BTreeSet<NonZeroU16>),
    AsRange{
        min: NonZeroU16,
        max: NonZeroU16,
        step: Option<usize>
    },
}


impl UnitContainer {

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item=u16> + 'a> {
        match self {
            UnitContainer::AsSet(s) => {
                Box::new(s.iter().map(|n| n.get()))
            },
            UnitContainer::AsRange{min, max, step} => {
                let r = min.get()..=max.get();
                match step {
                    Some(u) => Box::new(r.step_by(*u)),
                    None => Box::new(r)
                }
            }
        }
    }
}


impl IntoIterator for UnitContainer {
    type Item = u16;
    type IntoIter = std::collections::btree_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
            .collect::<BTreeSet<u16>>()
            .into_iter()
    }
}



lazy_static! {
    static ref AMPERSAND_REGEX: Regex = Regex::new(r"&(?P<w>\s+)").unwrap();
}
static ACCEL_MARKER: char = '\u{00}';
static NULL_CHAR: char = '\u{0}';
static UNACCEPTABLE_CHARS: [char; 2] = [ACCEL_MARKER, NULL_CHAR];


#[derive(Default, Debug, PartialEq, Serialize, Clone)]
pub struct PaddingSpecification {
    top: u16,
    bottom: u16,
    left: u16,
    right: u16
}

impl PaddingSpecification {

    pub fn left(&self) -> u16 {
        self.left
    }

    pub fn top(&self) -> u16 {
        self.top
    }

    pub fn total_horizontal_padding(&self) -> i32 {
        i32::from(self.left + self.right)
    }

    pub fn total_vertical_padding(&self) -> i32 {
        i32::from(self.top + self.bottom)
    }

    pub fn has_values(&self) -> bool {
        self.top != 0 || self.bottom != 0 || self.left != 0 || self.right != 0
    }
}


/// Pango needs c-style strings (i.e. without null chars)
/// and no unescaped ampersands. It will fail if incompatible strings
/// are passed to various functions. PangoCompatibleString
/// is simply a wrapper around a string to check these requirements.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
#[serde(try_from = "String")]
pub struct PangoCompatibleString(String);

impl PangoCompatibleString {
    pub fn new(s: &str) -> Result<Self, SvgTextBoxError> {
        if s.chars().all(|c| c.is_whitespace()) {
            return Err(SvgTextBoxError::PCSWhitespace);
        }
        if s.chars().any(|c| UNACCEPTABLE_CHARS.contains(&c)) {
            return Err(SvgTextBoxError::BadChar(s.to_string()));
        }
        let mut trimmed = s.trim().to_string();
        // Fix isolated and unambiguous ampersands
        if trimmed.contains('&') && AMPERSAND_REGEX.is_match(&trimmed) {
            trimmed = AMPERSAND_REGEX
                .replace_all(&trimmed, "&amp;$w")
                .into_owned();
        }
        let experimental_parse = pango::parse_markup(&trimmed, ACCEL_MARKER);
        match experimental_parse {
            Ok(_) => Ok(PangoCompatibleString(trimmed)),
            Err(e) => Err(SvgTextBoxError::from(e)),
        }
    }
}

impl Display for PangoCompatibleString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl AsRef<str> for PangoCompatibleString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for PangoCompatibleString {
    type Error = SvgTextBoxError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        PangoCompatibleString::new(&s)
    }
}

fn get_default_font_size() -> UnitContainer {
    let min = NonZeroU16::new(10)
        .unwrap();
    let max = NonZeroU16::new(100)
        .unwrap();
    UnitContainer::AsRange {
        min,
        max,
        step: None
    }
}

/// A textbox (possibly with flexible dimensions) which will have its text expand to fit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBox {
    /// the text of this textbox. Can be in Pango markup language.
    pub markup: PangoCompatibleString,
    /// possible widths
    pub width: UnitContainer,
    /// possible heights
    pub height: UnitContainer,
    /// a wrapper around the font description to use
    #[serde(default, alias="font-desc")]
    pub font_desc: FontDescriptionWrapper,
    /// the alignment of the text
    #[serde(default)]
    pub alignment: AlignmentWrapper,
    /// possible font sizes
    #[serde(default="get_default_font_size", alias="font-size")]
    pub font_size: UnitContainer,
    /// values for padding
    #[serde(default)]
    pub padding: PaddingSpecification,
    /// optional attributes for the background rectangle
    #[serde(flatten)]
    pub padding_attrs: HashMap<String, String>,
}

macro_rules! setter {
    ($func:ident, $input:ty, $target:ident) => {

        pub fn $func<T>(&mut self, value: T) -> &mut Self
            where $input: From<T>
        {
            let converted = <$input>::from(value);
            self.$target = converted;
            self
        }

    };
}


impl TextBox {

    pub fn new(markup: PangoCompatibleString, width: UnitContainer, height: UnitContainer) -> Self {
        TextBox{
            markup,
            width,
            height,
            alignment: AlignmentWrapper::default(),
            font_desc: FontDescriptionWrapper::default(),
            font_size: UnitContainer::AsRange{
                min: NonZeroU16::new(1).unwrap(),
                max: NonZeroU16::new(100).unwrap(),
                step: None
            },
            padding: PaddingSpecification::default(),
            padding_attrs: HashMap::new(),
        }
    }

    setter!(set_font_size, UnitContainer, font_size);
    setter!(set_font_desc, FontDescriptionWrapper, font_desc);
    setter!(set_alignment, AlignmentWrapper, alignment);
    setter!(set_padding, PaddingSpecification, padding);
    setter!(set_padding_attrs, HashMap<String, String>, padding_attrs);

    pub fn to_svg_image(&self) -> Result<RenderedTextbox, SvgTextBoxError> {
        let mut image = RenderedTextbox::new(self)?;
        if self.padding.has_values() && !self.padding_attrs.is_empty() {
            image.insert_background_rect(&self.padding_attrs)?;
        }
        Ok(image)
    }
}

impl LayoutSource for TextBox {

    fn output_width(&self, layout_width: i32) -> f64 {
        let unscaled = layout_width / SCALE;
        f64::from(unscaled + self.padding.total_horizontal_padding())
    }

    fn output_height(&self, layout_height: i32) -> f64 {
        let unscaled = layout_height / SCALE;
        f64::from(unscaled + self.padding.total_vertical_padding())
    }

    fn output_x(&self) -> f64 {
        self.padding.left().into()
    }

    fn output_y(&self) -> f64 {
        self.padding.top().into()
    }

    fn possible_font_sizes<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a> {
        Box::new(self.font_size.iter().map(|n| i32::from(n)*SCALE))
    }

    fn possible_widths<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a> {
        Box::new(self.width.iter()
            .map(|n| i32::from(n) * SCALE)
            .map(move |n| n - (self.padding.total_horizontal_padding() * SCALE)))
    }

    fn possible_heights<'a>(&'a self) -> Box<dyn Iterator<Item=i32> + 'a> {
        Box::new(self.width.iter()
            .map(|n| i32::from(n) * SCALE)
            .map(move |n| n - (self.padding.total_vertical_padding() * SCALE)))
    }

    fn font_description(&self) -> &FontDescription {
        self.font_desc.deref()
    }

    fn markup(&self) -> &str {
        self.markup.as_ref()
    }

    fn alignment(&self) -> Alignment {
        let AlignmentWrapper(a) = self.alignment;
        a
    }
}

impl <'de> Deserialize<'de> for PaddingSpecification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_any(PaddingSpecificationVisitor)
    }
}

impl <'de> Deserialize<'de> for UnitContainer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_any(UnitContainerVisitor)
    }
}

struct UnitContainerVisitor;

impl <'de> Visitor<'de> for UnitContainerVisitor {
    type Value = UnitContainer;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a single integer, a sequence of integers, a string composed of integers seperated by a space, a map of min, max, and step")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let integers = v.split_whitespace()
                 .map(|n| n.parse::<NonZeroU16>())
                 .collect::<Result<BTreeSet<NonZeroU16>, ParseIntError>>()
                 .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?;
        if integers.len() == 0 {
            return Err(de::Error::invalid_length(0, &self));
        }

        Ok(UnitContainer::AsSet(integers))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>
    {
        let mut values = BTreeSet::new();
        while let Some(v) = seq.next_element()? {
            values.insert(v);
        }
        if values.len() == 0 {
            return Err(de::Error::invalid_length(0, &self));
        }
        Ok(UnitContainer::AsSet(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut min: Option<NonZeroU16> = None;
        let mut max: Option<NonZeroU16> = None;
        let mut step: Option<usize> = None;

        while let Some((k, v)) = map.next_entry()? {
            match k {
                "min" => min = Some(v),
                "max" => max = Some(v),
                "step" => {
                    let u = u16::from(v);
                    let u = usize::from(u);
                    step = Some(u);
                },
                _ => {}
            }
        }

        let max = max.unwrap_or(NonZeroU16::new(std::u16::MAX).unwrap());
        let min = min.unwrap_or(NonZeroU16::new(1).unwrap());

        Ok(UnitContainer::AsRange{
            min,
            max,
            step
        })
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let u = u16::try_from(v)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Unsigned(v as u64), &self))?;
        let u = match NonZeroU16::new(u) {
            Some(i) => Ok(i),
            None => Err(de::Error::invalid_value(de::Unexpected::Unsigned(v as u64), &self))
        }?;
        let s = std::iter::once(u)
            .collect::<BTreeSet<_>>();
        Ok(UnitContainer::AsSet(s))
    }
}

struct PaddingSpecificationVisitor;

impl <'de> Visitor<'de> for PaddingSpecificationVisitor {
    type Value = PaddingSpecification;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or single integer or sequence of integers in the format of css padding, or a map of left, right, top, bottom")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let integers = v.split_whitespace()
            .map(|n| n.parse::<u16>())
            .collect::<Result<Vec<u16>, ParseIntError>>()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))?;
        let (top, right, bottom, left) = match integers[..4] {
            [] => (0, 0, 0, 0),
            [i] => (i, i, i, i),
            [top_and_bottom, right_and_left] => (
                top_and_bottom,
                right_and_left,
                top_and_bottom,
                right_and_left,
            ),
            [top, right_and_left, bottom] => (top, right_and_left, bottom, right_and_left),
            [top, right, bottom, left] => (top, right, bottom, left),
            _ => unreachable!(),
        };
        Ok(PaddingSpecification {top, right, bottom, left})
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>
    {
        let mut values: Vec<u16> = Vec::new();
        while let Some(v) = seq.next_element()? {
            values.push(v);
            if values.len() > 4 {
                break;
            }
        }
        let (top, right, bottom, left) = match values[..4] {
            [] => (0, 0, 0, 0),
            [i] => (i, i, i, i),
            [top_and_bottom, right_and_left] => (
                top_and_bottom,
                right_and_left,
                top_and_bottom,
                right_and_left,
            ),
            [top, right_and_left, bottom] => (top, right_and_left, bottom, right_and_left),
            [top, right, bottom, left] => (top, right, bottom, left),
            _ => unreachable!()
        };
        Ok(PaddingSpecification {top, right, bottom, left})
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de>
    {
        let mut top = 0;
        let mut bottom = 0;
        let mut right = 0;
        let mut left = 0;

        while let Some((k, v)) = map.next_entry()? {
            match k {
                "padding-top" | "top" => top = v,
                "padding-bottom" | "bottom" => bottom = v,
                "padding-right" | "right" => right = v,
                "padding-left" | "left" => left = v,
                _ => {}
            }
        }

        Ok(PaddingSpecification{top, bottom, right, left})
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(PaddingSpecification{top: v, bottom: v, right: v, left: v})
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let v = u16::try_from(v)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Unsigned(v as u64), &self))?;
        Ok(PaddingSpecification{top: v, bottom: v, right: v, left: v})
    }
}

#[cfg(test)]
mod textbox_tests {
    use super::*;
    use pango::FontDescription;

    #[test]
    fn paddedtextbox() {
        let src = r##"{            
            "markup": "Hello World",
            "width": 100,
            "height": [100, 200],
            "padding": {
                "top": 10,
                "bottom": 10,
                "left": 0,
                "right": 0
            },
            "fill": "red",
            "stroke": "blue"
        }"##;
        let p: TextBox = serde_json::from_str(src).expect("a");
        p.to_svg_image().expect("b");
    }

    #[test]
    fn unpaddedtextbox() {
        let src = r##"{            
            "markup": "Hello World",
            "width": 100,
            "height": [100, 200]
        }"##;
        let p: TextBox = serde_json::from_str(src).expect("a");
        p.to_svg_image().expect("b");
    }

    #[test]
    fn test_insert_padding_rect() {
        let src = r#"<?xml version="1.0"?>
            <svg width="100" height="100">
                <defs>
                    <def></def>
                </defs>
                <rect x="0" y="0" width="100" height="100"></rect>
            </svg>"#;
        let expected = r#"<?xml version="1.0"?>
            <svg width="100" height="100">
                <defs>
                    <def></def>
                </defs>
                <g>
                    <rect fill="red" height="50" width="50" x="0" y="0"/>
                </g>
                <rect x="0" y="0" width="100" height="100"></rect>
            </svg>"#
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        let attrs: HashMap<String, String> = [("fill", "red")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let mut textbox = RenderedTextbox {
            width: 50.0,
            height: 50.0,
            src: src.to_string()
        };

        textbox.insert_background_rect(&attrs).unwrap();
        let patched = textbox.to_string()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        assert_eq!(patched, expected);
    }

    #[test]
    fn serde() {
        let t = "\"Times New Roman, bold\"";
        let fdw: FontDescriptionWrapper = serde_json::from_str(t).unwrap();
        let fds = serde_json::to_string(&fdw).unwrap();
        let fdw2: FontDescriptionWrapper = serde_json::from_str(&fds).unwrap();
        assert_eq!(fdw, fdw2);
    }

    #[test]
    fn is_good_ref() {
        let fd = "Times New Roman".parse::<FontDescriptionWrapper>().unwrap();
        assert_eq!(
            fd.deref(),
            &FontDescription::from_string("Times New Roman")
        );
    }
}
