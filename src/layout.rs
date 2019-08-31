use pango::{Layout, FontDescription, FontMapExt, Alignment, SCALE};
use std::error::Error;

#[derive(Debug)]
pub enum LayoutError {
	CouldNotFit {
		font_size: i32,
		width: i32,
		height: i32,
		text: Option<String>,
		font_desc: Option<FontDescription>,
	},
	BadConversion
}


impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
        	LayoutError::CouldNotFit{font_size, width, height, ..} => {
        		write!(f,
		        	"Could not fit layout at text size {} in a box of {} x {}",
		        	font_size, width, height)},
        	LayoutError::BadConversion => {
        		write!(f, "Error converting layout")
        	}
        }
    }
}

impl Error for LayoutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {None}
}

pub trait LayoutSource {
	fn possible_font_sizes(&self) -> Vec<i32>;
	fn possible_dimensions(&self) -> Vec<(i32, i32)>;
	fn font_description(&self) -> &FontDescription;
	fn markup(&self) -> &str;
	fn alignment(&self) -> Alignment;
}


pub trait ExtendedLayout {
    fn font_size(&self) -> i32;
    fn fits(&self) -> bool;
    fn set_font_size(&self, new_font_size: i32);
    fn grow_to_maximum_font_size(&self, possible_font_sizes: &[i32]) -> Result<(), LayoutError>;
    fn calculate_top_padding(&self) -> i32;
    fn _will_fit(&self, new_font_size: i32) -> std::cmp::Ordering;
    fn create_layout(src: &impl LayoutSource) -> Result<Layout, LayoutError>;
}

impl ExtendedLayout for Layout {

	fn create_layout(src: &impl LayoutSource) -> Result<Layout, LayoutError> {
		let fd = src.font_description();
        let possible_font_sizes = src.possible_font_sizes();
        let markup = src.markup();
        let possible_dimensions = src.possible_dimensions();
        let alignment = src.alignment();

        println!("{:?}", fd);
        println!("{:?}", possible_font_sizes);
        println!("{:?}", markup);
        println!("{:?}", possible_dimensions);
        println!("{:?}", alignment);

		let fontmap = pangocairo::FontMap::get_default().expect("Could not get pango fontmap");
        let context = fontmap
            .create_context()
            .expect("Could not create pango font context");
        let layout = pango::Layout::new(&context);
        layout.set_font_description(Some(fd));
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_wrap(pango::WrapMode::Word);
        layout.set_alignment(alignment);
        layout.set_markup(markup);

        let mut last_err = None;

        for (width, height) in possible_dimensions {
        	layout.set_width(width);
        	layout.set_height(height);
        	let r = layout.grow_to_maximum_font_size(&possible_font_sizes);
            if r.is_ok() {
                return Ok(layout);
            } else {
            	last_err = Some(r.unwrap_err())
            }
        }

        Err(last_err.expect("3"))
	}

    fn font_size(&self) -> i32 {
        self.get_font_description().unwrap_or_default().get_size()
    }

    /// Whether this layout fits within a box of
    /// `layout.get_width()` x `layout.get_height()`.
    /// This means that the text is not ellipsized
    /// and no text or part of text ink extents are
    /// outside the box.
    fn fits(&self) -> bool {
        let ellipsized = self.is_ellipsized();
        let height = self.get_height();
        let width = self.get_width();
        // Now for the complicated bit.
        // Pango has a mystery habit of dropping lines
        // off the end if you let it.
        // so we check what the index of the char closest
        // to the bottom right is: as far as I can tell,
        // this gets you to the last utf8 byte index;
        let (_inside, last_char_index, _trailing) = self.xy_to_index(width, height);

        // in an ideal world, we would just compare this last_char_index
        // to the total character count
        // and make sure that they were the same.
        // let reported_char_count = self.get_character_count();
        // but the character count is _not_ the utf8 bytes count.
        // We have to get this from the text itself:
        let text_string = self.get_text().expect("No text");
        let dropped_chars = last_char_index != (text_string.len() as i32 - 1);
        !(ellipsized | dropped_chars)
    }

