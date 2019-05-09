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
//! # use svgtextbox::SVGTextBox;
//! extern crate pango;
//! // Markup should be a string in the Pango Text Attribute Markup Language.
//! // (See <https://developer.gnome.org/pango/stable/PangoMarkupFormat.html> for more).
//! // This particular example produces two lines, "Hello" and "World,"
//! // where "Hello" is larger and "World" is set in italic and coloured red.
//! let markup = "<span size=\"larger\">Hello</span>\n\
//!				  <span style=\"italic\" foreground=\"red\">World</span>";
//! let width = 500;
//! let height = 500;
//! // indicate an image where the text is resized to be as large as possible.
//! let tb = SVGTextBox::new(markup, width, height);
//!
//! // Indicate an image where the text stays at its original size
//! // and is all set in 10pt sans.
//! let ten_pt_sans = pango::FontDescription::from_string("Sans 10");
//! let static_tb = SVGTextBox::new("Hello World", width, height)
//!						.set_font_desc(ten_pt_sans)
//!						.set_static();
//! ```
//!
//! A SVGTextbox alone isn't very helpful! The trait [SVGTextBoxOut](::SVGTextboxOut), however, provides a number of the most common
//! transformations. (At some future point this is likely to include either native creation of png files or support for
//! conversion into pngs, but this isn't there yet. `librsvg` or `resvg` are both great conversion tools, though.)
//!
//! ```
//! # use std::str;
//! # use svgtextbox::SVGTextBox;
//! # use svgtextbox::SVGTextboxOut;
//! let tb = SVGTextBox::new("Hello World", 100, 100);
//! let as_bytes = tb.as_bytes().unwrap();
//! let as_base64_with_data_prefix = tb.as_embeddable_base64().unwrap();
//! let as_svg_string = tb.as_string().unwrap();
//! // or write to a file
//! tb.to_file("example.svg").unwrap();
//! # std::fs::remove_file("example.svg").unwrap();
//! ```
//!
//! It is possible to format the text layout in various ways beyond using markup.
//! The typeface of the text, together with options like style and weight,
//! can be set by using a `pango::FontDescription`.
//! See the documentation for this for a full description; briefly, this can be set
//! using either a string or by creating a FontDescription and adding attributes.
//! The created font description can be used as shown below. (Note that the size of
//! a font description is only meaningful if `set_static()` is called; otherwise the
//! text size will be changed to fit the image.)
//!
//! ```
//! extern crate pango;
//! # use svgtextbox::SVGTextBox;
//! # use svgtextbox::get_font_description;
//! // The simplest way to create a font description.
//! let font_eg = "Minion Pro bold italic 10";
//!
//! let eg = SVGTextBox::new("This text will be set in bold italic Minion Pro at size 10", 500, 100)
//!						.set_font_desc_from_str(font_eg).unwrap()
//!						.set_static();
//!
//! // More complicated font descriptions are possible by setting a FontDescription directly.
//! // A font description can be created using pango directly, but a convenience function is also available
//! // to create a FontDescription from optional strs; `eg2` below is the same as `eg`.
//!
//! let family = Some("Minion Pro");
//! let style = Some("italic");
//! let weight = Some("bold");
//! let mut fd = get_font_description(family, weight, style);
//! fd.set_size(10 * pango::SCALE);
//!
//! let eg2 = SVGTextBox::new("This text will be set in bold italic Minion Pro at size 10", 500, 100)
//!						.set_font_desc(fd)
//!						.set_static();
//!
//! ```
//! Another important option is how to align the text: left, centred, or right.
//!
//! ```
//! # use svgtextbox::SVGTextBox;
//! extern crate pango;
//!
//! let left_aligned = SVGTextBox::new("Left", 100, 100)
//!							.set_alignment_from_str("left");
//! let also_left = SVGTextBox::new("Left", 100, 100)
//!							.set_alignment(pango::Alignment::Left);
//! let centre_aligned = SVGTextBox::new("Centre", 100, 100)
//!							.set_alignment_from_str("centre");
//! let right_aligned = SVGTextBox::new("Right", 100, 100)
//!							.set_alignment_from_str("right");
//! ```
//! # Things to note
//!
//! * Specifying a font description that doesn't exist is not a fatal error. The closest match will be used instead. This could mean things don't look quite as you expect.
//! * Text will not be set to a base size of more than 500 pts.
//! 	There's no particular reason for this number, but some limit was required, and that's high enough to not be a problem generally.
//!		(Similarly, you can't specify a width or height greater than std::i32::MAX / pango::SCALE, but if I found that to be a problem I would
//!		rethink my strategies.)
extern crate pango;
use pango::FontMapExt;
use pango::LayoutExt;
use pango::Weight;
extern crate cairo;
use cairo::prelude::*;
extern crate pangocairo;
extern crate regex;
use regex::Regex;
use std::str;
extern crate glib;
use glib::translate::*;
use pango_sys;
use std::mem;
extern crate custom_error;
use custom_error::custom_error;

