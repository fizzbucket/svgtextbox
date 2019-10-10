use crate::layout::{RenderedTextbox, LayoutSource};
use lazy_static::lazy_static;
use pango::{Alignment, FontDescription, SCALE};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::convert::TryFrom;
use std::default::Default;
use std::fmt::Display;
use std::iter::{self, IntoIterator, FromIterator, Extend};
use std::num::NonZeroU16;
use std::num::ParseIntError;
use std::str::FromStr;
use crate::errors::SvgTextBoxError;
use std::ops::Deref;

pub use crate::pango_wrappers::{AlignmentWrapper, FontDescriptionWrapper};

/// a container to hold different groups of measurement units
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnitContainer {
    AsSingle(NonZeroU16),
    AsSet(BTreeSet<NonZeroU16>),
    AsRange{
        min: NonZeroU16,
        max: NonZeroU16,
        step: Option<usize>
    },
    Empty
}

impl Default for UnitContainer {
    fn default() -> Self {
        UnitContainer::AsSingle(NonZeroU16::new(1).unwrap())
    }
}

impl UnitContainer {

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item=u16> + 'a> {
        match self {
            UnitContainer::Empty => {
                Box::new(iter::empty())
            }
            UnitContainer::AsSingle(s) => {
                Box::new(iter::once(s.get()))
            },
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

    /// is this a set which could be expressed as a range?
    fn compress(&mut self) {
        if let UnitContainer::AsSet(set) = self {
            if set.len() > 2 {
                let a = set.iter();
                let b = set.iter().skip(1);
                let mut differences = a.zip(b).map(|(a, b)| b.get() - a.get());
                let first_step = differences.next().unwrap();
                let constant_step = differences.all(|s| s==first_step);
                if constant_step {
                    let min = *set.iter().min().unwrap();
                    let max = *set.iter().max().unwrap();
                    let step = NonZeroU16::new(first_step).unwrap();
                    *self = UnitContainer::from_range_values(min, max, step);
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        if let UnitContainer::Empty = self {
            true
        } else {
            false
        }
    }


    pub fn push(&mut self, n: NonZeroU16) {
        match self {
            UnitContainer::Empty => {
                *self = UnitContainer::AsSingle(n);
            },
            UnitContainer::AsSingle(i) => {
                if *i != n {
                    let v = vec![*i, n].into_iter();
                    *self = UnitContainer::AsSet(v.collect());
                }
            },
            UnitContainer::AsSet(s) => {
                s.insert(n);
            },
            UnitContainer::AsRange{..} => {
                let mut set = self.iter()
                    .map(|n| NonZeroU16::new(n).unwrap())
                    .collect::<BTreeSet<NonZeroU16>>();
                set.insert(n);
                *self = UnitContainer::AsSet(set);
            }
        }
    }

    pub(crate) fn from_range_values(min: NonZeroU16, max: NonZeroU16, step: NonZeroU16) -> Self {
        let step = match step.get() {
            1 => None,
            u => Some(usize::from(u))
        };
        UnitContainer::AsRange{min, max, step}
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

impl Extend<NonZeroU16> for UnitContainer {
    fn extend<T: IntoIterator<Item=NonZeroU16>>(&mut self, iter: T) {
        for elem in iter {
            self.push(elem);
        }
        self.compress();
    }
}


impl FromIterator<NonZeroU16> for UnitContainer {
    fn from_iter<I: IntoIterator<Item=NonZeroU16>>(iter: I) -> Self {
        let set = iter.into_iter().collect::<BTreeSet<NonZeroU16>>();
        let mut container = match set.len() {
            0 => UnitContainer::Empty,
            1 => UnitContainer::AsSingle(*set.iter().next().unwrap()),
            _ => UnitContainer::AsSet(set),
        };
        container.compress();
        container
    }            
}

impl FromStr for UnitContainer {
    type Err = SvgTextBoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let o = s.split_whitespace()
                 .map(|n| n.parse::<NonZeroU16>())
                 .collect::<Result<UnitContainer, ParseIntError>>()?;
        Ok(o)
    }
}

lazy_static! {
    static ref AMPERSAND_REGEX: Regex = Regex::new(r"&(?P<w>\s+)").unwrap();
}
static ACCEL_MARKER: char = '\u{00}';
static NULL_CHAR: char = '\u{0}';
static UNACCEPTABLE_CHARS: [char; 2] = [ACCEL_MARKER, NULL_CHAR];


#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(default)]
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

impl From<(u16, u16, u16, u16)> for PaddingSpecification {
    fn from(a: (u16, u16, u16, u16)) -> Self {
        let (top, right, bottom, left) = a;

        PaddingSpecification {
            top,
            right,
            bottom,
            left
        }
    }
}

impl From<[u16; 4]> for PaddingSpecification {
    fn from(s: [u16; 4]) -> Self {
        PaddingSpecification {top: s[0], right: s[1], bottom: s[2], left: s[3]}
    }
}

impl From<&[u16]> for PaddingSpecification {
    fn from(s: &[u16]) -> PaddingSpecification {
        let v = s.iter().cloned().take(4).collect::<Vec<u16>>();
        // follow css pattern
        let (top, right, bottom, left) = match v[..] {
            [] => (0, 0, 0, 0),
            [s] => (s, s, s, s),
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
        PaddingSpecification {top, right, bottom, left}
    }
}

impl FromIterator<u16> for PaddingSpecification {
    fn from_iter<I: IntoIterator<Item=u16>>(iter: I) -> Self {
        let v = iter.into_iter()
            .take(5)
            .collect::<Vec<u16>>();
        PaddingSpecification::from(&v[..])
    }
}

impl FromStr for PaddingSpecification {
    type Err = SvgTextBoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let o = s.split_whitespace()
            .map(|n| n.parse::<u16>())
            .collect::<Result<PaddingSpecification, ParseIntError>>()?;
        Ok(o)
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

impl TryFrom<&str> for PangoCompatibleString {
    type Error = SvgTextBoxError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        PangoCompatibleString::new(s)
    }
}

impl FromStr for PangoCompatibleString {
    type Err = SvgTextBoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PangoCompatibleString::new(s)
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
    #[serde(default)]
    pub font_desc: FontDescriptionWrapper,
    /// the alignment of the text
    #[serde(default)]
    pub alignment: AlignmentWrapper,
    /// possible font sizes
    #[serde(default)]
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

    pub fn new(markup: PangoCompatibleString, width: UnitContainer, height: UnitContainer) -> Result<Self, SvgTextBoxError> {
        if width.is_empty() {
            return Err(SvgTextBoxError::NoValidWidths);
        }
        if height.is_empty() {
            return Err(SvgTextBoxError::NoValidHeights);
        }

        Ok(TextBox{
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
        })
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
                <rect fill="red" height="50" width="50" x="0" y="0"/>
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
