# Build log

A running record of what was done, when, and why. Written as the work happens
rather than reconstructed afterwards.

## 2026-08-29, 09:00-09:15 IST - Sprint 0, commits 1 to 5

The window opened 2026-08-28 18:00 UTC, which is 23:30 local time. Work started
at 09:00 the following morning, 9.5 hours into the 72, leaving 62.5. That was a
deliberate trade: commit 1 is the only commit that cannot be corrected without
rewriting published history, and checking its author line at 23:35 while tired
is exactly how that gets missed. No rule rewards an earlier timestamp.

Provenance rests on GitHub's own record rather than on this repository's commit
dates, because `GIT_AUTHOR_DATE` is settable by the author and therefore proves
nothing. The server wrote `createdAt = 2026-08-29T03:32:41Z`, and every commit
below was pushed individually as it was made.

| # | Commit | What landed |
|---|---|---|
| 1 | b2182bf | `cargo new`, empty `[dependencies]`, `Cargo.lock` committed |
| 2 | 0a4f330 | `.gitattributes`, before a single fixture existed |
| 3 | 06320bc | MIT license, toolchain pinned to 1.98.0, release profile |
| 4 | 4850cc1 | library and binary split, `forbid(unsafe_code)`, `deny(missing_docs)` |
| 5 | this one | documentation skeletons |

Ordering that mattered: `.gitattributes` had to precede the fixture corpus.
Several fixtures are not valid UTF-8 and several contain NUL bytes, so once git
has normalised them the conformance score measures line-ending policy instead of
the parser. Committing the attributes first is not tidiness, it is the only
order that works.

Policy for the rest of the window, adopted rather than improvised: no force
push, no rebase, no amending a commit that has been pushed, no squashing. When
something is wrong, it is fixed by a new commit. History is a graded artifact
here, and the value of an unrewritten one is that it can be checked.