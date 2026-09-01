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
0-306-40615-2	Isbn10	0306406152	9780306406157	978-0-306-40615-7
978 0306 40615 7	Isbn13	9780306406157	9780306406157	978-0-306-40615-7
155860832x	Isbn10	155860832X	9781558608324	978-1-55860832-4
1234567890	INVALID	checksum mismatch: expected check digit '2', found '0'
```

Output columns are tab-separated: the original input, the detected kind (or
`INVALID`), the cleaned-up code, its ISBN-13 form, and that ISBN-13 form
hyphenated into prefix/group/registrant/publisher/check-digit segments.
Invalid input is reported rather than silently dropped, so a bad row in a
large file shows up instead of disappearing.

Hyphenation is only as precise as the boundary data behind it. Registration
group 0 (English) is split all the way down to the registrant, since that
range table is small and stable. Groups 1-5 and 7 get a group-level split
with the registrant and publisher left joined, since their registrant
ranges aren't built in. Everything else — multi-digit groups, the 979
prefix — comes back as a flat, unhyphenated 13-digit string rather than a
guess.

Blank lines are skipped. Anything that isn't a digit, a hyphen, a space, or a
trailing `x`/`X` is treated as bad data and reported as such rather than
stripped out — a stray letter usually means the source field got corrupted.

For large files, either pipe from disk or pass the path directly:

```
cargo run --release < barcodes.txt > normalized.tsv
cargo run --release -- barcodes.txt > normalized.tsv
```

Passing a path avoids an extra shell redirection and gives you a clear error
if the file doesn't exist, rather than a silently empty run.

## What's checked

- **ISBN-10**: 9 digits plus a check digit in `0-9` or `X`, verified with the
  modulus-11 weighted-sum algorithm.
- **ISBN-13 / EAN-13**: 13 digits, verified with the modulus-10
  alternating-weight algorithm used by all EAN/UPC barcodes, not just books.
- **ISBN-10 -> ISBN-13 conversion**: every valid ISBN-10 is also shown in its
  978-prefixed ISBN-13 form, which is what most current systems expect.
- **Registration-group hyphenation**: the ISBN-13 form is also shown split
  into its prefix, group, registrant, and publisher segments where that
  boundary data is available (see the note above on coverage).

## Status

Early skeleton. Checksum validation, conversion, file input, and
registration-group hyphenation for the ISBN-13 form work; see the roadmap
for what's still missing (batch summaries, transposition repair).

## License

MIT, see LICENSE.