custom_error! {
    /// The various errors we can encounter. These should be the only
    /// errors returned.
    /// * `MarkupTooLong`: Returned when markup is too long.
    /// * `MarkupNullChar`: Returned if markup contains '\u{00}' or '\u{0}'.
    /// * `MarkupWhitespace`: Returned if markup is empty or contains only whitespace.
    /// * `BadMarkup`: Returned if pango raises a warning when parsing markup.
    /// * `Distance`: Returned if a distance (width or height) of the layout is inappropriate.
    /// * `WidthNotSet`: Returned when trying to convert a layout without a set width.
    /// * `HeightNotSet`: Returned when trying to convert a layout without a set height.
    /// * `MinFontSize`: Returned when markup would not fit within a layout even at the minimum font size.
    /// * `StaticFontNoFit`: Returned when markup would not fit within a static layout at its set font size (or the default font size if no size was set).
    /// * `FontDescriptionStr`: Returned when attempting to create a font description from an invalid string.
    /// * `Utf8Error`: Wrapper for `std::str::Utf8Error`.
    /// * `Io`: Wrapper for `std::io::Error`.
    #[derive(PartialEq)]
    pub LayoutError
    MarkupTooLong = "Markup is too long",
    MarkupNullChar = "Markup contains a null char",
    MarkupWhitespace = "Markup is only whitespace or empty",
    BadMarkup{msg: String} = "Markup is improperly formatted: {msg}",
    Distance = "Layout width and height must be greater than zero and less than 2097151",
    WidthNotSet = "Layouts must have a width set",
    HeightNotSet = "Layouts must have a height set",
    MinFontSize = "Could not fit text at minimum font size",
    StaticFontNoFit = "Could not fit text at static font size",
    FontDescriptionStr = "The font description string provided could not be parsed",
    Utf8Error{msg: String} = "Utf 8 Error: {msg}",
    Io{msg: String} = "Io error: {msg}",
}

impl std::convert::From<std::str::Utf8Error> for LayoutError {
    fn from(error: std::str::Utf8Error) -> Self {
        LayoutError::Utf8Error {
            msg: error.to_string(),
        }
    }
}

impl std::convert::From<std::io::Error> for LayoutError {
    fn from(error: std::io::Error) -> Self {
        LayoutError::Io {
            msg: error.to_string(),
        }
    }
}

/// A little wrapper to construct pango::FontDescriptions from optional strs
/// rather than having to import pango directly in clients.
///```
///	# use svgtextbox::get_font_description;
///	let fd = get_font_description(Some("Open Sans"), Some("700"), None);
///	let fd2 = get_font_description(Some("Open Sans"), Some("bold"), None);
///	assert_eq!(fd, fd2);
///```
/// # Arguments
///
/// * `family`: the font family to use
/// * `weight`: the font weight to use. This can be specified either
///            as a numeric string, or as a common name.
///			   For example, a bold font could be specified as either
///			   "700" or "bold". Unrecognised names or weights
///				are treated as a normal weight (400).
///				Note that available weights are more numerous
///				than css weights, but that many fonts will not support all possible weights.
/// * `style`: the style of the font: normal, italic or oblique.
///				Any string that is not "italic" or "oblique" will be treated as specifying regular style.
///
pub fn get_font_description(
    family: Option<&str>,
    weight: Option<&str>,
    style: Option<&str>,
) -> pango::FontDescription {
    FontDescriptionStrings {
        family,
        weight,
        style,
    }
    .to_font_desc()
}

struct FontDescriptionStrings<'a> {
    family: Option<&'a str>,
    weight: Option<&'a str>,
    style: Option<&'a str>,
}

impl<'a> FontDescriptionStrings<'a> {
    fn to_font_desc(&self) -> pango::FontDescription {
        let mut font_desc = pango::FontDescription::new();
        if let Some(ff) = self.family {
            font_desc.set_family(ff);
        };
        if let Some(w) = self.weight {
            let weight = FontDescriptionStrings::str_to_weight(w);
            font_desc.set_weight(weight);
        };
        if let Some(s) = self.style {
            let style = FontDescriptionStrings::str_to_style(s);
            font_desc.set_style(style);
        };
        font_desc
    }

    pub fn str_to_weight(s: &str) -> pango::Weight {
        match s {
            "100" | "thin" | "hairline" => Weight::Thin,
            "200" | "ultralight" | "extralight" => Weight::Ultralight,
            "300" | "light" => Weight::Light,
            "350" | "semilight" => Weight::Semilight,
            "380" | "book" => Weight::Book,
            "400" | "normal" => Weight::Normal,
            "500" | "medium" => Weight::Medium,
            "600" | "semibold" | "demibold" => Weight::Semibold,
            "700" | "bold" => Weight::Bold,
            "800" | "ultrabold" | "extrabold" => Weight::Ultrabold,
            "900" | "black" | "heavy" => Weight::Heavy,
            "1000" | "ultraheavy" => Weight::Ultraheavy,
            _ => Weight::Normal,
        }
    }

