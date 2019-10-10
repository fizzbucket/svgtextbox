use std::error::Error;
use std::fmt::{self, Display};
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::ffi::NulError;
use glib::Error as GlibError;
use cairo::StreamWithError;
use std::any::Any;

#[derive(Debug)]
pub enum SvgTextBoxError {
    /// No valid widths were given
    NoValidWidths,
    /// No valid heights were given
    NoValidHeights,
    /// No valid font sizes were given
    NoValidFontSizes,
    /// Could not work out a way to make all the requirements for sizing match up
    CouldNotFit,
    // We tried to unwrap an option we thought was some
    UnexpectedNone,
    /// Tried to create an alignment from an invalid string
    InvalidAlignment,
    Utf8Error(Utf8Error),
    FromUtf8Error(FromUtf8Error),
    PCSWhitespace,
    BadChar(String),
    GlibErr(GlibError),
    MissingMarkup,
    BadIntParse(ParseIntError),
    NulError(NulError),
    CairoError(StreamWithError),
    Any(Box<dyn Any>),
    NSError,
    StackedTextboxes,
    Xml
}

impl Display for SvgTextBoxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SvgTextBoxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SvgTextBoxError::Utf8Error(e) => Some(e),
            SvgTextBoxError::GlibErr(e) => Some(e),
            SvgTextBoxError::BadIntParse(e) => Some(e),
            SvgTextBoxError::NulError(e) => Some(e),
            _ => None
        }
    }
}

impl From<Utf8Error> for SvgTextBoxError {
    fn from(e: Utf8Error) -> Self {
        SvgTextBoxError::Utf8Error(e)
    }
}

impl From<ParseIntError> for SvgTextBoxError {
    fn from(e: ParseIntError) -> Self {
        SvgTextBoxError::BadIntParse(e)
    }
}

impl From<NulError> for SvgTextBoxError {
    fn from(e: NulError) -> Self {
        SvgTextBoxError::NulError(e)
    }
}

impl From<GlibError> for SvgTextBoxError {
    fn from(e: GlibError) -> Self {
        SvgTextBoxError::GlibErr(e)
    }
}

impl From<StreamWithError> for SvgTextBoxError {
    fn from(e: StreamWithError) -> Self {
        SvgTextBoxError::CairoError(e)
    }
}

impl From<FromUtf8Error> for SvgTextBoxError {
    fn from(e: FromUtf8Error) -> Self {
        SvgTextBoxError::FromUtf8Error(e)
    }
}

impl From<Box<dyn Any>> for SvgTextBoxError {
    fn from(e: Box<dyn Any>) -> Self {
        SvgTextBoxError::Any(e)
    }
}