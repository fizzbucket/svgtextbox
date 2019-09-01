use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::str::FromStr;
use std::default::Default;

static DEFAULT_MAX_SIZE: u16 = 500;
static DEFAULT_MIN_SIZE: u16 = 1;

/// Represent possible font sizes
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum FontSizing {
    /// A single size
    Static(u16),
    /// A range of sizes
    Range {
        min: u16,
        max: u16,
        #[serde(default)]
        step: Option<usize>,
    },
    /// A selection of possible sizes
    Flex(HashSet<u16>),
}

impl FontSizing {


    pub fn set_min_size(self, s: u16) -> Self {
        match self {
            FontSizing::Range{max, step, ..} => {
                FontSizing::Range{min: s, max, step}
            },
            _ => {
                FontSizing::Range{min: s, max: DEFAULT_MAX_SIZE, step: None}
            }
        }
    }

    pub fn set_max_size(self, s: u16) -> Self {
        match self {
            FontSizing::Range{min, step, ..} => {
                FontSizing::Range{min, max: s, step}
            },
            _ => {
                FontSizing::Range{min: DEFAULT_MIN_SIZE, max: s, step: None}
            }
        }
    }

    pub fn set_step(self, s: usize) -> Self {
        if let FontSizing::Range{min, max, ..} = self {
            FontSizing::Range{min, max, step: Some(s)}
        } else{self}
    }
    
    /// return a vector of all possible font sizes
    fn to_vec<T>(&self) -> Vec<T> 
        where T: From<u16>{
        use FontSizing::*;

        let v = match self {
            Static(i) => vec![*i],
            Flex(h) => {
                let mut v = h.iter().cloned().collect::<Vec<u16>>();
                v.sort_unstable();
                v
            },
            Range{min, max, step} => {
                let r = *min..=*max;
                match step {
                    Some(s) => r.step_by(*s).collect(),
                    None => r.collect()
                }
            }
        };

        v.into_iter().map(|n| n.into()).collect()
    }

    /// All possible font sizes at pango scale
    pub fn to_pango_scaled_vec(&self) -> Vec<i32> {
        self.to_vec::<i32>()
            .into_iter()
            .map(|i| i * pango::SCALE)
            .collect()
    }

    /// Construct a fontsizing from a vec. Note that there is a special case;
    /// a vec of only two sizes will be taken as indicating a range from v[0] to v[1].
    /// To indicate instead a selection of two possible numbers, put the largest number
    /// first
    fn from_vec<T>(i: Vec<T>) -> Self 
        where u16: From<T> {
        let v: Vec<u16> = i.into_iter().map(u16::from).collect();
        match v[..] {
            [] => FontSizing::default(),
            [single_value] => FontSizing::Static(single_value),
            [first, second] if first > second => {FontSizing::Flex(v.into_iter().collect())},
            [first, second] if first < second => {
                let as_range = FontSizing::Range{min: first, max: second, step: None};
                let as_vec = as_range.to_vec();
                FontSizing::Flex(as_vec.into_iter().collect())
            },
            _ => FontSizing::Flex(v.into_iter().collect()),
        }
    }
}

impl From<&FontSizing> for Vec<u16> {
    fn from(f: &FontSizing) -> Self {
        f.to_vec()
    }
}

impl From<Vec<u16>> for FontSizing {
    fn from(v: Vec<u16>) -> Self {
        FontSizing::from_vec(v)
    }
}


impl Default for FontSizing {
    fn default() -> Self {
        FontSizing::Range {
            min: 10,
            max: 30,
            step: None,
        }
    }
}


impl TryFrom<String> for FontSizing {
    type Error = std::num::ParseIntError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse::<FontSizing>()
    }
}

impl FromStr for FontSizing {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.split_whitespace()
                 .map(|s| s.parse::<u16>())
                 .collect::<Result<Vec<u16>, Self::Err>>()?;
        Ok(FontSizing::from(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        let fontsize_cases = [
            ("100", FontSizing::Static(100)),
            ("100 103", FontSizing::Flex(vec![100, 101, 102, 103].into_iter().collect())),
            ("200 100", FontSizing::Flex(vec![200, 100].into_iter().collect())),
            ("100 200 300", FontSizing::Flex(vec![100, 200, 300].into_iter().collect()))
        ];

        for (i, e) in fontsize_cases.iter() {
            assert_eq!(i.parse::<FontSizing>().unwrap(), *e);
            assert_eq!(FontSizing::try_from(i.to_string()).unwrap(), *e);
        }
    }

    #[test]
    fn default_set() {
        FontSizing::default();
    }

    #[test]
    fn test_fontsizing() {
        let a = ("10", vec![10]);
        let b = ("[10, 20, 30]", vec![10, 20, 30]);
        let c = ("{min: 10, max: 20}", (10..=20).collect::<Vec<u16>>());
        let d = ("{min: 10, max: 20, step: 2}", vec![10, 12, 14, 16, 18, 20]);

        for (s, expected) in [a, b, c, d].iter() {
            let f: FontSizing = serde_yaml::from_str(s).unwrap();
            let v = f.to_vec::<u16>();
            assert_eq!(&v, expected);
            assert_eq!(FontSizing::from_vec(v.clone()).to_vec::<u16>(), v);
        }
    }



}
