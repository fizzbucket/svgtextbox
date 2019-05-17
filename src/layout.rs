use crate::errors::LayoutError;
use pango::FontMapExt;
use pango::LayoutExt;

pub trait LayoutBase {
    const DISTANCE_MIN: i32 = 0;
    const DISTANCE_MAX: i32 = std::i32::MAX / pango::SCALE;

    fn generate() -> pango::Layout;
    fn generate_from(
        markup: &str,
        px_width: i32,
        px_height: i32,
        alignment: pango::Alignment,
        font_desc: &pango::FontDescription,
        explicit_font_size: Option<i32>
    ) -> Result<pango::Layout, LayoutError>;
    fn font_size(&self) -> i32;
}

impl LayoutBase for pango::Layout {

    /// Create a new layout not linked to any particular
    /// surface.
    fn generate() -> pango::Layout {
        let fontmap = pangocairo::FontMap::get_default().unwrap();
        let context = fontmap.create_context().unwrap();
        pango::Layout::new(&context)
    }

    /// Generate a layout from the values specified in arguments.
    fn generate_from(
        markup: &str,
        px_width: i32,
        px_height: i32,
        alignment: pango::Alignment,
        font_desc: &pango::FontDescription,
        explicit_font_size: Option<i32>
    ) -> Result<pango::Layout, LayoutError> {
        // Quick check to see that distance values make sense.
        if (px_width <= Self::DISTANCE_MIN)
            | (px_width > Self::DISTANCE_MAX)
            | (px_height <= Self::DISTANCE_MIN)
            | (px_height > Self::DISTANCE_MAX)
        {
            return Err(LayoutError::BadDistanceValues{msg: "Attempted to create a layout with invalid distance values".to_string()});
        }

        let layout = pango::Layout::generate();

        layout.set_font_description(font_desc);
        
        // It's conceivable that we were given an explicit
        // font size but not a font description set to it already.
        if let Some(i) = explicit_font_size {
            let scaled_font_size = i * pango::SCALE;
            // avoid having to have original font_desc as mutable
            let mut fetched_fd = layout.get_font_description().unwrap();
            if fetched_fd.get_size() != scaled_font_size {
                fetched_fd.set_size(scaled_font_size);
                layout.set_font_description(&fetched_fd);
            }
        }
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_wrap(pango::WrapMode::Word);
        layout.set_alignment(alignment);
        layout.set_markup(&markup);
        // height and width need to be adjusted to svg.
        let px_to_scaled_pts = |x: i32| -> i32 { ((x * pango::SCALE) as f32 * 0.75) as i32 };

        layout.set_width(px_to_scaled_pts(px_width));
        layout.set_height(px_to_scaled_pts(px_height));
        Ok(layout)
    }

    /// get the base size of this layout's font description.
    /// Returns the default font description's size (0) if
    /// no font description has been set.
    fn font_size(&self) -> i32 {
        self.get_font_description().unwrap_or_default().get_size()
    }
}


pub trait LayoutSizing {
    fn fits(&self) -> bool;
    fn grow_to_maximum_font_size(&self, possible_font_sizes: &Vec<i32>) -> Result<i32, LayoutError>;
    fn change_font_size(&self, new_font_size: i32);
}

impl LayoutSizing for pango::Layout {
    /// Whether this layout fits within a box of
    /// `layout.get_width()` x `layout.get_height()`.
    /// This means that the text is not ellipsized
    /// and no text or part of text ink extents are
    /// outside the box.
    fn fits(&self) -> bool {
        let ellipsized = self.is_ellipsized();
        let height = self.get_height();
        let width = self.get_width();
        let (ink_extents, logical_extents) = self.get_extents();
        let northwest_bounds_exceeded = (ink_extents.x < 0) | (ink_extents.y < 0);
        let southeast_bounds_exceeded = ((ink_extents.height + logical_extents.y) > height)
            | ((ink_extents.width + logical_extents.x) > width);
        
        // Now for the complicated bit.
        // Pango has a mystery habit of dropping lines
        // off the end if you let it.

        // so we check what the index of the char closest
        // to the bottom right is: as far as I can tell,\
        // this gets you to the last utf8 byte index;
        let (_inside, last_char_index, _trailing) = self.xy_to_index(width, height);

        // in an ideal world, we would just compare this last_char_index
        // to the total character count
        // and make sure that they were the same.
        // let reported_char_count = self.get_character_count();
        // but the character count is _not_ the utf8 bytes count.
        // We have to get this from the text itself:
        let text_string = self.get_text()
                           .unwrap()
                           .as_str()
                           .to_string();
        let dropped_chars = last_char_index != (text_string.len() as i32 -1);


        !(ellipsized | northwest_bounds_exceeded | southeast_bounds_exceeded | dropped_chars)
    }