    pub fn str_to_style(s: &str) -> pango::Style {
        match s {
            "oblique" => pango::Style::Oblique,
            "italic" => pango::Style::Italic,
            _ => pango::Style::Normal,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SVGTextBox<'a> {
    markup: &'a str,
    width: i32,
    height: i32,
    font_desc: pango::FontDescription,
    alignment: pango::Alignment,
    grow: bool,
    insignificant_whitespace_present: bool
}

impl<'a> SVGTextBox<'a> {
    /// Generate a new textbox from the options given.
    ///
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// let mut tb = SVGTextBox::new("Hello World", 100, 100);
    ///
    /// // Further options can be given by chaining method calls together.
    /// // For example, to have left-aligned text set in italic Times New Roman:
    ///
    /// let times_new_roman_italic = pango::FontDescription::from_string("Times New Roman italic");
    /// tb.set_alignment_from_str("left");
    ///	tb.set_font_desc(times_new_roman_italic);
    ///
    /// // Alternatively, these can be combined into one without requiring the textbox to be mutable:
    ///
    /// let times_new_roman = pango::FontDescription::from_string("Times New Roman");
    /// let tb = SVGTextBox::new("Hello World", 100, 100)
    ///						.set_alignment_from_str("centre")
    ///						.set_font_desc(times_new_roman);
    /// ```
    /// # Arguments
    ///
    /// * `markup`: the text to use, formatted in [Pango Markup Language](https://developer.gnome.org/pango/stable/PangoMarkupFormat.html) if desired.
    /// * `width`: the width of the eventual image, in pixels.
    /// * `height`: the height of the eventual image, in pixels.
    ///
    pub fn new(markup: &str, px_width: i32, px_height: i32) -> SVGTextBox {
        SVGTextBox {
            markup,
            width: px_width,
            height: px_height,
            font_desc: pango::FontDescription::new(),
            alignment: pango::Alignment::Center,
            grow: true,
            insignificant_whitespace_present: false,
        }
    }

    /// Markup often has insignificant whitespace between tags or, as a result of
    /// indentation, before text strings. This whitespace is treated as significant by
    /// Pango, but often it really shouldn't be.
    /// This method indicates that whitespace in markup should be removed in the following manner:
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
    pub fn markup_has_insignificant_whitespace(&mut self) -> &mut SVGTextBox<'a> {
        self.insignificant_whitespace_present = true;
        self
    }

    /// Set how text should be aligned.
    /// # Arguments
    /// * `a`: can be any of "left", "centre", "center", and "right". Any other string will result in centre-aligned text.
    ///
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// let mut tb = SVGTextBox::new("Hello World", 100, 100);
    /// tb.set_alignment_from_str("centre");
    /// ```
    pub fn set_alignment_from_str(&mut self, alignment_str: &str) -> &mut SVGTextBox<'a> {
        let alignment = match alignment_str {
            "left" => pango::Alignment::Left,
            "centre" | "center" => pango::Alignment::Center,
            "right" => pango::Alignment::Right,
            // might as well not panic
            _ => pango::Alignment::Center,
        };
        self.set_alignment(alignment)
    }

    /// Set how text should be aligned, using a `pango::Alignment` directly.
    ///
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// let mut tb = SVGTextBox::new("Hello World", 100, 100);
    /// tb.set_alignment(pango::Alignment::Right);
    /// ```
    pub fn set_alignment(&mut self, a: pango::Alignment) -> &mut SVGTextBox<'a> {
        self.alignment = a;
        self
    }

    /// Set the font using a descriptive string.
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// let mut tb = SVGTextBox::new("Hello World", 100, 100);
    /// tb.set_font_desc_from_str("Times New Roman").unwrap();
    /// // The above is the equivalent of
    /// let fd = pango::FontDescription::from_string("Times New Roman");
    /// tb.set_font_desc(fd);
    /// ```
    pub fn set_font_desc_from_str(&mut self, fd: &str) -> Result<&mut SVGTextBox<'a>, LayoutError> {
        let fd_parsed = pango::Layout::check_whitespace(fd);
        match fd_parsed {
            Ok(_) => Ok(self.set_font_desc(pango::FontDescription::from_string(fd))),
            Err(_) => Err(LayoutError::FontDescriptionStr),
        }
    }

    /// Set a new font description.
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// let mut tb = SVGTextBox::new("Hello World", 100, 100);
    /// let fd = pango::FontDescription::from_string("Serif");
    /// tb.set_font_desc(fd);
    /// ```
    pub fn set_font_desc(&mut self, fd: pango::FontDescription) -> &mut SVGTextBox<'a> {
        self.font_desc = fd;
        self
    }

    /// Do _not_ grow or shrink text, but keep it at its original size.
    /// ```
    /// # use svgtextbox::SVGTextBox;
    /// // "Hello World" will grow to fit.
    /// let tb = SVGTextBox::new("Hello World", 100, 100);
    ///
    /// // "Hello World" will be set in 10 point Sans.
    /// let static_tb = SVGTextBox::new("Hello World", 100, 100).set_font_desc_from_str("Sans 10").unwrap().set_static();
    /// ```
    pub fn set_static(&mut self) -> &mut SVGTextBox<'a> {
        self.grow = false;
        self
    }
}

trait LayoutBase {
    const MAX_MARKUP_LEN: i32 = 1000;
    const ACCEL_MARKER: char = '\u{00}';
    const NULL_CHAR: char = '\u{0}';
    const DISTANCE_MIN: i32 = 0;
    const DISTANCE_MAX: i32 = std::i32::MAX / pango::SCALE;

