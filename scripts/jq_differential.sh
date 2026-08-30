#!/usr/bin/env bash
#
# Run jq beside this binary over the real-world corpus and require byte-for-byte
# agreement.
#
# The README opens its compatibility section by saying every claim in it was
# produced by running jq against this binary rather than by reading jq's manual.
# Until this script existed that was a claim about something the author once did
# at a terminal, which is exactly the kind of claim this repository asserts
# everywhere else and had left unasserted here. Now it is a command.
#
# Fourteen comparisons: the identity filter on each of the eight documents, and
# six paths chosen because each one is a place the two tools could reasonably
# disagree.
#
#   .Parent.Name                  a nested field
#   .message.spans[0].file_name   a string containing backslashes, which each
#                                 tool has to re-escape on the way out
#   .[] | .Name                   one filter, ten outputs: the stream, and
#                                 whatever separates its members
#   .[-1].Length                  a negative index, counted from the end
#   .packages[0].name             an index inside a path rather than at its end
#   .BaseUtcOffset.TotalDays      0.22916666666666666 -- seventeen significant
#                                 digits, which is the literal that changes if
#                                 either tool re-renders numbers through a
#                                 double formatter instead of reprinting the
#                                 bytes it read
#
# Exits non-zero on any disagreement, on a missing jq, and on a comparison count
# that is not the number this repository claims. That last one matters: a loop
# that silently compared nothing would pass, and would go on passing after
# somebody moved the corpus.
#
# Usage:  scripts/jq_differential.sh
# From:   the repository root, on a machine with jq on PATH.

set -eu

# The jq the README's numbers were measured against. A different version here is
# not a failure -- it is reported, and if the comparisons still agree then the
# claim is stronger than the README states, not weaker.
readonly MEASURED_AGAINST="jq-1.8.1"
readonly EXPECTED_COMPARISONS=14

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

if [ ! -x "$bin" ]; then
  if [ "$bin" != "$default_bin" ]; then
    # An explicit override that does not exist is a mistake worth naming rather
    # than quietly papering over by building somewhere the caller did not ask for.
    echo "JAQ_LITE_BIN was set but names nothing executable."
    exit 1
  fi
  echo "building      target/release/jaq-lite"
  cargo build --release --locked --offline >/dev/null
fi
echo "jaq-lite      $("$bin" --version)"

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

# One comparison. Both tools get the same flags, the same filter and the same
# file, and their standard output is compared byte for byte. Standard error is
# kept out of it on purpose: the two tools word their diagnostics differently by
# design, that difference is asserted in tests/query.rs against its own
# expectations, and mixing it in here would turn this into a test of prose.
compare() {
  local filter="$1" file="$2"
  local mine="$work/mine" theirs="$work/theirs"

  ran=$((ran + 1))

  "$bin" -c "$filter" "$corpus/$file" >"$mine" 2>"$work/mine.err" || true
  jq -c "$filter" "$corpus/$file" >"$theirs" 2>"$work/theirs.err" || true

  if cmp -s "$mine" "$theirs"; then
    printf '  agree      %-32s %s\n' "$filter" "$file"
    return 0
  fi

  disagreed=$((disagreed + 1))
  printf '  DISAGREE   %-32s %s\n' "$filter" "$file"
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
echo "== the identity filter, once per document =="
# Sorted, so the output of this script is the same on every filesystem. read_dir
# order is alphabetical on NTFS and hash order on ext4, and a differential whose
# log reorders itself between runs is a differential nobody reads twice.
for path in $(find "$corpus" -maxdepth 1 -name '*.json' | sort); do
  compare '.' "$(basename "$path")"
done

echo
echo "== paths a person would really type =="
compare '.Parent.Name' culture.json
compare '.message.spans[0].file_name' diagnostic1.json
compare '.[] | .Name' srclist.json
compare '.[-1].Length' srclist.json
compare '.packages[0].name' metadata.json
compare '.BaseUtcOffset.TotalDays' timezone.json

echo
echo "== result =="
echo "  comparisons   $ran"
echo "  disagreements $disagreed"

if [ "$ran" -ne "$EXPECTED_COMPARISONS" ]; then
  echo
  echo "This script ran $ran comparisons and this repository claims"
  echo "$EXPECTED_COMPARISONS, in README.md, CLAIMS.md and PROVENANCE.md. Either a"
  echo "document went missing from the corpus or a comparison was added without"
  echo "the count being updated. Both are worth a red run."
  exit 1
fi

if [ "$disagreed" -ne 0 ]; then
  echo
  echo "$disagreed of $ran comparisons disagree, against $theirs_version."
  exit 1
fi

echo
echo "All $ran comparisons agree byte for byte, against $theirs_version."