    /// Change the base font size of this layout.
    /// This will not override the sizes set in the original
    /// pango markup.
    fn change_font_size(&self, new_font_size: i32) {
        let mut font_desc: pango::FontDescription = self.get_font_description().unwrap_or_default();
        font_desc.set_size(new_font_size);
        self.set_font_description(&font_desc);
    }

    /// Grow this layout to the largest possible font size within `possible_font_sizes`.
    fn grow_to_maximum_font_size(&self, possible_font_sizes: &Vec<i32>) -> Result<i32, LayoutError> {

        let will_fit = |new_font_size| {
            self.change_font_size(new_font_size);
            let fits = self.fits();
            if fits {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        };

        // n.b. binary search assumes vec is already sorted
        let mut p = possible_font_sizes.clone();
        p.sort_unstable();
        p.dedup();
        let search_result = p.binary_search_by(|i| will_fit(i * pango::SCALE));
        let index: i32 = search_result.err().unwrap() as i32;
        // Almost always this is an error representing a value too small;
        // but just in case we have 1pt text...
        // We don't worry about if the result is greater than max size,
        // since the correct approach is just to return the max size and move on.
        let usize_i = match index {
            i if i < 1 => return Err(LayoutError::CouldNotFitLayout),
            1 => 1 as usize,
            _ => (index - 1) as usize,
        };

        let result = &possible_font_sizes[usize_i];
        self.change_font_size(result * pango::SCALE);
        Ok(*result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::LayoutWrite;

    #[test]
    fn test_change_font_size() {
        let r = pango::Layout::generate_from(
            "Hello &amp; World",
            100,
            100,
            pango::Alignment::Left,
            &pango::FontDescription::from_string("Sans 10"),
            None
        ).unwrap();
        assert_eq!(r.font_size(), 10 * pango::SCALE);
        r.change_font_size(12 * pango::SCALE);
        assert_eq!(r.font_size(), 12 * pango::SCALE);
        let r = pango::Layout::generate_from(
            "Hello &amp; World",
            100,
            100,
            pango::Alignment::Left,
            &pango::FontDescription::from_string("Sans 10"),
            Some(12)
        ).unwrap();
        assert_eq!(r.font_size(), 12 * pango::SCALE);
    }

    #[test]
    fn test_layout_generate_from() {
        let r = pango::Layout::generate_from(
            "Hello &amp; World",
            100,
            100,
            pango::Alignment::Left,
            &pango::FontDescription::from_string("Sans 10"),
            None
        )
        .unwrap();
        assert_eq!(r.get_text().unwrap(), "Hello & World");
        assert_eq!(r.get_alignment(), pango::Alignment::Left);
        assert_eq!(
            r.get_font_description().unwrap(),
            pango::FontDescription::from_string("Sans 10")
        );
        assert_eq!(r.get_height(), 76800);
        assert_eq!(r.get_width(), 76800);
        assert_eq!(r.px_height(), 100);
        assert_eq!(r.px_width(), 100);
        assert_eq!(r.pts_height(), 75.0);
        assert_eq!(r.pts_width(), 75.0);
    }

    #[test]
    fn test_bad_layout_dists() {
        let l = pango::Layout::generate_from("Hello", 1, 0, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", 0, 0, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", 0, 1, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", -1, 1, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", 1, -1, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", -1, -1, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_err());
        let l = pango::Layout::generate_from("Hello", 10, 10, pango::Alignment::Left, &pango::FontDescription::new(), None);
        assert!(l.is_ok());

}

 
    #[test]
    fn test_font_size() {
        let font_desc = pango::FontDescription::from_string("Sans 10");
        let r = pango::Layout::generate_from("Hello", 100, 100, pango::Alignment::Left, &font_desc, None)
            .unwrap();
        assert_eq!(r.font_size(), r.get_font_description().unwrap().get_size());
        assert_eq!(r.font_size(), (10 * pango::SCALE));
        let r = pango::Layout::generate_from("Hello", 100, 100, pango::Alignment::Left, &font_desc, Some(12))
            .unwrap();
        assert_eq!(r.font_size(), r.get_font_description().unwrap().get_size());
        assert_eq!(r.font_size(), (12 * pango::SCALE));

    }


    #[test]
    fn lines_drop() {
        let layout = pango::Layout::generate_from(
            "A\n\n\n\n\nB",
            500,
            500,
            pango::Alignment::Center,
            &pango::FontDescription::new(),
            None
        )
        .unwrap();
        let poss_sizes = (0..50).collect::<Vec<i32>>();
        let changed_font_size = layout.grow_to_maximum_font_size(&poss_sizes).unwrap();
        assert_eq!(changed_font_size, 46);

    }

    #[test]
    fn test_size_limitations() {
        let layout = pango::Layout::generate_from(
            "A\n\n\n\n\nB",
            500,
            500,
            pango::Alignment::Center,
            &pango::FontDescription::new(),
            None
        )
        .unwrap();
        let poss_sizes = (0..=45).collect::<Vec<i32>>();
        let maxed_font_size = layout.grow_to_maximum_font_size(&poss_sizes).unwrap();
        assert_eq!(maxed_font_size, 45);

        let restricted_sizes = vec![10, 12, 24];
        let restricted_size = layout.grow_to_maximum_font_size(&restricted_sizes).unwrap();
        assert_eq!(restricted_size, 24);

        let poss_sizes = (47..50).collect::<Vec<i32>>();
        let min_font_size = layout.grow_to_maximum_font_size(&poss_sizes);
        assert!(min_font_size.is_err());
    }

    #[test]
    fn lines_drop_2() {
        let layout = pango::Layout::generate_from(
            "SOME BOOK\n――\nSOME MANY NAMED AUTHOR",
            2000,
            1200,
            pango::Alignment::Center,
            &pango::FontDescription::new(),
            None
        )
        .unwrap();
        let poss_sizes = (135..145).collect::<Vec<i32>>();
        let changed_font_size = layout.grow_to_maximum_font_size(&poss_sizes).unwrap();
        println!("{:?}", layout.font_size());
        println!("{:?}", layout.get_character_count());
        assert_eq!(changed_font_size, 139);
    }

    #[test]
    fn lines_drop_3() {
        let layout = pango::Layout::generate_from("SOME TITLE\n――\nSOME AUTHOR\n<span size=\"smaller\"><span style=\"italic\">Edited by</span>\nSOME EDITOR</span>", 2000, 2000, pango::Alignment::Center, &pango::FontDescription::new(), None).unwrap();
        let poss_sizes = (185..195).collect::<Vec<i32>>();
        let changed_font_size = layout.grow_to_maximum_font_size(&poss_sizes).unwrap();
        assert_eq!(changed_font_size, 192);
    }

    #[test]
    fn test_report_too_large() {
        let markup = "Hello World";
        let mut font_desc = pango::FontDescription::new();
        let large_pt = 120 * pango::SCALE;
        font_desc.set_size(large_pt);
        let l = pango::Layout::generate_from(markup, 100, 100, pango::Alignment::Left, &font_desc, None).unwrap();
        assert!(!l.fits());
    }

    #[test]
    fn test_fits_reporting() {
        let l = pango::Layout::generate_from("<markup><span foreground=\"#FFFFFF\" weight=\"500\">SOME TITLE</span>\n<span foreground=\"#FFFFFF\" weight=\"400\">SOME AUTHOR</span></markup>", 1998, 1198, pango::Alignment::Right, &pango::FontDescription::from_string("Reforma 2018"), Some(40)).unwrap();
        assert!(l.fits());
    }


}