    fn generate() -> pango::Layout;
    fn generate_from(
        markup: &str,
        px_width: i32,
        px_height: i32,
        alignment: pango::Alignment,
        font_desc: &pango::FontDescription,
    ) -> Result<pango::Layout, LayoutError>;
    fn check_markup(markup: &str) -> Result<String, LayoutError>;
    fn check_whitespace(m: &str) -> Result<(), LayoutError>;
    fn calculate_top_padding(&self) -> i32;
    fn font_size(&self) -> i32;
}

impl LayoutBase for pango::Layout {
    /// Return the distance in pango units that would need to be
    /// moved down so that the ink extents of the layout appear vertically
    /// centred.
    fn calculate_top_padding(&self) -> i32 {
        let (ink_extents, _logical_extents) = self.get_extents();
        let surplus_height = self.get_height() - ink_extents.height;
        let top_padding = surplus_height / 2;
        // Need to offset by ink start also;
        top_padding - ink_extents.y
    }

    /// Create a new layout not linked to any particular
    /// surface.
    fn generate() -> pango::Layout {
        let fontmap = pangocairo::FontMap::get_default().unwrap();
        let context = fontmap.create_context().unwrap();
        pango::Layout::new(&context)
    }

    /// Generate a layout from the values specified in arguments.
    /// For a full description of these arguments, see `get_layout`.
    fn generate_from(
        markup: &str,
        px_width: i32,
        px_height: i32,
        alignment: pango::Alignment,
        font_desc: &pango::FontDescription,
    ) -> Result<pango::Layout, LayoutError> {
        // Quick check to see that distance values make sense.
        if (px_width <= Self::DISTANCE_MIN)
            | (px_width > Self::DISTANCE_MAX)
            | (px_height <= Self::DISTANCE_MIN)
            | (px_height > Self::DISTANCE_MAX)
        {
            return Err(LayoutError::Distance);
        }

        let layout = pango::Layout::generate();
        layout.set_font_description(font_desc);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_wrap(pango::WrapMode::Word);
        layout.set_alignment(alignment);
        let checked_markup = Self::check_markup(markup)?;
        layout.set_markup(&checked_markup);
        // height and width need to be adjusted to svg.
        let px_to_scaled_pts = |x: i32| -> i32 { ((x * pango::SCALE) as f32 * 0.75) as i32 };

        layout.set_width(px_to_scaled_pts(px_width));
        layout.set_height(px_to_scaled_pts(px_height));
        Ok(layout)
    }

    /// Check if `m` has non-whitespace chars, is not empty,
    /// does not have null chars which will panic pango, etc.
    fn check_whitespace(m: &str) -> Result<(), LayoutError> {
        // check has non-whitespace chars, no null chars, etc
        let non_whitespace = m.chars().any(|c| !c.is_whitespace());
        if m.is_empty() | !non_whitespace {
            return Err(LayoutError::MarkupWhitespace);
        }
        if m.contains(Self::NULL_CHAR) | m.contains(Self::ACCEL_MARKER) {
            return Err(LayoutError::MarkupNullChar);
        }
        Ok(())
    }

    /// Check markup for errors. Return the input if there are no errors.
    /// If errors are fixable, return a string
    /// with the appropriate changes made. Otherwise, return a `LayoutError`
    /// specifying the problem.
    fn check_markup(initial_markup: &str) -> Result<String, LayoutError> {
        // check length
        let mut markup = initial_markup.trim().to_string();
        let too_long = (markup.len() as i32) > Self::MAX_MARKUP_LEN;
        if too_long {
            return Err(LayoutError::MarkupTooLong);
        }

        Self::check_whitespace(&markup)?;

        // Fix isolated and unambiguous ampersands
        if markup.contains('&') {
            let isolated_ampersand = Regex::new(r"&(?P<w>\s+)").unwrap();
            if isolated_ampersand.is_match(&markup) {
                let n = isolated_ampersand
                    .replace_all(&markup, "&amp;$w")
                    .to_string();
                markup = n;
            }
        }
        // Run an experimental parse and see if Pango complains.
        let experimental_parse = pango::parse_markup(&markup, Self::ACCEL_MARKER);
        match experimental_parse {
            Ok(_) => Ok(markup),
            Err(pango_err) => Err(LayoutError::BadMarkup {
                msg: pango_err.to_string(),
            }),
        }
    }

    /// get the base size of this layout's font description.
    /// Returns the default font description's size (0) if
    /// no font description has been set.
    fn font_size(&self) -> i32 {
        self.get_font_description().unwrap_or_default().get_size()
    }
}

trait LayoutOutput {
    fn as_bytes(&self) -> Result<Vec<u8>, LayoutError>;
}

impl LayoutOutput for pango::Layout {
    /// return this layout as a vector of bytes representing
    /// a svg file.
    fn as_bytes(&self) -> Result<Vec<u8>, LayoutError> {
        // we need height and width to be set

        let unscaled_pts = |x: i32| -> f64 { f64::from(x) / f64::from(pango::SCALE) };

        let width = match self.get_width() {
            w if w > 0 => unscaled_pts(w),
            _ => return Err(LayoutError::WidthNotSet),
        };
        let height = match self.get_height() {
            h if h > 0 => unscaled_pts(h),
            _ => return Err(LayoutError::HeightNotSet),
        };

        let mut writable = Vec::new();
        let surface = cairo::svg::RefWriter::new(width, height, &mut writable);
        let context = cairo::Context::new(&surface);
        context.move_to(0.0, f64::from(self.calculate_top_padding() / pango::SCALE));
        pangocairo::functions::show_layout(&context, self);
        surface.finish();

        Ok(writable)
    }
}

