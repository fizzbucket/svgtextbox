use std::num::ParseIntError;
use std::str::FromStr;
use std::convert::From;


/// Convert a string like "1 2 3" to a Vec<T> of integers.
pub fn vec_from_str<T>(s: &str) -> Result<Vec<T>, ParseIntError>
where
    T: FromStr,
    ParseIntError: From<<T as FromStr>::Err>,
{
    let o = s.split_whitespace();
    let mut v = Vec::new();
    for s in o {
        let parsed = s.parse::<T>()?;
        v.push(parsed);
    }
    Ok(v)
}