extern crate custom_error;
use custom_error::custom_error;
use std::str::Utf8Error;
use std::num::ParseIntError;

custom_error! {
    pub LayoutError
    Utf8Error{source: Utf8Error} = "Utf 8 Error",
    ParseInt{source: ParseIntError} = "Error parsing int",
    XMLMinidomReportedError{msg: String} = "Error parsing xml: {}",
    XMLCouldNotFindMarkup = "Could not find markup tag in an xml element",
    CouldNotTransformStrToPangoEnum{msg: String} = "Could not transform `{}` to a pango enum",
    StringNotPangoCompatible{msg: String} = "The following string is not compatible with pango: {}",
    StringTooLong{msg: String} = "The following string is too long: {}",
    BadDistanceValues{msg: String} = "{}",
    CouldNotFitLayout = "Attempted to create a layout which could not fit",
    BadFontRange = "Attempted to create a font range where the minimum was greater than the maximum.",
    BadFontFamily = "Attempted to set an unparseable font family name",
    IntsFromString{msg: String} = "Could not parse ints from str: {}"
}

impl From<minidom::Error> for LayoutError {
    fn from(e: minidom::Error) -> Self {
        LayoutError::XMLMinidomReportedError{msg: e.to_string()}
    }
}