trait LayoutSizing {
    fn fits(&self) -> bool;
    fn grow_to_maximum_font_size(&self) -> Result<i32, LayoutError>;
    fn last_char_index(&self) -> i32;
    fn change_font_size(&self, new_font_size: i32);

    const MAX_FONT_SIZE: i32 = 500;
    const DEFAULT_FONT_SIZE: i32 = 10;
}

impl LayoutSizing for pango::Layout {
    /// Whether this layout fits within a box of
    /// `layout.get_width()` x `layout.get_height()`.
    /// This means that the text is not ellipsized
    /// and no text or part of text ink extents are
    /// outside the box.
    /// It is important to note that this relies on Pango's
    /// reporting, which is _not_ necessarily reliable
    /// when multiple paragraphs are involved.
    /// Further, more intensive, checks are required to be sure.
    fn fits(&self) -> bool {
        let ellipsized = self.is_ellipsized();
        let (ink_extents, _) = self.get_extents();
        let northwest_bounds_exceeded = (ink_extents.x < 0) | (ink_extents.y < 0);
        let southeast_bounds_exceeded = ((ink_extents.height + ink_extents.y) > self.get_height())
            | ((ink_extents.width + ink_extents.x) > self.get_width());

        !(ellipsized | northwest_bounds_exceeded | southeast_bounds_exceeded)
    }

    /// Get the index of the character furthest to the right
    /// in the last line of this layout.
    fn last_char_index(&self) -> i32 {
        let last_line = self.get_line_readonly(self.get_line_count() - 1).unwrap();
        // don't want to use wrapped version of x_to_index, because we don't care if to the right of the line.
        let x_pos = self.get_width();
        unsafe {
            let mut index_ = mem::uninitialized();
            let mut trailing = mem::uninitialized();
            let _ret: bool = from_glib(pango_sys::pango_layout_line_x_to_index(
                last_line.to_glib_none().0,
                x_pos,
                &mut index_,
                &mut trailing,
            ));
            index_
        }
    }

    /// Change the base font size of this layout.
    /// This will not override the sizes set in the original
    /// pango markup.
    fn change_font_size(&self, new_font_size: i32) {
        let mut font_desc: pango::FontDescription = self.get_font_description().unwrap_or_default();
        font_desc.set_size(new_font_size);
        self.set_font_description(&font_desc);
    }

    /// Grow this layout to the largest possible font size.
    fn grow_to_maximum_font_size(&self) -> Result<i32, LayoutError> {
        let orig_last_char = self.last_char_index();

        let will_fit = |new_font_size| {
            self.change_font_size(new_font_size);
            // pango occasionally reports fitting when in fact
            // lines are disappearing off the bottom.
            // here we check this by seeing if the index of the last
            // visible grapheme is the same as it was in the beginning.
            if self.fits() & (self.last_char_index() == orig_last_char) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        };

        let font_sizes_vec = (0..Self::MAX_FONT_SIZE).collect::<Vec<i32>>();
        let search_result = font_sizes_vec.binary_search_by(|i| will_fit(i * pango::SCALE));
        let index: i32 = search_result.err().unwrap() as i32;
        // Almost always this is an error representing a value too small;
        // but just in case we have 1pt text...
        // We don't worry about if the result is greater than max size,
        // since the correct approach is just to return the max size and move on.
        let usize_i = match index {
            i if i < 1 => return Err(LayoutError::MinFontSize),
            1 => 1 as usize,
            _ => (index - 1) as usize,
        };

        let result = &font_sizes_vec[usize_i];
        self.change_font_size(result * pango::SCALE);
        Ok(*result)
    }
}

pub trait SVGTextboxOut {
    fn remove_markup_insignificant_whitespace(markup: &str) -> String;
    fn as_bytes(&self) -> Result<Vec<u8>, LayoutError>;
    fn as_embeddable_base64(&self) -> Result<String, LayoutError>;
    fn as_string(&self) -> Result<String, LayoutError>;
    fn to_file(&self, path: &str) -> Result<(), LayoutError>;
    fn layout_from(&self) -> Result<pango::Layout, LayoutError>;
    fn get_layout(
        markup: &str,
        px_width: i32,
        px_height: i32,
        font_desc: &pango::FontDescription,
        alignment: pango::Alignment,
        grow: bool,
        insignificant_whitespace_present: bool
    ) -> Result<pango::Layout, LayoutError>;
}

