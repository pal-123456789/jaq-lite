#!/usr/bin/env bash
#
# Run jq beside this binary and require byte-for-byte agreement.
#
# The README opens its compatibility section by saying every claim in it was
# produced by running jq against this binary rather than by reading jq's manual.
# Until this script existed that was a claim about something the author once did
# at a terminal, which is exactly the kind of claim this repository asserts
# everywhere else and had left unasserted here. Now it is a command.
#
# Four sections, in widening order:
#
#   1. the identity filter, once per real-world document
#   2. six paths a person would really type
#   3. every one of the eleven builtins, against those same documents
#   4. every builtin again, against shapes the corpus does not contain
#
# Sections 3 and 4 are what this script was missing. Fourteen comparisons proved
# that the parser and the serializer agree with jq about eight documents and six
# paths through them; not one of them called a builtin. The filters the README
# spends a page describing were compared against nothing at all, in the one file
# whose entire purpose is to compare them.
#
# Two comparisons are deliberately absent, and both are absences this repository
# states out loud rather than quietly routes around.
#
#   `length` on a number. A value that passes through one of jq's builtins is
#   re-rendered, so `1e3 | length` is `1E+3` there and `1e3` here. That is row 16
#   of CLAIMS.md, kept on purpose, and a differential that compared it would go
#   red on a divergence this project chose rather than on a defect.
#
#   `from_entries` over entries whose keys are numbers. jq stringifies such a key
#   through `tojson`; doing that here would mean owning a number formatter, which
#   is the line the builtin set is drawn along. `to_entries` on an ARRAY is
#   compared below, because both tools agree that it keys with the number `0` --
#   its result is simply never piped back into `from_entries`.
#
# Section 4 writes its inputs into the temporary directory rather than adding a
# ninth file to tests/fixtures/real_world/. That corpus is eight documents a
# machine really produced, each with its provenance recorded, and four of them
# carry the CRLF a round-trip test measures; none of that should be diluted to
# obtain an array of arrays for `flatten` to flatten. These inputs are the
# opposite kind of thing -- one line each, hand-written for one shape -- so they
# live where their purpose is legible instead of in a corpus of real documents.
#
# What the script guards about itself, because a differential that compares
# nothing at all would pass:
#
#   * Every comparison must produce output. Standard error is deliberately kept
#     out of the comparison, so two tools that both refuse an input would
#     otherwise agree on an empty file. Every filter below is one both tools
#     answer; one that stops producing output fails here rather than passing.
#   * Every builtin must appear in at least one filter, against the roster
#     spelled out below. tests/claims.rs holds that roster to the arms of
#     `Builtin::name`, so a twelfth builtin turns `cargo test` red before it can
#     be forgotten here.
#   * The total must equal EXPECTED_COMPARISONS, which row 24 of CLAIMS.md
#     quotes. A loop that silently compared nothing would otherwise pass, and
#     would go on passing after somebody moved the corpus.
#
# Exits non-zero on any disagreement, on an empty comparison, on a builtin no
# comparison reaches, on a missing jq, and on a total that is not the number this
# repository claims.
#
# Usage:  scripts/jq_differential.sh
# From:   the repository root, on a machine with jq on PATH. It builds the binary
#         it compares rather than trusting one that is already there, so cargo has
#         to be reachable as well; JAQ_LITE_BIN names a binary instead, turns the
#         build off, and is checked against the roster before anything is compared.

set -eu

# The jq the README's numbers were measured against. A different version here is
# not a failure -- it is reported, and if the comparisons still agree then the
# claim is stronger than the README states, not weaker.
readonly MEASURED_AGAINST="jq-1.8.1"
readonly EXPECTED_COMPARISONS=62

# The eleven bare-name filters this build answers to, in the order the private
# `Builtin::name` lists them. A test reads that method and requires every name in
# it to appear on this line, so the roster cannot fall behind the code; the loop
# at the end of this script requires every name on this line to appear in a
# filter, so it cannot get ahead of it either.
readonly BUILTINS="first flatten from_entries keys keys_unsorted last length \
not reverse to_entries type"

readonly root="$(cd "$(dirname "$0")/.." && pwd)"
readonly corpus="$root/tests/fixtures/real_world"

