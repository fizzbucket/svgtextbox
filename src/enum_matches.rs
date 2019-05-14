use pango::{Weight, Style, Variant, Stretch};
use crate::errors::LayoutError;

pub trait PangoEnumMatch where Self: Sized {
    fn from_str<>(s: &str) -> Result<Self, LayoutError>;
}

macro_rules! match_impl {
    ($target:path, $($pattern:pat => $result:path,)* ) => (
        impl PangoEnumMatch for $target {
            fn from_str(s: &str) -> Result<Self, LayoutError> {
                let x = match s {
                    $($pattern => $result,)*
                    _ => return Err(LayoutError::CouldNotTransformStrToPangoEnum{msg: s.to_string()}),
                };
                Ok(x)
            }
        })
}

match_impl!(Weight,
    "100" => Weight::Thin,
    "thin" => Weight::Thin,
    "hairline" => Weight::Thin,
    "200" => Weight::Ultralight,
    "ultralight" => Weight::Ultralight,
    "extralight" => Weight::Ultralight,
    "300" => Weight::Light,
    "light" => Weight::Light,
    "350" => Weight::Semilight,
    "semilight" => Weight::Semilight,
    "380" => Weight::Book,
    "book" => Weight::Book,
    "400" => Weight::Normal,
    "normal" => Weight::Normal,
    "500" => Weight::Medium,
    "medium" => Weight::Medium,
    "600" => Weight::Semibold,
    "semibold" => Weight::Semibold,
    "demibold" => Weight::Semibold,
    "700" => Weight::Bold,
    "bold" => Weight::Bold,
    "800" => Weight::Ultrabold,
    "ultrabold" => Weight::Ultrabold,
    "extrabold" => Weight::Ultrabold,
    "900" => Weight::Heavy,
    "black" => Weight::Heavy,
    "heavy" => Weight::Heavy,
    "1000" => Weight::Ultraheavy,
    "ultraheavy" => Weight::Ultraheavy,);

match_impl!(Style,
    "oblique" => Style::Oblique,
    "italic" => Style::Italic,
    "normal" => Style::Normal,);

match_impl!(Variant,
    "smallcaps"=> Variant::SmallCaps,
    "normal" => Variant::Normal,);

match_impl!(Stretch,
    "ultracondensed" => Stretch::UltraCondensed,
    "extracondensed" => Stretch::ExtraCondensed,
    "condensed" => Stretch::Condensed,
    "semicondensed" => Stretch::SemiCondensed,
    "normal" => Stretch::Normal,
    "semiexpanded" => Stretch::SemiExpanded,
    "expanded" => Stretch::Expanded,
    "extraexpanded" => Stretch::ExtraExpanded,
    "ultraexpanded" => Stretch::UltraExpanded,);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_variants() {
        let w = Weight::from_str("bold");
        let s = Style::from_str("italic");
        let v = Variant::from_str("smallcaps");
        let stretch = Stretch::from_str("condensed");
        assert_eq!(w.unwrap(), Weight::Bold);
        assert_eq!(s.unwrap(), Style::Italic);
        assert_eq!(v.unwrap(), Variant::SmallCaps);
        assert_eq!(stretch.unwrap(), Stretch::Condensed);

    }

    #[test]
    fn test_get_bad_variants() {
        let w = Weight::from_str("bad");
        let s = Style::from_str("bad");
        let v = Variant::from_str("bad");
        let stretch = Stretch::from_str("bad");
        assert!(w.is_err());
        assert!(s.is_err());
        assert!(v.is_err());
        assert!(stretch.is_err());
    }

    #[test]
    fn test_weight_equivalents() {
        let semibold = Weight::from_str("600").unwrap();
        let also_semibold = Weight::from_str("demibold").unwrap();
        let yeahp_semibold = Weight::from_str("semibold").unwrap();
        assert_eq!(semibold, also_semibold);
        assert_eq!(semibold, yeahp_semibold);
    }
}