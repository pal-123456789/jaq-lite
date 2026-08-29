#!/usr/bin/env bash
#
# Prove that `cargo build --release` produces the same bytes twice -- and that
# this check would notice if it did not.
#
# Determinism comes from three keys in [profile.release]: codegen-units = 1,
# debug = 0, strip = "symbols". Deliberately not from RUSTFLAGS. A published
# hash that only reproduces when the reader exports a long environment variable
# correctly is a hash that quietly stops matching, and a bare
# `cargo build --release` has to be the command that produces it.
#
# Usage: scripts/reproducible_build.sh [CRATE_ROOT] [--bin NAME]
#
# Exits 0 only if all four assertions hold, so CI can gate on it. Exits 2 for a
# broken harness, which is a different thing from a failed assertion.

set -u

die() { printf 'FATAL: %s\n' "$*" >&2; exit 2; }

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || die "cannot locate myself"
root="$(cd "$here/.." && pwd)" || die "cannot locate the crate root"
bin=""
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) shift; [ $# -gt 0 ] || die "--bin needs a name"; bin="$1" ;;
    -*) die "unknown option: $1" ;;
    *) root="$(cd "$1" 2>/dev/null && pwd)" || die "no such crate root: $1" ;;
  esac
  shift
done

[ -f "$root/Cargo.toml" ] || die "no Cargo.toml under $root"
[ -f "$root/Cargo.lock" ] || die "no Cargo.lock under $root; it must be committed"
if [ -z "$bin" ]; then
  bin="$(sed -n 's/^name = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)"
fi
[ -n "$bin" ] || die "cannot work out the binary name; pass --bin NAME"

# Preconditions. Each of these is a way the build could embed something about
# the machine it ran on, and each is cheaper to assert than to debug later.
[ -e "$root/build.rs" ] && die "build.rs exists; nothing here accounts for what it might emit"
if grep -rn --include='*.rs' -e 'CARGO_MANIFEST_DIR' -e 'option_env!' -e 'file!()' "$root/src" >/dev/null 2>&1; then
  grep -rn --include='*.rs' -e 'CARGO_MANIFEST_DIR' -e 'option_env!' -e 'file!()' "$root/src" >&2
  die "src/ can reach a build path through the above; the binary would not be portable"
fi
for key in 'codegen-units = 1' 'debug = 0' 'strip = "symbols"'; do
  grep -qxF "$key" "$root/Cargo.toml" || die "[profile.release] is missing: $key"
done

# Every needle in the leak scan below must be non-empty. `grep -aqF -- ""`
# matches any file, so an unset HOME -- which is what a non-login shell can hand
# you -- would report a leak that is not there. A path that cannot exist keeps
# the scan at six real needles instead of five.
home="${HOME:-}"
[ -n "$home" ] || home="/nonexistent-home-$$"
who="$(id -un 2>/dev/null || true)"
[ -n "$who" ] || who="(no login name)"

# rustup installs cargo into ~/.cargo/bin, and it is a login profile that puts
# that directory on PATH. `wsl --exec bash script.sh` is neither a login nor an
# interactive shell, so the profile is never read and cargo is simply absent.
# That is exactly how the first run of this script failed. Source rustup's own
# env file rather than guessing at a layout.
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$home/.cargo/env" ]; then
    . "$home/.cargo/env"
  elif [ -x "$home/.cargo/bin/cargo" ]; then
    PATH="$home/.cargo/bin:$PATH"
    export PATH
  fi
fi
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH, and neither $home/.cargo/env nor $home/.cargo/bin/cargo exists"
command -v rustc >/dev/null 2>&1 || die "rustc is not on PATH"

# Captured into variables, asserted non-empty, then printed from those variables.
# The first run printed two blank toolchain lines and carried on to fail forty
# lines later; a hash published beside a blank toolchain is not evidence.
rustc_v="$(rustc --version)" || die "rustc --version failed"
cargo_v="$(cargo --version)" || die "cargo --version failed"
[ -n "$rustc_v" ] || die "rustc --version printed nothing"
[ -n "$cargo_v" ] || die "cargo --version printed nothing"

