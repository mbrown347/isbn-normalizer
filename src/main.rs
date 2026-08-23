mod isbn;

use isbn::{normalize, to_isbn13, NormalizeError};
use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
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
