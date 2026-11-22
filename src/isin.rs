/// Finds the first valid ISIN inside a symbol such as `GF_IE00B4L5YC18` or `IE00B4L5YC18.DE`.
pub fn find(symbol: &str) -> Option<String> {
    let upper = symbol.to_ascii_uppercase();
    upper
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|run| run.len() >= 12)
        .flat_map(|run| (0..=run.len() - 12).map(move |i| &run[i..i + 12]))
        .find(|candidate| is_valid(candidate))
        .map(str::to_owned)
}

pub fn is_valid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 12
        || !b[..2].iter().all(u8::is_ascii_uppercase)
        || !b[2..11].iter().all(u8::is_ascii_alphanumeric)
        || !b[11].is_ascii_digit()
    {
        return false;
    }
    let digits: Vec<u32> = b
        .iter()
        .flat_map(|&c| {
            let v = (c as char).to_digit(36).unwrap();
            if v >= 10 {
                vec![v / 10, v % 10]
            } else {
                vec![v]
            }
        })
        .collect();
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let x = d * 2;
                x / 10 + x % 10
            } else {
                d
            }
        })
        .sum();
    sum % 10 == 0
}

/// Every valid ISIN mentioned anywhere in `text`.
pub fn find_all(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| is_valid(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_checksums() {
        assert!(is_valid("IE00B4L5YC18"));
        assert!(is_valid("US0378331005"));
        assert!(!is_valid("IE00B4L5YC19"));
        assert!(!is_valid("ie00B4L5YC18"));
    }

    #[test]
    fn finds_isin_in_symbols() {
        assert_eq!(find("GF_IE00B4L5YC18").as_deref(), Some("IE00B4L5YC18"));
        assert_eq!(find("ie00b4l5yc18.de").as_deref(), Some("IE00B4L5YC18"));
        assert_eq!(find("XIE00B4L5YC18").as_deref(), Some("IE00B4L5YC18"));
        assert_eq!(find("EUNM.DE"), None);
    }
}