# Two build directories whose paths differ in LENGTH, not merely in content.
# This is the whole reason the harness is trustworthy: an earlier version of it
# used /tmp/a and /tmp/b, and its positive control matched when it should have
# differed, because a leaked path of equal length shifts nothing in the output.
base="$(mktemp -d)" || die "mktemp failed"
trap 'rm -rf "$base"' EXIT
pad="$(printf 'b%.0s' $(seq 1 35))"
dir_a="$base/a"
dir_b="$base/a$pad"
dir_c="$base/control"
delta=$(( ${#dir_b} - ${#dir_a} ))
[ "$delta" -eq 35 ] || die "build paths differ by $delta characters, not 35"

copy_crate() {
  mkdir -p "$1" || die "cannot create $1"
  cp "$root/Cargo.toml" "$root/Cargo.lock" "$1/" || die "cannot copy the manifest into $1"
  if [ -f "$root/rust-toolchain.toml" ]; then
    cp "$root/rust-toolchain.toml" "$1/" || die "cannot copy rust-toolchain.toml into $1"
  fi
  cp -R "$root/src" "$1/src" || die "cannot copy src into $1"
}

# CARGO_INCREMENTAL=0 is not optional. Incremental compilation is independently
# nondeterministic, and a DIFFER caused by it looks exactly like a real leak.
build() {
  ( cd "$1" && CARGO_INCREMENTAL=0 cargo build --release --locked --offline --quiet ) \
    || die "cargo build --release failed in $1"
  [ -f "$1/target/release/$bin" ] || die "no target/release/$bin in $1"
}

hash_of() { sha256sum "$1" | cut -d' ' -f1; }
size_of() { wc -c < "$1" | tr -d '[:space:]'; }

printf '%s reproducible build check\n' "$bin"
printf '  crate root        %s\n' "$root"
printf '  toolchain         %s\n' "$rustc_v"
printf '                    %s\n' "$cargo_v"
printf '  path A            %s (%s chars)\n' "$dir_a" "${#dir_a}"
printf '  path B            %s (%s chars)\n' "$dir_b" "${#dir_b}"
printf '  path length delta %s\n' "$delta"

copy_crate "$dir_a"; build "$dir_a"
copy_crate "$dir_b"; build "$dir_b"
bin_a="$dir_a/target/release/$bin"
bin_b="$dir_b/target/release/$bin"
hash_a="$(hash_of "$bin_a")"; size_a="$(size_of "$bin_a")"
hash_b="$(hash_of "$bin_b")"; size_b="$(size_of "$bin_b")"

# The control. Inverting debug and strip is what a machine-specific build looks
# like, so this must DIFFER. A checker that cannot fail is not a checker, and
# this is the answer to the obvious question about the two matching hashes above.
copy_crate "$dir_c"
sed -i 's/^debug = 0$/debug = 2/; s/^strip = "symbols"$/strip = "none"/' "$dir_c/Cargo.toml"
grep -qxF 'debug = 2' "$dir_c/Cargo.toml" || die "the control injection did not land"
grep -qxF 'strip = "none"' "$dir_c/Cargo.toml" || die "the control injection did not land"
build "$dir_c"
bin_c="$dir_c/target/release/$bin"
hash_c="$(hash_of "$bin_c")"; size_c="$(size_of "$bin_c")"

printf '\n  A         %s  %s bytes\n' "$hash_a" "$size_a"
printf '  B         %s  %s bytes\n' "$hash_b" "$size_b"
printf '  control   %s  %s bytes\n' "$hash_c" "$size_c"

# A leak scan that finds nothing proves nothing unless grep can find something
# that is certainly in the file. The crate name is printed by the binary itself,
# so it lives in .rodata and survives stripping.
grep -aqF -- "$bin" "$bin_a" || die "grep cannot find the crate name in the binary; the scan below would be meaningless"
leaks=0
for needle in "$dir_a" "$dir_b" "$home" "/home/" ".rustup" ".cargo"; do
  [ -n "$needle" ] || die "an empty needle would match any file; the scan would be meaningless"
  if grep -aqF -- "$needle" "$bin_a"; then
    leaks=$(( leaks + 1 ))
    printf '  LEAK      %s\n' "$needle"
  fi
done
# Reported, never fatal: a short login name collides with ordinary byte
# sequences, and $home above already covers every leak that would matter.
if grep -aqF -- "$who" "$bin_a"; then who_hit="present (not fatal)"; else who_hit="absent"; fi

verdict() { if [ "$1" = 0 ]; then printf 'PASS'; else printf 'FAIL'; fi; }
fail=0
r1=0; [ "$hash_a" = "$hash_b" ] || { r1=1; fail=1; }
r2=0; [ "$hash_c" != "$hash_a" ] || { r2=1; fail=1; }
r3=0; [ "$leaks" -eq 0 ] || { r3=1; fail=1; }
r4=0; [ "$size_a" = "$size_b" ] || { r4=1; fail=1; }

printf '\nRESULT\n'
printf '  1  two builds, unequal path lengths, same hash   %s\n' "$(verdict $r1)"
printf '  2  control with debug=2 strip=none must differ   %s\n' "$(verdict $r2)"
printf '  3  no build path, home, rustup or cargo in it    %s  (%s hits over 6 needles)\n' "$(verdict $r3)" "$leaks"
printf '  4  the two sizes are equal                       %s\n' "$(verdict $r4)"
printf '  login name in the binary                        %s\n' "$who_hit"
if [ "$fail" = 0 ]; then
  printf '\n  sha256  %s\n' "$hash_a"
  printf '  bytes   %s\n' "$size_a"
  printf '  verify  git clone, then: cargo build --release --locked --offline && sha256sum target/release/%s\n' "$bin"
  printf '\nreproducible\n'
else
  printf '\nNOT REPRODUCIBLE\n'
fi
exit "$fail"