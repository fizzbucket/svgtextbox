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

    fn _generate_layout(&self, width: &i32, height: &i32) -> Result<pango::Layout, LayoutError> {
        let layout = pango::Layout::generate_from(&self.input.markup, *width, *height, self.input.alignment, &self.input.font_desc)?;

        match &self.input.fontsizing {
            FontSizing::Static(i) => {
                // this shouldn't have to happen, but might as well check?
                if !layout.font_size() == i * pango::SCALE {
                    layout.change_font_size(i * pango::SCALE);
                }
            },
            FontSizing::Selection(v) => {
                layout.grow_to_maximum_font_size(v)?;
            },
        }
        Ok(layout)
    }

    fn to_layout(&self) -> Result<pango::Layout, LayoutError>{

        match &self.input.dimensions {
            LayoutDimensions::Static(width, height) => {
                let layout = self._generate_layout(width, height)?;
                if layout.fits() {
                    return Ok(layout)
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::StaticWidthFlexHeight(width, heights) => {
                for height in heights {
                    let layout = self._generate_layout(width, height)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::FlexWidthStaticHeight(widths, height) => {
                for width in widths {
                    let layout = self._generate_layout(width, height)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }
                Err(LayoutError::CouldNotFitLayout)
            },
            LayoutDimensions::Flex(widths, heights) => {
                // try to expand width first
                let max_width = widths.iter().max().unwrap();
                let min_height = heights.iter().min().unwrap();

                for width in widths {
                    let layout = self._generate_layout(width, min_height)?;
                    if layout.fits() {
                        return Ok(layout);
                    }
                }

                for height in heights {
                    let layout = self._generate_layout(max_width, height)?;
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
}