    fn set_font_size(&self, new_font_size: i32) {
        let mut font_desc: pango::FontDescription = self.get_font_description().unwrap_or_default();
        font_desc.set_size(new_font_size);
        self.set_font_description(Some(&font_desc));
    }

    fn _will_fit(&self, new_font_size: i32) -> std::cmp::Ordering {
        self.set_font_size(new_font_size);
        if self.fits() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }

    fn grow_to_maximum_font_size(&self, possible_font_sizes: &[i32]) -> Result<(), LayoutError> {
        let search_result = possible_font_sizes.binary_search_by(|i| self._will_fit(*i));
        let index = search_result.err().expect("2");
        // We don't worry about if the result is greater than max size,
        // since the correct approach is just to return the max size and move on.
        if index == 0 {
          return Err(
                LayoutError::CouldNotFit {
                    font_size: possible_font_sizes[0],
                    width: self.get_width(),
                    height: self.get_height(),
                    text: self.get_text().map(|g| g.to_string()),
                    font_desc: self.get_font_description()
                }
            );
        }
        let last_fit: usize = index - 1;
        let r = possible_font_sizes.get(last_fit).expect("1");
        self.set_font_size(*r);
        Ok(())
    }

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
}

pub trait ExportLayout {
	fn as_bytes(&self, width: Option<f64>, height: Option<f64>, x: Option<f64>, y: Option<f64>) -> Result<Vec<u8>, LayoutError>;
	fn as_string(&self, width: Option<f64>, height: Option<f64>, x: Option<f64>, y: Option<f64>) -> Result<String, Box<Error>>;
	//fn write_to_path<T: AsRef<Path>>(&self, p: &T) -> Result<(), Box<Error>>;
    //fn as_svg_image_tag(&self, x: i32, y: i32) -> Result<String, Box<Error>>;
}

impl ExportLayout for Layout {
	
    /// width: the image output width as distinct from the textbox width (defaults to textbox width)
    /// height: the image output height as distinct from the textbox height (defaults to textbox height)
    /// x: the x-coordinate to place the textbox on the surface (defaults to 0.0)
    /// y: the y-coordinate to place the textbox on the surface (defaults to 0.0)
    fn as_bytes(&self, width: Option<f64>, height: Option<f64>, x: Option<f64>, y: Option<f64>) -> Result<Vec<u8>, LayoutError> {
        

        let width = width.unwrap_or(f64::from(self.get_width()) / f64::from(SCALE));
        let height = height.unwrap_or(f64::from(self.get_height()) / f64::from(SCALE));
        let writable = Vec::new();
        let surface = cairo::SvgSurface::for_stream(width, height, writable);
        let context = cairo::Context::new(&surface);
        let x = x.unwrap_or(0.0);
        let vertical_offset = f64::from(self.calculate_top_padding() / SCALE);
        let y = y.unwrap_or(0.0) + vertical_offset;
        context.move_to(
            x,
            y
        );
        pangocairo::functions::show_layout(&context, &self);
        let o = surface
            .finish_output_stream()
            .map_err(|_e| LayoutError::BadConversion)?;
        match o.downcast::<Vec<u8>>() {
            Ok(v) => Ok(v.to_vec()),
            Err(_e) => Err(LayoutError::BadConversion),
        }
    } 

	fn as_string(&self, width: Option<f64>, height: Option<f64>, x: Option<f64>, y: Option<f64>) -> Result<String, Box<Error>> {
        let as_bytes = self.as_bytes(width, height, x, y)?;
        let s = std::str::from_utf8(&as_bytes)?;
        Ok(s.to_string())
	}

    // fn as_svg_image_tag(&self, x: i32, y: i32) -> Result<String, Box<Error>> {
    //     let width = self.get_width() / SCALE;
    //     let height = self.get_height() / SCALE;
    //     let textbox = self.as_string()?;
    //     let b64 = base64::encode(&textbox);
    //     let prefixed_b64 = format!("data:image/svg+xml;base64, {}", b64);
    //     let s = format!("<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"{}\"/>",
    //         x, y, width, height, prefixed_b64);
    //     Ok(s)
    // }

    // fn shift_output(&self, x:i32, y:i32)
}
