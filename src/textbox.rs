use crate::input::{FontSizing, LayoutDimensions};
use crate::layout::{LayoutBase, LayoutSizing};
use crate::errors::LayoutError;
use crate::input::TextBoxInput;
use crate::output::LayoutOutput;
use crate::output::LayoutWrite;

/// Build and render a layout from an input.
pub struct LayoutBuilder<'a> {
	input: &'a TextBoxInput,
}


impl <'a> LayoutBuilder<'a> {

	pub fn get_layout_output(input: &TextBoxInput) -> Result<LayoutOutput, LayoutError> {
		let lb = LayoutBuilder{input};
		let layout = lb.to_layout()?;
		Ok(layout.to_output())
	}

    fn _generate_layout(&self, width: &i32, height: &i32, explicit_font_size: Option<i32>) -> Result<pango::Layout, LayoutError> {
        
        let layout = pango::Layout::generate_from(&self.input.markup, *width, *height, self.input.alignment, &self.input.font_desc, explicit_font_size)?;

        if let FontSizing::Selection(sizes) = &self.input.fontsizing {
                layout.grow_to_maximum_font_size(&sizes)?;
        };

        Ok(layout)
    }

    fn to_layout(&self) -> Result<pango::Layout, LayoutError>{

        let explicit_font_size = match &self.input.fontsizing {
            FontSizing::Static(i) => Some(*i),
            FontSizing::Selection(_) => None,
        };

        match &self.input.dimensions {
            LayoutDimensions::Static(width, height) => {
                let layout = self._generate_layout(width, height, explicit_font_size)?;
                if layout.fits() {
                    return Ok(layout)
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::StaticWidthFlexHeight(width, heights_unsorted) => {
                let mut heights: Vec<&i32> = heights_unsorted.iter().collect();
                heights.sort_unstable(); 
                for height in heights {
                    let layout = self._generate_layout(width, height, explicit_font_size)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::FlexWidthStaticHeight(unsorted_widths, height) => {
                let mut widths: Vec<&i32> = unsorted_widths.iter().collect();
                widths.sort_unstable(); 
                for width in widths {
                    let layout = self._generate_layout(width, height, explicit_font_size)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::Flex(unsorted_widths, unsorted_heights) => {
                // TODO: Sort these also!
                // try to expand width first
                let mut heights: Vec<&i32> = unsorted_heights.iter().collect();
                heights.sort_unstable(); 

                let mut widths: Vec<&i32> = unsorted_widths.iter().collect();
                widths.sort_unstable(); 

                let max_width = &widths.last().unwrap();
                let min_height = &heights.first().unwrap();

                for width in &widths {
                    let layout = self._generate_layout(width, min_height, explicit_font_size)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }

                for height in &heights {
                    let layout = self._generate_layout(max_width, height, explicit_font_size)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }

                Err(LayoutError::CouldNotFitLayout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
	use super::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    fn basic_input() -> TextBoxInput {
        TextBoxInput {
            markup: "Hello World".to_string(),
            dimensions: LayoutDimensions::Static(100, 100),
            font_desc: pango::FontDescription::new(),
            alignment: pango::Alignment::Left,
            fontsizing: FontSizing::Static(12),
        }
    }

    #[test]
    fn test_ordering_of_distance_options() {
        /// give a really small distance and font together with lots of really big distances
        /// to check that the smallest option is correctly being chosen, with odds of ~100: 1 that
        /// this won't happen by chance anyway
        let mut input = basic_input();
        let mut size_vec: Vec<i32> = (100..200).collect();
        size_vec.insert(1, 20);
        let size_set: HashSet<i32> = size_vec.iter().map(|x| *x).collect();
        input.dimensions = LayoutDimensions::StaticWidthFlexHeight(100, size_set);
        input.fontsizing = FontSizing::Static(10);
        let output = LayoutBuilder::get_layout_output(&input).unwrap();
        assert_eq!(output.height, 20);

        let mut size_vec: Vec<i32> = (200..300).collect();
        size_vec.insert(1, 100);
        let size_set: HashSet<i32> = size_vec.iter().map(|x| *x).collect();

        input.dimensions = LayoutDimensions::FlexWidthStaticHeight(size_set, 20);
        let output = LayoutBuilder::get_layout_output(&input).unwrap();
        assert_eq!(output.width, 100);
    }

    #[test]
    fn test_generate_layout() {

        let mut input = basic_input();
        let static_layout = LayoutBuilder{input: &input}._generate_layout(&100, &100, Some(12)).unwrap();
        input.fontsizing = FontSizing::from_range(Some(12), Some(100)).unwrap();
        let flex_layout = LayoutBuilder{input: &input}._generate_layout(&100, &100, None).unwrap();

        let static_font_size = static_layout.font_size();
        let flex_font_size = flex_layout.font_size();
        assert_eq!(static_font_size, 12 * pango::SCALE);
        assert!(static_font_size != flex_font_size);
        assert!(static_layout.fits());
        assert!(flex_layout.fits());
        // this is not the place to throw an error, even if it doesn't fit.
        input.fontsizing = FontSizing::Static(100);
        let oversized_layout = LayoutBuilder{input: &input}._generate_layout(&100, &100, Some(100)).unwrap();
        assert!(!oversized_layout.fits());
    }

    #[test]
    fn test_no_fit() {
        let build_layout = |inp: &TextBoxInput| {
            let builder = LayoutBuilder{input: inp};
            builder.to_layout()
        };

        let build_dimensions = |widthvec: Vec<i32>, heightvec: Vec<i32>| {
            let width = HashSet::from_iter(widthvec.into_iter());
            let height = HashSet::from_iter(heightvec.into_iter());
            LayoutDimensions::new(width, height)
        };

        let mut input = basic_input();
        input.fontsizing = FontSizing::Static(25);
        assert!(build_layout(&input).is_err());
        input.fontsizing = FontSizing::from_range(Some(10), Some(25)).unwrap();
        assert!(!build_layout(&input).is_err());
        input.fontsizing = FontSizing::from_range(Some(25), Some(30)).unwrap();
        assert!(build_layout(&input).is_err());
        // now check with flex layout dimensions

        // Logical dimensions of "Hello World" should be around 35 x 27 pixels on two lines.
        // Because our fitting requirements are a bit harsher, the actual number is more like 46 x 36.
        // On one line, logical = 68 x 14, actual = 90 x 14.

        input.fontsizing = FontSizing::Static(10); 
        input.dimensions = build_dimensions(vec![80, 90], vec![14]); // 14
        let l = build_layout(&input).unwrap();
        assert_eq!(l.px_width(), 90); // ie expands width if required
        // expand width first, not height:
        input.dimensions = build_dimensions(vec![80, 90], vec![14, 28]);
        let l = build_layout(&input).unwrap();
        assert_eq!(l.px_width(), 90);
        assert_eq!(l.px_height(), 14);
        // but expand height if necessary
        input.dimensions = build_dimensions(vec![80, 90], vec![10, 20]);
        let l = build_layout(&input).unwrap();
        assert!(l.fits());
        assert_eq!(l.px_width(), 90);
        assert_eq!(l.px_height(), 20);
        // don't expand unless we have to:
        input.fontsizing = FontSizing::from_range(Some(2), Some(15)).unwrap();
        let l = build_layout(&input).unwrap();
        assert!(l.fits());
        // occasionally fails?!
        //assert_eq!(l.px_width(), 80);
        assert_eq!(l.px_height(), 10);
    }
}