impl<'a> SVGTextboxOut for SVGTextBox<'a> {
    /// Get a new `pango::Layout`.
    /// # Example usage.
    /// ```
    /// # use svgtextbox::LayoutError;
    /// # use svgtextbox::SVGTextBox;
    /// # use svgtextbox::SVGTextboxOut;
    ///
    /// let font_desc = pango::FontDescription::from_string("Sans 10");
    /// // a static layout, where the text will be 10pts in size.
    /// let layout = SVGTextBox::get_layout("Hello World", 100, 100, &font_desc, pango::Alignment::Left, false, false).unwrap();
    /// // a flex layout, where the text will be whatever size is the largest that still fits.
    /// let layout = SVGTextBox::get_layout("Hello World", 100, 100, &font_desc, pango::Alignment::Left, true, false).unwrap();
    /// // Some basic checks will be conducted on input:
    /// let bad_layout = SVGTextBox::get_layout("\n", 100, 100, &font_desc, pango::Alignment::Left, false, false);
    /// assert_eq!(bad_layout.unwrap_err(), LayoutError::MarkupWhitespace);
    /// ```
    /// # Arguments
    /// * `markup`: the text to use, formatted in [Pango Markup Language](https://developer.gnome.org/pango/stable/PangoMarkupFormat.html) if desired.
    /// * `px_width`: the width of the layout, in pixels.
    /// * `height`: the height of the layout, in pixels.
    /// * `font_desc`: the `pango::FontDescription` to use in the layout. (This can be empty; if
    ///   the layout is static without a font size, a default size will be set.)
    /// * `alignment`: the text alignment of the layout.
    /// * `grow`: whether or not to increase the layout font size to the maximum size that
    ///    does not overflow boundaries.
    fn get_layout(
        markup: &str,
        px_width: i32,
        px_height: i32,
        font_desc: &pango::FontDescription,
        alignment: pango::Alignment,
        grow: bool,
        insignificant_whitespace_present: bool
    ) -> Result<pango::Layout, LayoutError> {
        let clean_markup = match insignificant_whitespace_present {
            false => markup.to_string(),
            true => SVGTextBox::remove_markup_insignificant_whitespace(markup),
        };

        let layout = pango::Layout::generate_from(&clean_markup, px_width, px_height, alignment, font_desc)?;
        if grow {
            layout.grow_to_maximum_font_size()?;
        } else {
            if layout.font_size() <= 0 {
                layout.change_font_size(pango::Layout::DEFAULT_FONT_SIZE * pango::SCALE);
            }
            if !layout.fits() {
                return Err(LayoutError::StaticFontNoFit);
            }
        }
        Ok(layout)
    }

    /// Generate a pango::Layout from this textbox.
    fn layout_from(&self) -> Result<pango::Layout, LayoutError> {
       let layout = Self::get_layout(
            self.markup,
            self.width,
            self.height,
            &self.font_desc,
            self.alignment,
            self.grow,
            self.insignificant_whitespace_present
        )?;
        Ok(layout) 
    }