# Which binary to compare, overridable. Running this under WSL against a checkout
# that lives on a Windows drive would otherwise drop Linux artifacts into the
# same target/ directory the Windows toolchain is using, and from then on both
# toolchains rebuild the world every time they alternate. Point JAQ_LITE_BIN at a
# build made with CARGO_TARGET_DIR somewhere else and the two never meet.
readonly default_bin="$root/target/release/jaq-lite"
readonly bin="${JAQ_LITE_BIN:-$default_bin}"

cd "$root"

echo "== what is being compared =="

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is not on PATH."
  echo
  echo "This script exists to compare two implementations, so it fails rather"
  echo "than skips: a run that compared nothing and exited 0 would be worse than"
  echo "no run at all. Install jq, or read the recorded results in"
  echo "tests/fixtures/real_world/PROVENANCE.md."
  exit 1
fi

theirs_version="$(jq --version)"
echo "jq            $theirs_version"

# Build on the default path every time, and not only when nothing is there. `-x`
# cannot tell a current binary from one that predates half the language, and this
# checkout can sit on a drive both toolchains see: a Windows toolchain writes
# jaq-lite.exe here and a Linux one writes jaq-lite, so the file at this path is
# whatever the other one last built. On 2026-08-30 that was a 471824-byte ELF from
# the previous evening; `-x` trusted it, 48 of 62 comparisons disagreed, and every
# message was `cannot appear here` on a builtin this source implements. cargo is
# the only thing that knows whether there is work to do, so let it decide, and let
# it print nothing when there is not.
if [ "$bin" != "$default_bin" ]; then
  if [ ! -x "$bin" ]; then
    # An explicit override that does not exist is a mistake worth naming rather
    # than quietly papering over by building somewhere the caller did not ask for.
    echo "JAQ_LITE_BIN was set but names nothing executable."
    exit 1
  fi
  echo "override      JAQ_LITE_BIN, so this script does not build what it compares"
else
  # rustup installs cargo into ~/.cargo/bin, and it is a LOGIN profile that puts
  # that directory on PATH. `wsl --exec bash scripts/jq_differential.sh` is neither
  # a login nor an interactive shell, so it starts with no cargo at all.
  # scripts/reproducible_build.sh has resolved that for itself since the first run
  # it lost to it. This script never inherited the preamble, so the build line
  # below could not have run from WSL either -- a second defect the first one hid,
  # because an executable stale binary meant the line was never reached.
  home="${HOME:-}"
  [ -n "$home" ] || home="/nonexistent-home-$$"
  if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "$home/.cargo/env" ]; then
      . "$home/.cargo/env"
    elif [ -x "$home/.cargo/bin/cargo" ]; then
      PATH="$home/.cargo/bin:$PATH"
      export PATH
    fi
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is not on PATH, and this shell's HOME has neither .cargo/env nor"
    echo ".cargo/bin/cargo. This script builds what it compares, so it fails here"
    echo "rather than comparing whatever was left in target/release."
    exit 1
  fi
  echo "building      target/release/jaq-lite"
  cargo build --release --locked --offline >/dev/null
fi
echo "jaq-lite      $("$bin" --version)"

# Then ask that binary whether it knows the roster, before comparing anything with
# it. Forty-eight identical `cannot appear here` lines are one finding printed
# forty-eight times, and reading them as forty-eight cost most of an evening. Exit
# 3 is EXIT_FILTER -- a filter that does not compile -- and it does not depend on
# the input, so `null` is enough to ask each name whether this build has it. The
# default path was rebuilt above and will pass; an override cannot be rebuilt from
# here, and that is the case this guard is for.
unknown=""
for name in $BUILTINS; do
  set +e
  printf 'null' | "$bin" -c "$name" >/dev/null 2>&1
  asked=$?
  set -e
  if [ "$asked" -eq 3 ]; then
    unknown="$unknown $name"
  fi
done
if [ -n "$unknown" ]; then
  echo
  echo "the binary being compared does not compile these builtins:$unknown"
  echo
  echo "That is one defect and not one per comparison. Either the binary predates"
  echo "them -- check JAQ_LITE_BIN, and the mtime of the path printed above -- or"
  echo "the BUILTINS line here has got ahead of the code, which the test named"
  echo "the_differential_compares_every_builtin_this_build_has catches first."
  exit 1
