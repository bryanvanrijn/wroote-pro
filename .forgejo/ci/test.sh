#!/usr/bin/env bash
# Wroote Pro checks, run inside rust:1.87-bookworm from the repository root (see .forgejo/workflows/ci.yml).
# Mirrors what upstream CI does for the parts we change: rustfmt (pinned nightly), clippy -D warnings,
# cargo test, benchmark build, ruff, mypy, and trial (our own tests plus a regression set).
#
# Network during the run, per the egress policy of the CI runner:
#   deb.debian.org over https (signed Release files), static.rust-lang.org (rustup verifies sha256),
#   crates.io (Cargo.lock checksums), PyPI (poetry.lock hashes; poetry itself via --require-hashes).
# Exact versions of what was fetched are printed in the log. A prebuilt CI image is the follow-up.
set -uo pipefail
export CARGO_TERM_COLOR=never PIP_DISABLE_PIP_VERSION_CHECK=1 POETRY_VIRTUALENVS_IN_PROJECT=true
fail=0
step() { echo; echo "===== $1 ($(date -u +%H:%M:%S))"; }
result() { if [ "$1" -eq 0 ]; then echo "RESULT $2: OK"; else echo "RESULT $2: FAILED ($1)"; fail=1; fi; }

step "system"
sed -i 's#http://deb.debian.org#https://deb.debian.org#g' /etc/apt/sources.list.d/debian.sources
apt-get update -qq >/dev/null && apt-get install -y -qq --no-install-recommends python3-dev python3-venv libpq-dev >/dev/null
result $? apt-packages
echo "apt sources: $(grep -h '^URIs:' /etc/apt/sources.list.d/*.sources | tr '\n' ' ')"
echo "apt versions: $(dpkg-query -W -f='${Package}=${Version} ' python3-dev python3-venv libpq-dev)"
rustc --version; python3 --version; nproc; awk '/MemTotal/{print "memory (host view): "$2" kB"}' /proc/meminfo

step "rustfmt (nightly-2025-03-27, as upstream)"
rustup toolchain install nightly-2025-03-27 --profile minimal --component rustfmt >/dev/null 2>&1
echo "rustfmt: $(rustfmt +nightly-2025-03-27 --version)"
cargo +nightly-2025-03-27 fmt --check; result $? rustfmt

step "clippy -D warnings"
rustup component add clippy >/dev/null 2>&1
echo "clippy: $(cargo clippy --version)"
cargo clippy -q -- -D warnings 2>&1 | tail -40; result "${PIPESTATUS[0]}" clippy

step "cargo test"
cargo test -q 2>&1 | grep -E "^test result|FAILED|panicked|error(\[|:)" | head -40; result "${PIPESTATUS[0]}" cargo-test

step "cargo bench --no-run (build the benchmarks, nightly as upstream)"
cargo +nightly-2025-03-27 bench --no-run -q 2>&1 | grep -E "^error" | head -20; result "${PIPESTATUS[0]}" cargo-bench-build

step "poetry install (extras all, builds the Rust extension)"
python3 -m venv /tools && /tools/bin/pip install -q --require-hashes -r .forgejo/ci/poetry-requirements.txt 2>&1 | tail -3
result "${PIPESTATUS[0]}" poetry-bootstrap
export PATH="/tools/bin:$PATH"
poetry --version
poetry install --extras all --no-interaction -q 2>&1 | tail -15; result "${PIPESTATUS[0]}" poetry-install

step "ruff check + format"
poetry run ruff check --output-format=concise . 2>&1 | tail -20; r1=${PIPESTATUS[0]}
poetry run ruff format --check . 2>&1 | tail -10; r2=${PIPESTATUS[0]}
result $((r1 + r2)) ruff

step "mypy (full, as upstream)"
timeout 1800 poetry run mypy 2>&1 | tail -25; result "${PIPESTATUS[0]}" mypy

step "trial: our own tests (tests/**/test_wroote*.py)"
own=$(find tests -name 'test_wroote*.py' | sort | sed 's#\.py$##; s#/#.#g' | tr '\n' ' ')
if [ -n "$own" ]; then
  echo "modules: $own"
  # shellcheck disable=SC2086
  poetry run trial $own 2>&1 | tail -25; result "${PIPESTATUS[0]}" trial-own
else
  echo "no own test modules on this commit"
fi

step "trial: regression set (push, push rules API, config, module API, appservice handler)"
poetry run trial -j4 tests.push tests.rest.client.test_push_rule_attrs tests.config tests.module_api \
  tests.handlers.test_appservice 2>&1 | tail -15; result "${PIPESTATUS[0]}" trial-regression

step "end"
echo "TOTAL: $([ $fail -eq 0 ] && echo GREEN || echo RED)"
exit $fail
