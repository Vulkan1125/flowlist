#!/bin/sh
# Print one CHANGELOG section, reflowed for GitHub release bodies.
#
# GitHub turns every source newline into a hard <br>. The changelog is wrapped
# for reading in an editor, so a section pasted straight into a release renders
# as a narrow column. This script joins each bullet back into one line; GitHub
# then wraps it to the width of the release box.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
version=${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)}

section=$(
	awk -v heading="## $version" '
		index($0, heading) == 1 { inside = 1; next }
		inside && /^## / { exit }
		inside { print }
	' "$root/CHANGELOG.md"
)
if [ -z "$section" ]; then
	echo "no changelog section for $version" >&2
	exit 1
fi

printf '%s\n' "$section" | awk '
	function flush() {
		if (block != "") {
			gsub(/[[:space:]]+/, " ", block)
			sub(/^ /, "", block)
			print block
			block = ""
		}
	}
	/^[[:space:]]*$/ { flush(); print ""; next }
	/^- / || /^\* / || /^\+ / || /^#/ || /^>/ || /^```/ || /^[0-9]+\. / || /^\|/ {
		flush()
		block = $0
		next
	}
	{
		if (block == "") block = $0
		else block = block " " $0
	}
	END { flush() }
'