fi

if [ "$theirs_version" = "$MEASURED_AGAINST" ]; then
  echo "version       the one the README's numbers were measured against"
else
  echo "version       NOT $MEASURED_AGAINST, which is what the README names."
  echo "              Agreement below still counts; a disagreement below has to"
  echo "              be read as possibly a jq version difference rather than"
  echo "              straight away as a defect here."
fi

test -d "$corpus" || { echo "no corpus at tests/fixtures/real_world"; exit 1; }

readonly work="$(mktemp -d)"
# shellcheck disable=SC2064
trap "rm -rf '$work'" EXIT

ran=0
disagreed=0
vacuous=0
filters=""

# One comparison. Both tools get the same flags, the same filter and the same
# file, and their standard output is compared byte for byte. Standard error is
# kept out of it on purpose: the two tools word their diagnostics differently by
# design, that difference is asserted in tests/query.rs against its own
# expectations, and mixing it in here would turn this into a test of prose.
#
# Because standard error is out of it, two tools that both REFUSED the input
# would agree on an empty file, and a filter that quietly stopped answering
# would go on passing forever. So an empty result on both sides is its own
# outcome, reported and counted, and it fails the run at the bottom.
compare() {
  local filter="$1" file="$2"
  local mine="$work/mine" theirs="$work/theirs"
  local shown
  shown="$(basename "$file")"

  ran=$((ran + 1))
  filters="$filters$filter
"

  "$bin" -c "$filter" "$file" >"$mine" 2>"$work/mine.err" || true
  jq -c "$filter" "$file" >"$theirs" 2>"$work/theirs.err" || true

  if [ ! -s "$mine" ] && [ ! -s "$theirs" ]; then
    vacuous=$((vacuous + 1))
    printf '  EMPTY      %-34s %s\n' "$filter" "$shown"
    if [ -s "$work/mine.err" ]; then
      echo "    jaq-lite  $(head -n 1 "$work/mine.err")"
    fi
    if [ -s "$work/theirs.err" ]; then
      echo "    jq        $(head -n 1 "$work/theirs.err")"
    fi
    return 0
  fi

  if cmp -s "$mine" "$theirs"; then
    printf '  agree      %-34s %s\n' "$filter" "$shown"
    return 0
  fi

  disagreed=$((disagreed + 1))
  printf '  DISAGREE   %-34s %s\n' "$filter" "$shown"
  echo "    jaq-lite  $(wc -c <"$mine" | tr -d ' ') bytes, jq $(wc -c <"$theirs" | tr -d ' ') bytes"
  echo "    first difference:"
  # Truncated on purpose. A whole 60 KB document printed into a CI log buries
  # the one line that matters.
  diff "$theirs" "$mine" 2>/dev/null | head -n 6 | sed 's/^/      /' || true
  if [ -s "$work/mine.err" ]; then
    echo "    jaq-lite wrote to stderr: $(head -n 1 "$work/mine.err")"
  fi
  if [ -s "$work/theirs.err" ]; then
    echo "    jq wrote to stderr: $(head -n 1 "$work/theirs.err")"
  fi
}

echo
echo "== 1. the identity filter, once per document =="
# Sorted, so the output of this script is the same on every filesystem. read_dir
# order is alphabetical on NTFS and hash order on ext4, and a differential whose
# log reorders itself between runs is a differential nobody reads twice.
for path in $(find "$corpus" -maxdepth 1 -name '*.json' | sort); do
  compare '.' "$path"
done

echo
echo "== 2. paths a person would really type =="
# Each of these is a place the two tools could reasonably disagree.
#
#   .Parent.Name                  a nested field
#   .message.spans[0].file_name   a string containing backslashes, which each
#                                 tool has to re-escape on the way out
#   .[] | .Name                   one filter, ten outputs: the stream, and
#                                 whatever separates its members
#   .[-1].Length                  a negative index, counted from the end
#   .packages[0].name             an index inside a path rather than at its end
#   .BaseUtcOffset.TotalDays      0.22916666666666666 -- seventeen significant
#                                 digits, the literal that changes if either tool
#                                 re-renders numbers through a double formatter
#                                 instead of reprinting the bytes it read
compare '.Parent.Name' "$corpus/culture.json"
compare '.message.spans[0].file_name' "$corpus/diagnostic1.json"
compare '.[] | .Name' "$corpus/srclist.json"
compare '.[-1].Length' "$corpus/srclist.json"
compare '.packages[0].name' "$corpus/metadata.json"
compare '.BaseUtcOffset.TotalDays' "$corpus/timezone.json"

