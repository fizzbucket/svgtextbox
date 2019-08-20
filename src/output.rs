use pango::Layout;
use cairo;

pub trait LayoutWrite {
    fn as_bytes(&self) -> Vec<u8>;
    fn pts_height(&self) -> f64;
    fn pts_width(&self) -> f64;
    fn px_height(&self) -> i32;
    fn px_width(&self) -> i32;
    fn calculate_top_padding(&self) -> i32;
    fn to_output(&self) -> LayoutOutput;
    fn write_to_file<P: AsRef<std::path::Path>>(&self, p: P) -> Result<(), std::io::Error>;
}

impl LayoutWrite for pango::Layout {

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


    fn pts_height(&self) -> f64 {
        let scaled_height = self.get_height();
        f64::from(scaled_height) / f64::from(pango::SCALE)
    }

    fn pts_width(&self) -> f64 {
        let scaled_width = self.get_width();
        f64::from(scaled_width) / f64::from(pango::SCALE)

    }

    fn px_height(&self) -> i32 {
        let px_height_float = self.pts_height() / 0.75;
        px_height_float as i32

    }

    fn px_width(&self) -> i32 {
        let px_width_float = self.pts_width() / 0.75;
        px_width_float as i32
    }

    /// return this layout as a vector of bytes representing
    /// a svg file.
    fn as_bytes(&self) -> Vec<u8> {

        let mut writable = Vec::new();
        let surface = cairo::SvgSurface::for_stream(self.pts_width(), self.pts_height(), writable);
        let context = cairo::Context::new(&surface);
        context.move_to(0.0, f64::from(self.calculate_top_padding() / pango::SCALE));
        pangocairo::functions::show_layout(&context, self);
        let o = surface.finish_output_stream().unwrap();
        o.downcast::<Vec<u8>>().unwrap().to_vec()
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, p: P) -> Result<(), std::io::Error> {
        let b = self.as_bytes();
        std::fs::write(p, b)
    }

    fn to_output(&self) -> LayoutOutput {
        LayoutOutput {
            rendered: self.as_bytes(),
            height: self.px_height(),
            width: self.px_width(),
        }
    }
}


/// The output format of a textbox.
pub struct LayoutOutput {
	/// the textbox svg as bytes
    pub rendered: Vec<u8>,
    /// the svg height in pixels
    pub height: i32,
    /// the svg width in pixels
    pub width: i32,
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutBase;
    use std::str;


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
            None
        )
        .unwrap()
        .as_bytes();
        let r = str::from_utf8(&x).unwrap();
        let check_str = format!("width=\"{}pt\" height=\"{}pt\"", pt_width, pt_height);
        assert!(r.contains(&check_str));
    }

    #[test]
    fn dimensions_reporting() {
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
            None
        ).unwrap();

        assert_eq!(x.pts_height(), pt_height);
        assert_eq!(x.pts_width(), pt_width);
        assert_eq!(x.px_height(), px_height);
        assert_eq!(x.px_width(), px_width);
    }
    #[test]
    fn test_padding() {
        let mut font_desc = pango::FontDescription::new();
        font_desc.set_size(20 * pango::SCALE);
        let layout =
            pango::Layout::generate_from("Jyrfg", 100, 100, pango::Alignment::Center, &font_desc, None)
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
}



