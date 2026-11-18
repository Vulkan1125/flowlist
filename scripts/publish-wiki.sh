#!/bin/sh
# Publish the repository docs to the GitHub wiki.
#
# GitHub creates a wiki repository only after the first page is saved in the
# web UI. Until then `git ls-remote` reports "Repository not found" and this
# script stops with instructions instead of failing halfway through.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
wiki_url=${WIKI_URL:-https://github.com/Vulkan1125/flowlist.wiki.git}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-wiki.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

if ! GIT_TERMINAL_PROMPT=0 git ls-remote "$wiki_url" >/dev/null 2>&1; then
	cat >&2 <<EOF
The wiki repository is not available yet:
  $wiki_url

Open the repository on GitHub, choose Wiki, and create the first page.
Then rerun this script to publish the full documentation tree.
EOF
	exit 1
fi

GIT_TERMINAL_PROMPT=0 git clone --depth 1 "$wiki_url" "$tmp/wiki" >/dev/null
cp "$root/docs/wiki/Home.md" "$tmp/wiki/Home.md"
cp "$root/docs/gui.md" "$tmp/wiki/GUI.md"
cp "$root/docs/cli.md" "$tmp/wiki/CLI.md"
cp "$root/docs/mcp.md" "$tmp/wiki/MCP.md"
cp "$root/docs/features.md" "$tmp/wiki/Features.md"
cp "$root/docs/accessibility.md" "$tmp/wiki/Accessibility.md"
cp "$root/docs/architecture.md" "$tmp/wiki/Architecture.md"
cp "$root/docs/gui-automation.md" "$tmp/wiki/Testing-the-GUI.md"
cp "$root/docs/packaging.md" "$tmp/wiki/Packaging.md"
cp "$root/docs/backlog.md" "$tmp/wiki/Backlog.md"

git -C "$tmp/wiki" add -A
if git -C "$tmp/wiki" diff --cached --quiet; then
	echo "Wiki is already up to date."
	exit 0
fi
git -C "$tmp/wiki" commit -q -m "Publish Flowlist documentation"
GIT_TERMINAL_PROMPT=0 git -C "$tmp/wiki" push -q
echo "Published $wiki_url"