echo
echo "== 3. every builtin, against the real documents =="

# length: over an object, a string, and two arrays. The string one is the
# interesting one -- jq counts code points, and so does this build, which is why
# the README says so about characters rather than about bytes.
compare 'length' "$corpus/timezone.json"
compare '.Name | length' "$corpus/culture.json"
compare '.PSCompatibleVersions | length' "$corpus/psversion.json"
compare '.message.spans | length' "$corpus/diagnostic1.json"

# keys: sorted, and sorted by code point rather than by locale. culture.json is
# the document that would expose a locale-aware collation, and the third
# comparison takes the count so that a 60 KB answer still fails legibly.
compare 'keys' "$corpus/timezone.json"
compare 'keys' "$corpus/srclist.json"
compare 'keys | length' "$corpus/culture.json"

# keys_unsorted: insertion order, which is only a claim about the parser holding
# on to the order it read.
compare 'keys_unsorted' "$corpus/psversion.json"
compare '.PSVersion | keys_unsorted' "$corpus/psversion.json"

# A postfix step applied to a builtin rather than to a path, which is the one
# piece of grammar the README describes and nothing here was comparing.
compare 'keys[0]' "$corpus/metadata.json"

# type: the word, never the value. The second and third are numbers and objects,
# the two cases where a tool that reformatted its input would show it.
compare 'type' "$corpus/srclist.json"
compare '.LCID | type' "$corpus/culture.json"
compare '.Parent | type' "$corpus/culture.json"

# not: a real boolean, then the two truthiness rules -- every object is true,
# every string is true, including the empty one (compared in section 4).
compare '.SupportsDaylightSavingTime | not' "$corpus/timezone.json"
compare '.Parent | not' "$corpus/culture.json"
compare '.reason | not' "$corpus/diagnostic3.json"

# first and last: on an array of objects, on an array of objects nested one
# level down, and piped onward so that the value and not just its shape is read.
compare 'first' "$corpus/srclist.json"
compare 'last' "$corpus/srclist.json"
compare '.PSCompatibleVersions | first' "$corpus/psversion.json"
compare '.packages | last | .name' "$corpus/metadata.json"

# reverse: on ten objects, on a short array, and on the output of another
# builtin, which is the case that needs the two filters to compose.
compare 'reverse' "$corpus/srclist.json"
compare '.PSCompatibleVersions | reverse' "$corpus/psversion.json"
compare 'keys | reverse' "$corpus/timezone.json"

# to_entries: over an object, over an object of numbers, and over an ARRAY,
# where both tools key with the number 0. That last result is deliberately not
# piped into from_entries; the header explains why.
compare 'to_entries' "$corpus/timezone.json"
compare '.PSVersion | to_entries' "$corpus/psversion.json"
compare 'to_entries' "$corpus/srclist.json"

# from_entries, reached the only way this build can reach it over a real
# document: as the far side of a round trip through to_entries.
compare 'to_entries | from_entries' "$corpus/timezone.json"

# flatten: on shapes that have nothing to flatten, which is the half of the
# claim that is easy to get wrong. Section 4 supplies the other half.
compare 'flatten' "$corpus/srclist.json"
compare '.PSCompatibleVersions | flatten' "$corpus/psversion.json"

echo
echo "== 4. shapes the corpus does not contain =="
# Written here rather than added to the corpus, for the reason the header gives.
# One line each, and each line exists for exactly one comparison below.
printf '%s\n' '["a",["b",["c",["d","e"]]],"f"]' >"$work/nest.json"
printf '%s\n' '[{"key":"one","value":"first"},{"key":"two","value":"second"}]' >"$work/entries.json"
printf '%s\n' '{"array":[],"object":{},"string":"","nothing":null}' >"$work/empties.json"
printf '%s\n' '{"zebra":"z","apple":"a","mango":"m"}' >"$work/order.json"
# Raw UTF-8, written as octal so that this script stays ASCII on disk and cannot
# be mangled by an editor or by a PowerShell here-string that decodes as ANSI.
# c3 af is U+00EF, e2 98 83 is U+2603: two code points that are two and three
# bytes wide. Neither is ever re-emitted below -- they are only counted -- because
# how each tool re-escapes non-ASCII on the way out is a separate question, and
# one the identity comparisons in section 1 already settle for real documents.
printf '%b\n' '{"text":"na\0303\0257ve \0342\0230\0203","tail":[["x"],["y","z"]]}' >"$work/wide.json"

