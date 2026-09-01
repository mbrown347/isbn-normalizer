//! Checksum validation and normalization for ISBN-10 and ISBN-13/EAN-13 codes.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKind {
    Isbn10,
    Isbn13,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NormalizeError {
    Empty,
    WrongLength(usize),
    BadCharacter(char),
    ChecksumMismatch { expected: char, found: char },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizeError::Empty => write!(f, "empty input"),
            NormalizeError::WrongLength(n) => {
                write!(f, "expected 10 or 13 digits after cleanup, got {n}")
            }
            NormalizeError::BadCharacter(c) => write!(f, "unexpected character '{c}'"),
            NormalizeError::ChecksumMismatch { expected, found } => write!(
                f,
                "checksum mismatch: expected check digit '{expected}', found '{found}'"
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Normalized {
    pub kind: CodeKind,
    pub digits: String,
}

// Strips whitespace and hyphen-like separators that spreadsheets and barcode
// scanners tend to insert. Anything else is treated as a data error rather
// than silently dropped, since a stray letter usually means the field got
// mangled somewhere upstream.
fn strip_separators(input: &str) -> Result<String, NormalizeError> {
    let mut cleaned = String::with_capacity(input.len());
    for c in input.trim().chars() {
        match c {
            '0'..='9' => cleaned.push(c),
            'x' | 'X' => cleaned.push('X'),
            '-' | ' ' | '\u{2010}'..='\u{2015}' => continue,
            _ => return Err(NormalizeError::BadCharacter(c)),
        }
    }
    Ok(cleaned)
}

fn isbn10_checksum_digit(digits: &str) -> char {
    let sum: u32 = digits
        .chars()
        .take(9)
        .enumerate()
        .map(|(i, c)| (10 - i as u32) * c.to_digit(10).unwrap())
        .sum();
    match (11 - (sum % 11)) % 11 {
        10 => 'X',
        n => std::char::from_digit(n, 10).unwrap(),
    }
}

fn isbn13_checksum_digit(digits: &str) -> char {
    let sum: u32 = digits
        .chars()
        .take(12)
        .enumerate()
        .map(|(i, c)| {
            let weight = if i % 2 == 0 { 1 } else { 3 };
            weight * c.to_digit(10).unwrap()
        })
        .sum();
    std::char::from_digit((10 - (sum % 10)) % 10, 10).unwrap()
}

/// Cleans a single ISBN/EAN string and checks its final digit against the
/// checksum computed from the rest.
pub fn normalize(input: &str) -> Result<Normalized, NormalizeError> {
    let cleaned = strip_separators(input)?;
    if cleaned.is_empty() {
        return Err(NormalizeError::Empty);
    }
    match cleaned.len() {
        10 => {
            if cleaned[..9].contains('X') {
                return Err(NormalizeError::BadCharacter('X'));
            }
            let expected = isbn10_checksum_digit(&cleaned);
            let found = cleaned.chars().last().unwrap();
            if expected != found {
                return Err(NormalizeError::ChecksumMismatch { expected, found });
            }
            Ok(Normalized {
                kind: CodeKind::Isbn10,
                digits: cleaned,
            })
        }
        13 => {
            if cleaned.contains('X') {
                return Err(NormalizeError::BadCharacter('X'));
            }
            let expected = isbn13_checksum_digit(&cleaned);
            let found = cleaned.chars().last().unwrap();
            if expected != found {
                return Err(NormalizeError::ChecksumMismatch { expected, found });
            }
            Ok(Normalized {
                kind: CodeKind::Isbn13,
                digits: cleaned,
            })
        }
        n => Err(NormalizeError::WrongLength(n)),
    }
}

/// Converts a valid ISBN-10 to its ISBN-13 form (978 prefix, recomputed
/// check digit). ISBN-13 input is returned unchanged.
pub fn to_isbn13(code: &Normalized) -> Normalized {
    match code.kind {
        CodeKind::Isbn13 => Normalized {
            kind: CodeKind::Isbn13,
            digits: code.digits.clone(),
        },
        CodeKind::Isbn10 => {
            let mut digits = String::with_capacity(13);
            digits.push_str("978");
            digits.push_str(&code.digits[..9]);
            let check = isbn13_checksum_digit(&digits);
            digits.push(check);
            Normalized {
                kind: CodeKind::Isbn13,
                digits,
            }
        }
    }
}

/// Inserts hyphens into a canonical 13-digit ISBN at the EAN prefix,
/// registration group, registrant, and publisher boundaries, e.g.
/// "9780306406157" -> "978-0-306-40615-7".
///
/// Full boundary data (the official ISBN Range Message) is large and
/// changes over time, so this only hyphenates registration group 0
/// (English) down to the registrant, since that table is stable and
/// well documented. Other single-digit groups get a group-level split
/// with the registrant and publisher left joined, since we don't have
/// their registrant ranges. Multi-digit groups and unrecognized prefixes
/// are returned as a flat, unhyphenated string rather than guessed at.
pub fn hyphenate_isbn13(code: &Normalized) -> String {
    let d = &code.digits;
    if code.kind != CodeKind::Isbn13 || d.len() != 13 {
        return d.clone();
    }
    let prefix = &d[0..3];
    let group = &d[3..4];
    let rest = &d[4..12];
    let check = &d[12..13];
    match group {
        "0" => {
            let split = group0_registrant_len(rest);
            format!("{prefix}-{group}-{}-{}-{check}", &rest[..split], &rest[split..])
        }
        "1" | "2" | "3" | "4" | "5" | "7" => {
            format!("{prefix}-{group}-{rest}-{check}")
        }
        _ => d.clone(),
    }
}

// Registration group 0 splits its 8-digit registrant+publisher block by
// numeric range rather than a fixed position; a registrant "00"-"19"
// leaves 6 digits for the publisher, "85"-"89" leaves only 3, and so on.
// This is the standard table used by the English-language ISBN agency.
fn group0_registrant_len(rest: &str) -> usize {
    let n: u32 = rest.parse().expect("8 ASCII digits");
    match n {
        0..=19_999_999 => 2,
        20_000_000..=69_999_999 => 3,
        70_000_000..=84_999_999 => 4,
        85_000_000..=89_999_999 => 5,
        90_000_000..=94_999_999 => 6,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_clean_isbn10() {
        let n = normalize("0306406152").unwrap();
        assert_eq!(n.kind, CodeKind::Isbn10);
        assert_eq!(n.digits, "0306406152");
    }

    #[test]
    fn strips_hyphens_and_spaces() {
        let n = normalize(" 0-306-40615-2 ").unwrap();
        assert_eq!(n.digits, "0306406152");
    }

    #[test]
    fn accepts_isbn10_with_x_check_digit() {
        // A well-known ISBN-10 whose check digit is X.
        let n = normalize("155860832X").unwrap();
        assert_eq!(n.kind, CodeKind::Isbn10);
    }

    #[test]
    fn rejects_bad_isbn10_checksum() {
        let err = normalize("0306406153").unwrap_err();
        assert_eq!(
            err,
            NormalizeError::ChecksumMismatch {
                expected: '2',
                found: '3'
            }
        );
    }

    #[test]
    fn accepts_clean_isbn13() {
        let n = normalize("978-0-306-40615-7").unwrap();
        assert_eq!(n.kind, CodeKind::Isbn13);
        assert_eq!(n.digits, "9780306406157");
    }

    #[test]
    fn converts_isbn10_to_isbn13() {
        let n = normalize("0306406152").unwrap();
        let converted = to_isbn13(&n);
        assert_eq!(converted.digits, "9780306406157");
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(normalize("12345").unwrap_err(), NormalizeError::WrongLength(5));
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(normalize("   ").unwrap_err(), NormalizeError::Empty);
    }

    #[test]
    fn hyphenates_group_zero_by_registrant_range() {
        // 3-digit registrant (306 falls in the 200-699 band).
        let n = normalize("9780306406157").unwrap();
        assert_eq!(hyphenate_isbn13(&n), "978-0-306-40615-7");

        // 2-digit registrant (00 falls in the 00-19 band).
        let n = normalize("9780004722238").unwrap();
        assert_eq!(hyphenate_isbn13(&n), "978-0-00-472223-8");
    }

    #[test]
    fn hyphenates_other_single_digit_groups_without_registrant_split() {
        let n = normalize("9782070408504").unwrap();
        assert_eq!(hyphenate_isbn13(&n), "978-2-07040850-4");
    }

    #[test]
    fn leaves_unrecognized_groups_unhyphenated() {
        let n = normalize("9788000000008").unwrap();
        assert_eq!(hyphenate_isbn13(&n), "9788000000008");
    }

    #[test]
    fn hyphenates_isbn10_converted_to_isbn13() {
        let n = normalize("0306406152").unwrap();
        let isbn13 = to_isbn13(&n);
        assert_eq!(hyphenate_isbn13(&isbn13), "978-0-306-40615-7");
    }
}
