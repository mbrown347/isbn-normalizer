# isbn-normalizer

ISBN and barcode data collected from spreadsheets, scanners, and old library
databases is rarely clean. You get hyphens in inconsistent places, stray
spaces, lowercase `x` for the ISBN-10 check digit, mixed ISBN-10/ISBN-13, and
plain typos that break the checksum. This is a small command-line tool that
takes that mess, strips it down to digits, verifies the checksum, and reports
what it found — one line in, one line out. Alongside ISBN/EAN it also
recognizes UPC-A (retail barcodes) and ISSN (serials) by their length, since
they're the same kind of "digits plus a check digit" input and show up in the
same messy exports.

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

Output columns are tab-separated. For ISBN-10/ISBN-13 input that's the
original input, the detected kind (or `INVALID`), the cleaned-up code, its
ISBN-13 form, and that ISBN-13 form hyphenated into
prefix/group/registrant/publisher/check-digit segments. UPC-A and ISSN have
no ISBN-13 equivalent, so those rows only get the original input, the
detected kind, and the cleaned-up code:

```
$ printf '036000291452\n1234-5679\n' | cargo run --release
036000291452	UpcA	036000291452
1234-5679	Issn	12345679
```

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

Add `--summary` to get a count of valid ISBN-10, valid ISBN-13, valid UPC-A,
valid ISSN, and invalid lines printed to stderr after the run, which is handy
for checking a batch without scrolling through every row:

```
$ printf '0306406152\n1234567890\n' | cargo run --release -- --summary
0306406152	Isbn10	0306406152	9780306406157	978-0-306-40615-7
1234567890	INVALID	checksum mismatch: expected check digit '2', found '0'
2 lines: 1 valid ISBN-10, 0 valid ISBN-13, 0 valid UPC-A, 0 valid ISSN, 1 invalid
```

The flag and the file path can be given in either order.

When a checksum fails, the tool also checks whether swapping two adjacent
digits would have made it valid — the classic typo of entering "0360..."
instead of "0306...". If exactly one such swap fixes it, the corrected code
is appended as a fifth column:

```
$ printf '5657585951\n' | cargo run --release
5657585951	INVALID	checksum mismatch: expected check digit 'X', found '1'	possible transposition -> 5655785951
```

If no single swap fixes it, or more than one swap would (so there's no way
to tell which correction is right), nothing is appended and the row is
reported as plain invalid input.

## What's checked

- **ISBN-10**: 9 digits plus a check digit in `0-9` or `X`, verified with the
  modulus-11 weighted-sum algorithm.
- **ISBN-13 / EAN-13**: 13 digits, verified with the modulus-10
  alternating-weight algorithm used by all EAN/UPC barcodes, not just books.
- **UPC-A**: 12 digits, verified with the same modulus-10 scheme as EAN-13
  but with the odd/even weights swapped (a UPC-A is an EAN-13 with the
  leading zero dropped).
- **ISSN**: 8 digits, verified with the same modulus-11 scheme as ISBN-10
  but over 7 data digits instead of 9.
- **ISBN-10 -> ISBN-13 conversion**: every valid ISBN-10 is also shown in its
  978-prefixed ISBN-13 form, which is what most current systems expect.
- **Registration-group hyphenation**: the ISBN-13 form is also shown split
  into its prefix, group, registrant, and publisher segments where that
  boundary data is available (see the note above on coverage).
- **Single-transposition repair**: a checksum failure caused by one adjacent
  digit swap is detected and the corrected code is suggested, as long as the
  fix is unambiguous.

## Status

Early skeleton. Checksum validation and normalization for ISBN-10, ISBN-13,
UPC-A, and ISSN, ISBN-10 -> ISBN-13 conversion, file input,
registration-group hyphenation for the ISBN-13 form, batch summaries, and
single-transposition repair all work. A separate library crate is still on
the roadmap.

## License

MIT, see LICENSE.
