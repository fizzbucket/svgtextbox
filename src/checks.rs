extern crate regex;
use regex::Regex;
extern crate pango;

const MAX_MARKUP_LEN: i32 = 1000;
pub const DEFAULT_FONT_SIZE: i32 = 10; // default font size to use if not scaling text, in pts.
pub const MAX_FONT_SIZE: i32 = 500; // maximum font size possible, in pts.
pub const SCALED_MIN_FONT_SIZE: i32 = pango::SCALE;
pub const SCALED_MAX_FONT_SIZE: i32 = MAX_FONT_SIZE * pango::SCALE;
pub const PX_TO_PT_RATIO: f32 = 0.75; // the ratio of pixels to points; ie Ypx multiplied by PX_TO_PT_RATIO = Ypts,
// or, of course, Ypts divided by PX_TO_PT_RATIO = Ypx.


#[derive(Debug, PartialEq)]
pub struct DistanceMeasure {
	value: i32,
}

impl DistanceMeasure {

	/// Get a new distance measure.
	/// # Arguments
	///
	/// * `n`: the distance in pixels.
	pub fn new(n: i32) -> DistanceMeasure {
		DistanceMeasure{value: n}
	}

	pub fn as_pts(&self) -> f32 {
		self.value as f32 * PX_TO_PT_RATIO
	}

	pub fn as_px(&self) -> i32 {
		self.value
	}

	pub fn as_scaled_pts(&self) -> i32 {
		self.as_pts() as i32 * pango::SCALE
	}
}





#[derive(Debug, PartialEq)]
pub struct FontSize {
	value: i32,
}

impl FontSize {

	pub fn new(value: i32) -> Result<FontSize, i32> {

		let not_set = value == 0;
		let too_big = value > SCALED_MAX_FONT_SIZE;
		let too_small = value < SCALED_MIN_FONT_SIZE;

		if too_big | too_small | not_set {
			return Err(value)
		}

		Ok(FontSize {
			value
		})
	}

	#[allow(dead_code)]
	pub fn pts(&self) -> i32 {
		self.value / pango::SCALE
	}

	pub fn scaled(&self) -> i32 {
		self.value
	}

	pub fn step_down(&self) -> Result<FontSize, i32> {
		FontSize::new(self.value - pango::SCALE)
	}

	pub fn step_up(&self) -> Result<FontSize, i32> {
		FontSize::new(self.value + pango::SCALE)
	}

	pub fn range() -> FontSize {
		FontSize{value: SCALED_MIN_FONT_SIZE - pango::SCALE}
	}

}


impl Iterator for FontSize {
	type Item = FontSize;

	fn next(&mut self) -> Option<FontSize> {
		let n = self.step_up();
		if !n.is_err() {
			let x = n.unwrap();
			self.value = x.value;
			return Some(x);
		} else {
			return None
		}
	}
}

impl Default for FontSize {
	fn default() -> Self {
		FontSize::new(DEFAULT_FONT_SIZE * pango::SCALE).unwrap()
	}
}


pub trait FontDescriptionExt {
	fn change_size(&mut self, new_size: &FontSize);
	fn fetch_size(&self) -> Result<FontSize, i32>;
}

impl FontDescriptionExt for pango::FontDescription {

	fn change_size(&mut self, new_size: &FontSize) {
		self.set_size(new_size.scaled());
	}

	fn fetch_size(&self) -> Result<FontSize, i32> {
		let size = self.get_size();
		FontSize::new(size)
	}
}

pub struct Markup {
	value: String
}

impl Markup {

	pub fn new(m: &str) -> Result<Markup, &'static str> {
		let mut markup = m.trim().to_string();
		if (markup.len() as i32) > MAX_MARKUP_LEN {
			return Err("Markup too long");
		}

		let accel_marker = '\u{00}';
		let nullchar = markup.contains('\u{0}') | markup.contains(accel_marker);
		let has_non_whitespace = markup.chars().any(|c| !c.is_whitespace());
		if markup.is_empty() | !has_non_whitespace | nullchar {
			return Err("Markup contains a null character or is empty");
		}

		if markup.contains("&") {
			let isolated_ampersand = Regex::new(r"&(?P<w>\s+)").unwrap();
			if isolated_ampersand.is_match(&m) {
				let n = isolated_ampersand.replace_all(m, "&amp;$w").to_string();
				markup = n;
			}
		}

