#!/bin/sh
# Generate a large library and time the shared query engine through the CLI.
#
# Usage: scripts/perf-check.sh [task-count]
#   BIN=path/to/flowlist    binary to test (default: target/release/flowlist)
#   KEEP=1                  keep the generated library and print its path
#
# The GUI renders the same `query::view` results as `flowlist list`, so these
# numbers cover the domain layer. Timings are wall-clock and include process
# start-up, which is the honest cost a CLI user pays.

set -eu

count=${1:-10000}
bin=${BIN:-target/release/flowlist}
if [ ! -x "$bin" ]; then
	echo "missing $bin; run: cargo build --release" >&2
	exit 1
fi

root=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-perf.XXXXXX")
seed="$root/seed.taskpaper"
trap '[ "${KEEP:-0}" = "1" ] || rm -rf "$root"' EXIT

python3 - "$seed" "$count" <<'PY'
import sys

path, count = sys.argv[1], int(sys.argv[2])
per_project = 20
projects = max(1, count // per_project)
with open(path, "w") as out:
    out.write("# Flowlist TaskPaper\n\n")
    for p in range(projects):
        out.write(f"# Project: Area {p // 25:02d}/Project {p:04d}\n")
        for t in range(per_project):
            due = f" @due(2026-12-{(t % 28) + 1:02d})" if t % 3 == 0 else ""
            flag = " @flag" if t % 17 == 0 else ""
            est = " @estimate(45m)" if t % 4 == 0 else ""
            tag = f" @tags(Context {t % 8:02d})" if t % 5 == 0 else ""
            out.write(
                f"\t- Task {p:04d}-{t:02d} with a reasonably descriptive title"
                f"{due}{flag}{est}{tag}\n"
            )
        out.write("\n")
PY

echo "library: $root ($count tasks)"
FLOWLIST_HOME="$root" python3 - "$bin" "$root" <<'PY'
import os
import subprocess
import sys
import time

bin, root = sys.argv[1], sys.argv[2]
commands = [
    ("import", os.path.join(root, "seed.taskpaper"), "--format", "taskpaper", "--json"),
    ("list", "--json"),
    ("list", "--group", "project", "--sort", "due", "--json"),
    ("list", "--forecast", "--json"),
    ("list", "--flagged", "--json"),
    ("stats",),
]
env = dict(os.environ)
for args in commands:
    started = time.perf_counter()
    result = subprocess.run(
        [bin, *args], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE
    )
    elapsed = (time.perf_counter() - started) * 1000
    label = " ".join(args[:3])
    print(f"  {label:<42} {elapsed:8.1f} ms  exit {result.returncode}")
    if result.returncode != 0:
        sys.stderr.write(result.stderr.decode())
        sys.exit(1)
PY

if [ "${KEEP:-0}" = "1" ]; then
	echo "kept: $root"
fi
