mod isbn;

use isbn::{hyphenate_isbn13, normalize, to_isbn13, CodeKind, NormalizeError};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut path = None;
    let mut summary = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--summary" => summary = true,
            _ if path.is_none() => path = Some(arg),
            _ => {
                eprintln!("usage: isbn-normalizer [--summary] [FILE]");
                eprintln!("reads lines from FILE, or from stdin if FILE is omitted");
                return ExitCode::FAILURE;
            }
        }
    }

    let reader: Box<dyn BufRead> = match &path {
        None => Box::new(io::stdin().lock()),
        Some(p) => match File::open(p) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("isbn-normalizer: {p}: {e}");
                return ExitCode::FAILURE;
            }
        },
    };

    match run(reader, summary) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("isbn-normalizer: {e}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Default)]
struct Counts {
    isbn10: usize,
    isbn13: usize,
    invalid: usize,
}

impl Counts {
    fn total(&self) -> usize {
        self.isbn10 + self.isbn13 + self.invalid
    }
}

impl std::fmt::Display for Counts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} lines: {} valid ISBN-10, {} valid ISBN-13, {} invalid",
            self.total(),
            self.isbn10,
            self.isbn13,
            self.invalid
        )
    }
}

fn run(mut reader: impl BufRead, summary: bool) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut counts = Counts::default();

    // Read one line at a time and reuse the buffer instead of collecting the
    // whole input into a String or Vec first. A barcode export can run to
    // millions of rows; memory use here stays flat regardless of file size.
    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.trim().is_empty() {
            continue;
        }
        report(&mut out, trimmed, &mut counts)?;
    }
    out.flush()?;

    if summary {
        eprintln!("{counts}");
    }
    Ok(())
}

fn report(out: &mut impl Write, raw: &str, counts: &mut Counts) -> io::Result<()> {
    match normalize(raw) {
        Ok(code) => {
            match code.kind {
                CodeKind::Isbn10 => counts.isbn10 += 1,
                CodeKind::Isbn13 => counts.isbn13 += 1,
            }
            let isbn13 = to_isbn13(&code);
            let hyphenated = hyphenate_isbn13(&isbn13);
            writeln!(
                out,
                "{raw}\t{:?}\t{}\t{}\t{}",
                code.kind, code.digits, isbn13.digits, hyphenated
            )
        }
        Err(NormalizeError::Empty) => Ok(()),
        Err(e) => {
            counts.invalid += 1;
            writeln!(out, "{raw}\tINVALID\t{e}")
        }
    }
}