		let experimental_parse = pango::parse_markup(&markup, accel_marker);
		if experimental_parse.is_err() {
			return Err("Improperly formatted markup");
		}

		Ok(Markup {
			value: markup,
		})
	}

	pub fn to_string(&self) -> String {
		self.value.clone()
	}
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn distance_measure() {
    	let i = DistanceMeasure::new(100);
    	assert_eq!(i.as_pts(), 75.0);
    	assert_eq!(i.as_px(), 100);
    	assert_eq!(i.as_scaled_pts(), 75 * 1024);
    }

	#[test]
	fn fontsize() {
		let f = FontSize::new(1024).unwrap();
		assert_eq!(f.scaled(), 1024);
		let f2 = f.step_up().unwrap();
		assert_eq!(f2.scaled(), 1024 * 2);
		assert_eq!(f2.pts(), 2);
		assert_eq!(f2.step_down().unwrap().scaled(), f.scaled());
	}

	#[test]
	#[should_panic]
	fn fontsize_zero() {
		FontSize::new(0).unwrap();
	}

	#[test]
	fn fontsize_default() {
		let f: FontSize = Default::default();
		assert_eq!(f.scaled(), DEFAULT_FONT_SIZE * pango::SCALE);
		assert_eq!(f.pts(), DEFAULT_FONT_SIZE);
	}

	#[test]
	#[should_panic]
	fn too_big_fontsize() {
		FontSize::new(SCALED_MAX_FONT_SIZE + 1).unwrap();
	}

	#[test]
	#[should_panic]
	fn too_small_fontsize() {
		FontSize::new(1).unwrap();
	}

	#[test]
	#[should_panic]
	fn negative_fontsize() {
		FontSize::new(-1).unwrap();
	}


	#[test]
	fn change_size() {
		let mut fd = pango::FontDescription::new();
		let size = DEFAULT_FONT_SIZE * 1024;
		let fs = FontSize::new(size).unwrap();
		fd.change_size(&fs);
		let fp = fd.fetch_size().unwrap();
		assert_eq!(size, fp.value);
	}

	#[test]
	fn iteration() {
		let mut fsi = FontSize::range();
		let first = fsi.next().unwrap();
		assert_eq!(first, FontSize::new(SCALED_MIN_FONT_SIZE).unwrap());
		let second = fsi.next().unwrap();
		assert_eq!(second.value - first.value, pango::SCALE);
		assert_eq!(fsi.last().unwrap(), FontSize::new(SCALED_MAX_FONT_SIZE).unwrap());
	}





	#[test]
	#[should_panic(expected = "Markup too long")]
	fn long_markup() {
		let mut many_chars = String::new();
		for c in std::iter::repeat('a').take(MAX_MARKUP_LEN as usize + 1) {
			many_chars.push(c);
		}
		let result = Markup::new(&many_chars);
		result.unwrap();
	}

	#[test]
	#[should_panic(expected = "Markup contains a null character or is empty")]
	fn nullchar_markup() {
		let result = Markup::new("Hello \u{0}");
		result.unwrap();
	}

	#[test]
	#[should_panic(expected = "Markup contains a null character or is empty")]
	fn empty_markup() {
		let result = Markup::new("");
		result.unwrap();
	}

	#[test]
	#[should_panic(expected = "Markup contains a null character or is empty")]
	fn all_whitespace_markup() {
		let result = Markup::new("   \n    ");
		result.unwrap();
	}

	#[test]
	fn already_escaped_ampersand_markup() {
		let s = "Trouble &amp; Strife";
		let result = Markup::new(&s);
		assert_eq!(result.unwrap().value, s);
	}

	#[test]
	fn isolated_ampersand_markup() {
		let s = "Trouble & Strife";
		let y = "Trouble &amp; Strife";
		let result = Markup::new(s);
		assert_eq!(result.unwrap().value, y);
	}

	#[test]
	#[should_panic(expected = "Improperly formatted markup")]
	fn unisolated_ampersand_markup() {
		let result = Markup::new("Trouble &amp Strife");
		result.unwrap();
	}

	#[test]
	#[should_panic(expected = "Improperly formatted markup")]
	fn unescaped_angle_brackets_markup() {
		let result = Markup::new("<censored>");
		result.unwrap();
	}


	#[test]
	#[should_panic(expected = "Improperly formatted markup")]
	fn incomplete_span() {
		let result = Markup::new("<span>Trouble et Strife");
		result.unwrap();
	}


} 