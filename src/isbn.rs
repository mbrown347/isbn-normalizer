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
}
