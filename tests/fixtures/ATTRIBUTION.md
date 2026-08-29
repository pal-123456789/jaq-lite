# Attribution for the vendored conformance corpus

The `.json` files in `test_parsing/` and `test_transform/` are not the work of
this project. They are vendored from JSONTestSuite and are reproduced here under
the terms of their own license.

## Upstream

| | |
|---|---|
| Project | JSONTestSuite |
| Author | Nicolas Seriot |
| Repository | https://github.com/nst/JSONTestSuite |
| Commit | `1ef36fa01286573e846ac449e8683f8833c5b26a` |
| Commit date | 2024-11-22 |
| License | MIT, Copyright (c) 2016 Nicolas Seriot |

The upstream license is reproduced verbatim, byte for byte, in
`LICENSE-JSONTestSuite` in this directory. MIT requires the copyright notice and
the permission notice to travel with any substantial portion of the material, so
that file is part of the obligation and not a courtesy.

## What was vendored

| Directory | Files |
|---|---|
| `test_parsing/` | 318 (95 named `y_`, 188 named `n_`, 35 named `i_`) |
| `test_transform/` | 22 |
| Total | 340 files, 354380 bytes |

Nothing was modified. No file was reformatted, re-encoded, renamed or removed,
and no file was added. The naming convention is upstream's: `y_` must be
accepted, `n_` must be rejected, and `i_` is implementation-defined, where a
parser may do either so long as it does so deliberately.

## Integrity

`FIXTURES_MANIFEST.sha256` lists a SHA-256 for each of the 340 files, in the
standard `sha256sum` format, sorted by relative path using ordinal byte
comparison, with LF line endings and no header. From this directory:

    sha256sum -c FIXTURES_MANIFEST.sha256

The manifest covers the 340 data files only. It does not cover itself, this
file, or the license.

The aggregate digest of the corpus is the SHA-256 of the manifest file:

    baf040b307bbf57479003dd1fd6cf1bca6c5dc7ada8e50ef1bc21dce1d15bf9a

That value was computed from a git-free snapshot taken before this repository
existed, and recomputed after vendoring, independently, by the same recipe. The
two agree, which is what establishes that the copy into this repository changed
no bytes.

## Byte fidelity is not a detail here

Some fixtures are deliberately not valid UTF-8, and some contain bare control
bytes inside string literals. Those files exist precisely to be malformed, so
any text-mode handling that normalised line endings or re-encoded them would
quietly change what the test tests, and a parser could then pass by accident.

`.gitattributes` therefore marks `tests/fixtures/**` as `-text`, which makes git
store and check out these paths byte for byte with no end-of-line conversion in
either direction. That attribute was committed before the corpus was added, in
commit 2, rather than afterwards -- the order matters, because git applies
end-of-line conversion at the moment a file is staged.

## This is data, not a dependency

To be unambiguous, since this project's entire claim is an empty dependency
graph:

- These files are never compiled into the binary. No `include_str!`, no
  `include_bytes!`, no build script reads this directory.
- They are read from disk at test time only, by integration tests under
  `tests/`, which are not part of the published artifact.
- They appear nowhere in `Cargo.toml` or `Cargo.lock`.
- They are marked `linguist-vendored` so they are excluded from the
  repository's language statistics.

`cargo build --release` produces a working binary with this directory deleted.
The only thing that breaks without it is `cargo test`.

This disclosure is repeated in `STDLIB.md` and the README, because a reader
checking the zero-dependency claim should not have to find this file to learn
that third-party material is present.