    /// Remove insignificant whitespace from `markup`; see
    /// the documentation for `SVGTextBox::markup_has_insignificant_whitespace`
    /// for more details of the algorithm.
    fn remove_markup_insignificant_whitespace(markup: &str) -> String {
        let non_whitespace = markup.split_whitespace();
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

    ///Get a textbox rendered as a vector of bytes representing an svg.
    fn as_bytes(&self) -> Result<Vec<u8>, LayoutError> {
        let layout = self.layout_from()?;
        let as_bytes = layout.as_bytes()?;
        Ok(as_bytes)
    }

    ///Get a textbox rendered as a base64 string with the appropriate prefix for
    /// inclusion in other svgs.
    fn as_embeddable_base64(&self) -> Result<String, LayoutError> {
        let as_bytes = self.as_bytes()?;
        let as_b64 = base64::encode(&as_bytes);
        Ok(format!("data:image/svg+xml;base64, {}", as_b64))
    }

    /// Get a textbox as a string.
    fn as_string(&self) -> Result<String, LayoutError> {
        let as_bytes = self.as_bytes()?;
        let s = str::from_utf8(&as_bytes)?;
        Ok(s.to_string())
    }

    /// Write textbox as an svg file to path.
    fn to_file(&self, path: &str) -> Result<(), LayoutError> {
        let as_bytes = self.as_bytes()?;
        std::fs::write(path, as_bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // tests for layout base.

    #[test]
    fn test_layout_generate_from() {
        let r = pango::Layout::generate_from(
            "Hello & World",
            100,
            100,
            pango::Alignment::Left,
            &pango::FontDescription::new(),
        )
        .unwrap();
        assert_eq!(r.get_text().unwrap(), "Hello & World");
        assert_eq!(r.get_alignment(), pango::Alignment::Left);
        assert_eq!(
            r.get_font_description().unwrap(),
            pango::FontDescription::new()
        );
        assert_eq!(r.get_height(), 76800);
        assert_eq!(r.get_width(), 76800);
    }

    #[test]
    fn test_long_markup() {
        let mut many_chars = String::new();
        let excess_len: usize = 1001;
        for c in std::iter::repeat('a').take(excess_len) {
            many_chars.push(c);
        }
        let r = pango::Layout::check_markup(&many_chars);
        assert_eq!(r.unwrap_err(), LayoutError::MarkupTooLong);
    }

    #[test]
    fn test_null_markup() {
        let r = pango::Layout::check_markup("Hello \u{0}");
        assert_eq!(r.unwrap_err(), LayoutError::MarkupNullChar);
    }

    #[test]
    fn test_empty_markup() {
        let r = pango::Layout::check_markup("");
        assert_eq!(r.unwrap_err(), LayoutError::MarkupWhitespace);
    }

    #[test]
    fn test_all_whitespace_markup() {
        let r = pango::Layout::check_markup("   \n    ");
        assert_eq!(r.unwrap_err(), LayoutError::MarkupWhitespace);
    }

    #[test]
    fn test_escaped_ampersand_markup() {
        pango::Layout::check_markup("Trouble &amp; Strife").unwrap();
    }

    #[test]
    fn test_isolated_ampersand_markup() {
        let r = pango::Layout::check_markup("Trouble & Strife").unwrap();
        assert_eq!(r, "Trouble &amp; Strife");
    }

    #[test]
    fn test_unisolated_ampersand_markup() {
        let r = pango::Layout::check_markup("Trouble &amp Strife");
        assert_eq!(r.unwrap_err(), LayoutError::BadMarkup{msg: "Error on line 1: Entity did not end with a semicolon; most likely you used an ampersand character without intending to start an entity — escape ampersand as &amp;".into()});
    }

    #[test]
    fn test_unescaped_angle_brackets_markup() {
        let r = pango::Layout::check_markup("<censored>");
        assert_eq!(
            r.unwrap_err(),
            LayoutError::BadMarkup {
                msg: "Unknown tag \'censored\' on line 1 char 19".into()
            }
        );
    }

    #[test]
    fn test_incomplete_span_markup() {
        let r = pango::Layout::check_markup("<span>Trouble et Strife");
        assert_eq!(r.unwrap_err(), LayoutError::BadMarkup{msg: "Error on line 1 char 40: Element “markup” was closed, but the currently open element is “span”".into()});
    }

    #[test]
    fn test_font_size() {
        let font_desc = pango::FontDescription::from_string("Sans 10");
        let r = pango::Layout::generate_from("Hello", 100, 100, pango::Alignment::Left, &font_desc)
            .unwrap();
        assert_eq!(r.font_size(), r.get_font_description().unwrap().get_size());
        assert_eq!(r.font_size(), (10 * pango::SCALE));
    }

    // tests for layout output

    fn output_helper() -> pango::Layout {
        pango::Layout::generate_from(
            "A",
            10,
            10,
            pango::Alignment::Left,
            &pango::FontDescription::new(),
        )
        .unwrap()
    }

    #[test]
    fn to_bytes() {
        output_helper().as_bytes().unwrap();
    }

    #[test]
    fn to_bytes_width_height_not_set() {
        let layout = pango::Layout::generate();
        layout.set_markup("No width");
        let r = layout.as_bytes();
        assert_eq!(r.unwrap_err(), LayoutError::WidthNotSet);
        layout.set_width(10 * pango::SCALE);
        let r = layout.as_bytes();
        assert_eq!(r.unwrap_err(), LayoutError::HeightNotSet);
        layout.set_height(10 * pango::SCALE);
        layout.as_bytes().unwrap();
    }

    #[test]
    fn check_layout_to_surface_uses_pts() {
        let px_width = 10;
        let px_height = 10;
        let pt_width = 7.5;
        let pt_height = 7.5;

        let x = pango::Layout::generate_from(
            "A",
            px_width,
            px_height,
            pango::Alignment::Left,
            &pango::FontDescription::new(),
        )
        .unwrap()
        .as_bytes()
        .unwrap();
        let r = str::from_utf8(&x).unwrap();
        let check_str = format!("width=\"{}pt\" height=\"{}pt\"", pt_width, pt_height);
        assert!(r.contains(&check_str));
    }

    #[test]
    fn lines_drop() {
        let layout = pango::Layout::generate_from(
            "A\n\n\n\n\nB",
            500,
            500,
            pango::Alignment::Center,
            &pango::FontDescription::new(),
        )
        .unwrap();
        let changed_font_size = layout.grow_to_maximum_font_size().unwrap();
        assert!(changed_font_size == 46);
    }

    #[test]
    fn lines_drop_2() {
        let layout = pango::Layout::generate_from(
            "SOME BOOK\n――\nSOME MANY NAMED AUTHOR",
            2000,
            1200,
            pango::Alignment::Center,
            &pango::FontDescription::new(),
        )
        .unwrap();
        let changed_font_size = layout.grow_to_maximum_font_size().unwrap();
        assert!(changed_font_size == 139);
    }

    #[test]
    fn lines_drop_3() {
        let layout = pango::Layout::generate_from("SOME TITLE\n――\nSOME AUTHOR\n<span size=\"smaller\"><span style=\"italic\">Edited by</span>\nSOME EDITOR</span>", 2000, 2000, pango::Alignment::Center, &pango::FontDescription::new()).unwrap();
        let changed_font_size = layout.grow_to_maximum_font_size().unwrap();
        assert!(changed_font_size == 192);
    }

    #[test]
    fn test_get_static_layout() {
        let markup = "Hello World";
        let r = SVGTextBox::get_layout(
            markup,
            100,
            100,
            &pango::FontDescription::new(),
            pango::Alignment::Left,
            false,
            false
        )
        .unwrap();
        assert_eq!(
            r.font_size(),
            pango::Layout::DEFAULT_FONT_SIZE * pango::SCALE
        );
    }

    #[test]
    fn test_get_static_layout_font_size_set() {
        let markup = "Hello World";
        let mut font_desc = pango::FontDescription::new();
        let twelve_pt = 12 * pango::SCALE;
        font_desc.set_size(twelve_pt);
        let r = SVGTextBox::get_layout(markup, 100, 100, &font_desc, pango::Alignment::Left, false, false)
            .unwrap();
        assert_eq!(r.font_size(), twelve_pt);
    }

    #[test]
    fn test_get_static_layout_font_size_set_too_large() {
        let markup = "Hello World";
        let mut font_desc = pango::FontDescription::new();
        let large_pt = 120 * pango::SCALE;
        font_desc.set_size(large_pt);
        let l = SVGTextBox::get_layout(markup, 100, 100, &font_desc, pango::Alignment::Left, false, false);
        assert_eq!(l.unwrap_err(), LayoutError::StaticFontNoFit);
    }

    #[test]
    fn test_get_flex_layout() {
        let r = SVGTextBox::get_layout(
            "Hello World",
            100,
            100,
            &pango::FontDescription::new(),
            pango::Alignment::Left,
            true,
            false
        )
        .unwrap();
        assert_eq!(r.font_size(), 22528);
    }

    #[test]
    fn test_padding() {
        let mut font_desc = pango::FontDescription::new();
        font_desc.set_size(20 * pango::SCALE);
        let layout =
            pango::Layout::generate_from("Jyrfg", 100, 100, pango::Alignment::Center, &font_desc)
                .unwrap();

        let reported_offset_padding = layout.calculate_top_padding();

        let (ink_extents, _logical_extents) = layout.get_extents();
        let start = ink_extents.y;
        let end = ink_extents.y + ink_extents.height;

        let total_height_from_start = end + reported_offset_padding;
        let bottom_padding = layout.get_height() - total_height_from_start;

        let offset_bottom_padding = bottom_padding - start;

        // can't rely on absolute equality with integers.
        let approx_equal = (offset_bottom_padding - 1 == reported_offset_padding)
            | (offset_bottom_padding == reported_offset_padding)
            | (offset_bottom_padding + 1 == reported_offset_padding);
        assert!(approx_equal);
    }

    // Property testing

    extern crate proptest;
    use proptest::prelude::*;

    fn alignment() -> impl Strategy<Value = pango::Alignment> {
        prop_oneof![
            Just(pango::Alignment::Left),
            Just(pango::Alignment::Right),
            Just(pango::Alignment::Center),
        ]
    }

    proptest! {
        #[test]
        fn no_crashes(markup in ".*", height in prop::num::i32::ANY, width in prop::num::i32::ANY, alignment in alignment(), grow in prop::bool::ANY, insignificant_whitespace_present in prop::bool::ANY) {
            let font_desc = pango::FontDescription::new();
            let _r = SVGTextBox::get_layout(&markup, width, height, &font_desc, alignment, grow, insignificant_whitespace_present);
        }
    }

    // Api testing

    #[test]
    fn test_option_setting() {
        let mut tb = SVGTextBox::new("A", 10, 10);
        tb.set_alignment(pango::Alignment::Right);
        let fd = pango::FontDescription::from_string("Serif 10");
        tb.set_font_desc(fd.clone());
        tb.set_static();
        assert_eq!(tb.alignment, pango::Alignment::Right);
        assert_eq!(tb.font_desc, fd);
        assert_eq!(tb.grow, false);
        tb.set_alignment_from_str("left");
        tb.set_font_desc_from_str("Sans 10").unwrap();
        assert_eq!(tb.alignment, pango::Alignment::Left);
        assert_eq!(tb.font_desc, pango::FontDescription::from_string("Sans 10"));
    }

    #[test]
    fn test_svg_bytes() {
        panic!();
    }

    #[test]
    fn test_embeddable_base64_svg() {
        panic!();
    }

    #[test]
    fn test_svg_string() {
        panic!();
    }

    #[test]
    fn test_svg_to_file() {
        panic!();
    }

    #[test]
    fn test_insignificant_whitespace_is_removed_if() {
        let tb = SVGTextBox::new("\tA", 10, 10);
        let layout = tb.layout_from().unwrap();
        assert_eq!(layout.get_text().unwrap(), "A");
    }

    #[test]
    fn test_insignificant_whitespace_removal_algorithm() {
        let remove_whitespace = SVGTextBox::remove_markup_insignificant_whitespace;
        assert_eq!(remove_whitespace("\tA"), "A");
        assert_eq!(remove_whitespace("<span>\tA</span>"), "<span>A</span>");
        assert_eq!(remove_whitespace("<span>A B</span>"), "<span>A B</span>");
        assert_eq!(remove_whitespace("<span>A <br/> B</span>"), "<span>A\nB</span>");
        assert_eq!(remove_whitespace("<span>\n\tOuter\n\t<span>\n\t\tInner\n\t</span>\n</span>"), "<span>Outer<span>Inner</span></span>");
    }


    #[test]
    fn test_get_font_description() {
        let fd = get_font_description(Some("Times New Roman"), Some("700"), Some("italic"));
        assert_eq!(fd.get_family().unwrap(), "Times New Roman");
        assert_eq!(fd.get_weight(), pango::Weight::Bold);
        assert_eq!(fd.get_style(), pango::Style::Italic);
    }

}
