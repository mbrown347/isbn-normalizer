# isbn-normalizer

ISBN and barcode data collected from spreadsheets, scanners, and old library
databases is rarely clean. You get hyphens in inconsistent places, stray
spaces, lowercase `x` for the ISBN-10 check digit, mixed ISBN-10/ISBN-13, and
plain typos that break the checksum. This is a small command-line tool that
takes that mess, strips it down to digits, verifies the checksum, and reports
what it found — one line in, one line out.

It reads its input a line at a time instead of loading the whole file, so it
can be pointed at a barcode export with millions of rows without blowing up
memory.

## Usage

Build and run with cargo (no external crates required):

```
cargo run --release
```

Feed it lines of ISBNs, one per line, on stdin:

```
$ printf '0-306-40615-2\n978 0306 40615 7\n155860832x\n1234567890\n' | cargo run --release
0-306-40615-2	Isbn10	0306406152	9780306406157
978 0306 40615 7	Isbn13	9780306406157	9780306406157
155860832x	Isbn10	155860832X	9781558608324
1234567890	INVALID	checksum mismatch: expected check digit '2', found '0'
```

Output columns are tab-separated: the original input, the detected kind (or
`INVALID`), the cleaned-up code, and its ISBN-13 form. Invalid input is
reported rather than silently dropped, so a bad row in a large file shows up
instead of disappearing.

Blank lines are skipped. Anything that isn't a digit, a hyphen, a space, or a
trailing `x`/`X` is treated as bad data and reported as such rather than
stripped out — a stray letter usually means the source field got corrupted.

For large files, pipe from disk instead of typing input by hand:

```
cargo run --release < barcodes.txt > normalized.tsv
```

## What's checked

- **ISBN-10**: 9 digits plus a check digit in `0-9` or `X`, verified with the
  modulus-11 weighted-sum algorithm.
- **ISBN-13 / EAN-13**: 13 digits, verified with the modulus-10
  alternating-weight algorithm used by all EAN/UPC barcodes, not just books.
- **ISBN-10 -> ISBN-13 conversion**: every valid ISBN-10 is also shown in its
  978-prefixed ISBN-13 form, which is what most current systems expect.

## Status

Early skeleton. Checksum validation and conversion work; see the roadmap for
what's still missing (hyphenation by registration group, file arguments,
batch summaries).

## License

MIT, see LICENSE.
