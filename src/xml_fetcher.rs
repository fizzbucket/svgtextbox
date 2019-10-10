use std::num::NonZeroU16;
use crate::textbox::{TextBox, UnitContainer, PaddingSpecification, PangoCompatibleString, AlignmentWrapper, FontDescriptionWrapper};
use std::str::FromStr;
use std::collections::HashMap;
use crate::errors::SvgTextBoxError;
use std::convert::TryFrom;

struct FetchedValues<T>(Vec<T>);

impl From<FetchedValues<NonZeroU16>> for UnitContainer {
	fn from(v: FetchedValues<NonZeroU16>) -> Self {
		let min = v.0[0];
		let max = v.0[1];
		let step = v.0[2];
		UnitContainer::from_range_values(min, max, step)
	}
}

impl From<FetchedValues<u16>> for PaddingSpecification {
	fn from(v: FetchedValues<u16>) -> Self {
		PaddingSpecification::from(v.0.as_slice())
	}
}

trait AttrFetcher {
	type Item: FromStr;
	type DefaultItem: FromStr;

	fn get_from_primary(hm: &mut HashMap<&str, &str>) -> Result<Option<Self::Item>, <Self::Item as FromStr>::Err>;
	fn get_from_alternative(hm: &mut HashMap<&str, &str>) -> Result<Self::Item, <Self::DefaultItem as FromStr>::Err>
		where Self::Item: From<FetchedValues<Self::DefaultItem>>;
	fn get(hm: &mut HashMap<&str, &str>) -> Result<Self::Item, SvgTextBoxError>
		where
			SvgTextBoxError: From<<Self::Item as FromStr>::Err>,
			SvgTextBoxError: From<<Self::DefaultItem as FromStr>::Err>,
			Self::Item: From<FetchedValues<Self::DefaultItem>>,
	{
		let a = Self::get_from_primary(hm)?;
		let b = Self::get_from_alternative(hm)?;
		Ok(a.unwrap_or(b))
	}
}

impl <T> AttrFetcher for T
	where
		T: AttrFetcherVisitor,
		T::Target: From<FetchedValues<T::AltKeyType>>
{
	type Item = T::Target;
	type DefaultItem = T::AltKeyType;

	fn get_from_primary(hm: &mut HashMap<&str, &str>) -> Result<Option<Self::Item>, <Self::Item as FromStr>::Err> {
		hm.remove(<T>::primary_key())
		  .map(|a| a.parse::<Self::Item>())
		  .transpose()
	}

	fn get_from_alternative(hm: &mut HashMap<&str, &str>) -> Result<Self::Item, <Self::DefaultItem as FromStr>::Err> {
		let alts = <T>::alt_keys().iter()
			.map(|(key, default)| {
				hm.remove(key)
				  .unwrap_or(default)
				  .parse::<Self::DefaultItem>()
			})
			.collect::<Result<Vec<Self::DefaultItem>, _>>()?;
		let f = FetchedValues(alts);
		let r = <Self::Item as From<FetchedValues<Self::DefaultItem>>>::from(f);
		Ok(r)
	}
}

trait AttrFetcherVisitor {
	type Target: FromStr;
	type AltKeyType: FromStr;

	fn primary_key() -> &'static str;
	fn alt_keys() -> &'static[(&'static str, &'static str)]; 
}

macro_rules! fetcher {
	($visitorname:ident => $primarykey:expr;
		$($altkey:expr, $default:expr),+
		=> $alt:ty => $target:ty) => {
		struct $visitorname;

		impl AttrFetcherVisitor for $visitorname {
			type Target = $target;
			type AltKeyType = $alt;

			fn primary_key() -> &'static str {
				$primarykey
			}

			fn alt_keys() -> &'static [(&'static str, &'static str)] {
				&[$(($altkey, $default)),+]
			}
		}
	};
}

fetcher!(PaddingFetcher => "padding";
	"padding-top", "0",
	"padding-right", "0",
	"padding-bottom", "0",
	"padding-left", "0" => u16 => PaddingSpecification);

fetcher!(FontSizeFetcher => "font-size";
	"min-font-size", "1",
	"max-font-size", "100",
	"font-size-step", "1"
	=> NonZeroU16 => UnitContainer);

fetcher!(WidthFetcher => "width";
	"min-width", "100",
	"max-width", "1000",
	"width-step", "10"
	=> NonZeroU16 => UnitContainer);

fetcher!(HeightFetcher => "height";
	"min-height", "100",
	"max-height", "1000",
	"height-step", "10"
	=> NonZeroU16 => UnitContainer);

#[derive(Debug)]
pub(crate) struct TextBoxToRender{
	textbox: TextBox,
	pub x: u16,
	pub y: u16,
	prefix: String
}

impl TextBoxToRender {

	pub(crate) fn render(&self) -> Result<String, SvgTextBoxError> {
		let o = self.textbox.to_svg_image()?;
		let extra_info = format!("<?svgtextbox-prefix {}?>\n<?svgtextbox-x_offset {} ?>\n<svgtextbox-y_offset {}?>\n</svg>", self.prefix, self.x, self.y);
		let o = o.src.replace("</svg>", &extra_info);
		Ok(o)
	}

	pub(crate) fn get_translate_value(&self) -> Option<String> {
		if self.x==0 && self.y==0 {
			None
		} else {
			let t = format!("translate({},{})", self.x, self.y);
			Some(t)
		}
	}
}

impl TryFrom<HashMap<&str, &str>> for TextBoxToRender {
	type Error = SvgTextBoxError;
	
	fn try_from(mut s: HashMap<&str, &str>) -> Result<Self, Self::Error> {
		
		let markup = s.remove("markup")
			.unwrap()
			.parse::<PangoCompatibleString>()?;

		macro_rules! get_and_parse_attr {
			($k:expr => $t:ty) => {
				s.remove($k)
				 .map(|a| a.parse::<$t>())
				 .transpose()?
			};
			($k:expr, $d:expr => $t:ty ) => {
				s.remove($k)
					.unwrap_or($d)
					.parse::<$t>()?
			};
		}

		let x = get_and_parse_attr!("x", "0" => u16);
		let y = get_and_parse_attr!("y", "0" => u16);
		let prefix = s.remove("__id")
			.unwrap_or("textbox-00")
			.to_string();
		let width = WidthFetcher::get(&mut s)?;
		let height = HeightFetcher::get(&mut s)?;
		let padding = PaddingFetcher::get(& mut s)?;
		let fontsize = FontSizeFetcher::get(&mut s)?;
		let alignment = get_and_parse_attr!("alignment" => AlignmentWrapper);
		let font_desc = get_and_parse_attr!("font-desc" => FontDescriptionWrapper);
		
		let mut textbox = TextBox::new(markup, width, height)?;
		textbox.set_padding(padding);
		textbox.set_font_size(fontsize);
		if let Some(v) = alignment {
			textbox.set_alignment(v);
		};
		if let Some(v) = font_desc {
			textbox.set_font_desc(v);
		};

		if !s.is_empty() {
			let a = s.iter()
				.map(|(k,v)| (k.to_string(), v.to_string()))
				.collect::<HashMap<String, String>>();
			textbox.set_padding_attrs(a);
		}

		Ok(TextBoxToRender{
			x,
			y,
			prefix,
			textbox
		})
	}
}

pub(crate) fn get_textbox<'a>(markup: &'a str, mut attributes: HashMap<&str, &'a str>) -> Result<TextBoxToRender, SvgTextBoxError> {
	attributes.insert("markup", markup);
	TextBoxToRender::try_from(attributes) 
}