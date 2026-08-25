mod isbn;

use isbn::{normalize, to_isbn13, NormalizeError};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = match args.as_slice() {
        [] => None,
        [p] => Some(p.as_str()),
        _ => {
            eprintln!("usage: isbn-normalizer [FILE]");
            eprintln!("reads lines from FILE, or from stdin if FILE is omitted");
            return ExitCode::FAILURE;
        }
    };

    let reader: Box<dyn BufRead> = match path {
        None => Box::new(io::stdin().lock()),
        Some(p) => match File::open(p) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("isbn-normalizer: {p}: {e}");
                return ExitCode::FAILURE;
            }
        },
    };

    match run(reader) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("isbn-normalizer: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(mut reader: impl BufRead) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

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
        report(&mut out, trimmed)?;
    }
    out.flush()
}

fn report(out: &mut impl Write, raw: &str) -> io::Result<()> {
    match normalize(raw) {
        Ok(code) => {
            let isbn13 = to_isbn13(&code);
            writeln!(
                out,
                "{raw}\t{:?}\t{}\t{}",
                code.kind, code.digits, isbn13.digits
            )
        }
        Err(NormalizeError::Empty) => Ok(()),
        Err(e) => writeln!(out, "{raw}\tINVALID\t{e}"),
    }
}