# flatten, on the only shape that can tell full flattening from one level.
compare 'flatten' "$work/nest.json"
compare 'flatten | length' "$work/nest.json"
# first and last see past the nesting to the ends of the outer array.
compare 'first' "$work/nest.json"
compare 'last' "$work/nest.json"
# reverse must not flatten anything while it reverses.
compare 'reverse' "$work/nest.json"

# from_entries, over entries written as entries rather than derived from an
# object. Both keys are strings; the header says why they have to be.
compare 'from_entries' "$work/entries.json"
compare 'first | .key' "$work/entries.json"

# Every length that is zero, and the one type that has a length without having
# any members. `null | length` is 0 in both tools; `false | length` is an error
# in both, so it is not compared -- this script would report it EMPTY.
compare 'length' "$work/empties.json"
compare '.array | length' "$work/empties.json"
compare '.object | length' "$work/empties.json"
compare '.string | length' "$work/empties.json"
compare '.nothing | length' "$work/empties.json"
compare '.nothing | not' "$work/empties.json"

# keys against keys_unsorted, over the one document where the two answers
# differ. Nothing in the corpus has three keys in an order this wrong.
compare 'keys' "$work/order.json"
compare 'keys_unsorted' "$work/order.json"
compare 'to_entries | from_entries' "$work/order.json"

# Code points rather than bytes: the string above is 7 code points and 10 bytes,
# so a tool that counted bytes would answer 10 here.
compare '.text | length' "$work/wide.json"
compare '.tail | flatten' "$work/wide.json"
compare 'type' "$work/wide.json"

echo
echo "== result =="
echo "  comparisons   $ran"
echo "  disagreements $disagreed"
echo "  empty         $vacuous"

# Every builtin must be reachable from the filters above. Word matching, because
# `keys` is a substring of `keys_unsorted` and a roster checked with a plain
# substring search would call `keys_unsorted` covered by a filter that only ever
# says `keys`.
missing=""
for name in $BUILTINS; do
  if ! printf '%s' "$filters" | grep -qw -- "$name"; then
    missing="$missing $name"
  fi
done

echo "  builtins      $(printf '%s' "$BUILTINS" | wc -w | tr -d ' ') in the roster,\
$(if [ -z "$missing" ]; then echo " all reached"; else echo " NOT all reached"; fi)"

if [ -n "$missing" ]; then
  echo
  echo "No comparison above calls:$missing"
  echo
  echo "The roster on this script's BUILTINS line is held to the arms of the"
  echo "private Builtin::name by tests/claims.rs, so a name arriving there arrives"
  echo "here too -- and then this run stays red until a comparison exercises it."
  exit 1
fi

if [ "$vacuous" -ne 0 ]; then
  echo
  echo "$vacuous of $ran comparisons produced nothing on either side."
  echo
  echo "Standard error is kept out of the comparison on purpose, so two tools that"
  echo "both refuse an input agree on an empty file. Every filter here is one both"
  echo "tools answer, so an empty pair means a filter stopped answering rather than"
  echo "that the two agree about it."
  exit 1
fi

if [ "$ran" -ne "$EXPECTED_COMPARISONS" ]; then
  echo
  echo "This script ran $ran comparisons and this repository claims"
  echo "$EXPECTED_COMPARISONS, on the EXPECTED_COMPARISONS line above and in row 24"
  echo "of CLAIMS.md, which a test holds to this file. Either a document went"
  echo "missing from the corpus or a comparison was added without the count being"
  echo "updated. Both are worth a red run."
  exit 1
fi

if [ "$disagreed" -ne 0 ]; then
  echo
  echo "$disagreed of $ran comparisons disagree, against $theirs_version."
  exit 1
fi

echo
echo "All $ran comparisons agree byte for byte, against $theirs_version."
