# Where these documents came from

`test_parsing/` beside this directory is JSONTestSuite: files chosen to break
parsers, a quarter of which are not valid UTF-8 and two of which are a hundred
thousand levels deep. Passing it is necessary and it is not sufficient, because
no tool emits anything like it. Nothing in that corpus tells you whether this
parser handles the JSON that comes out of a build system.

This directory is the other half of that question. Every document in it was
produced by a tool that was already on the machine, on 2026-08-30, inside the
hackathon window, and kept byte for byte.

## The documents and the commands that wrote them

| file | producer | command |
|---|---|---|
| `metadata.json` | cargo 1.98.0 | `cargo metadata --format-version 1` |
| `diagnostic1.json` | rustc 1.98.0 | `cargo build --message-format=json` on a scratch crate with two deliberate errors; the three longest message lines were kept, one per file |
| `diagnostic2.json` | rustc 1.98.0 | the same run |
| `diagnostic3.json` | rustc 1.98.0 | the same run |
| `culture.json` | PowerShell 5.1 | `Get-Culture \| ConvertTo-Json -Depth 4` |
| `psversion.json` | PowerShell 5.1 | `$PSVersionTable \| ConvertTo-Json -Depth 6` |
| `timezone.json` | PowerShell 5.1 | `Get-TimeZone \| ConvertTo-Json -Depth 3` |
| `srclist.json` | PowerShell 5.1 | `Get-ChildItem src -File \| Select-Object Name, Length, LastWriteTimeUtc \| ConvertTo-Json -Depth 3` |

Three producers rather than one, because a single producer would only prove this
parser handles that producer. Each of these has habits the others do not:

- cargo and rustc write a whole document on one line with no whitespace in it
  and escape nothing they do not have to, which is what makes a byte-for-byte
  reprint a testable claim rather than an aspiration.
- rustc's diagnostics are the gnarliest of the three: spans nested inside spans,
  a good number of `null` fields, and strings containing newlines.
- PowerShell indents four spaces, ends its lines with CRLF, and escapes
  characters that never needed escaping -- the apostrophe as an escape spelled
  backslash, `u`, then `0027`, and the forward slash with a backslash in front
  of it -- which turns the round trip into a real question about strings.

## What was substituted

These came off a personal machine, so the machine had to come out of them before
anything was kept. Replaced everywhere, in both the raw and the JSON-escaped
spelling of each, and in the forward-slash spelling too:

| what | with |
|---|---|
| the user's profile directory | `X:\home` |
| the user name | `USER` |
| the machine name | `HOST` |
| the repository directory | `X:\repo` |
| the working directory beside it | `X:\prep` |
| the scratch crate's directory | `X:\scratch` |

The collector then re-read every document and refused to keep any of them if the
user name, the machine name, or any recognisable home-directory prefix had
survived. It did refuse once, on `metadata.json`: one of the needles was a
substring of this crate's own description, which is a false positive rather than
a leak, and the needle was narrowed to the three concrete path forms instead of
being deleted.

Nothing else was changed. No reformatting, no re-indenting, no line-ending
conversion, no key reordering. The CRLFs in the four PowerShell documents are
the ones PowerShell wrote.

## Why these are committed rather than generated

Re-running the commands above on another machine produces different bytes:
different crate versions, a different culture, a different time zone, a
different `src` listing. So the documents are the artifact and the commands are
provenance rather than a build step. `.gitattributes` marks this directory
`-text`, so git stores and checks these paths out byte for byte and the CRLFs
survive a clone on Linux.

These files are not vendored third-party material and no attribution is owed for
them; see the closing section of `../ATTRIBUTION.md`. They are read from disk at
test time only, are never compiled into the binary, and appear nowhere in
`Cargo.toml` or `Cargo.lock`.

## What is asserted about them

By `tests/real_world.rs`:

- Every document parses, reprints compact, reparses, and reprints to the same
  bytes the first reprint produced.
- `metadata.json` and the three diagnostics arrived from their producer with no
  whitespace in them, so for those four the compact reprint has to equal the
  input exactly -- the same bytes cargo and rustc wrote.
- `culture.json` carries 104 escaped apostrophes and 48 escaped solidi going in
  and none of either coming out, while its 31 raw non-ASCII bytes are still 31
  raw non-ASCII bytes on the way out. Numbers keep their spelling; strings keep
  their meaning. That asymmetry is deliberate, it is jq's, and it is the one
  correction this corpus produced.

To see what it measured:

    cargo test --test real_world -- --nocapture --test-threads=1

## What it did not find

No behavioural defect. All eight documents round-tripped on the first attempt,
and `jq-1.8.1` was run beside this binary over the same documents and agreed
byte for byte on every comparison. That is a result worth recording rather than
a section worth omitting: the prediction going in was that the escape handling
would be where the two tools diverged, and it is not. What the corpus did find
was a sentence in the README that overstated the rule for numbers by leaving the
rule for strings unstated.
