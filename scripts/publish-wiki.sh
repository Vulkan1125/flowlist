#!/bin/sh
# Publish the user-facing wiki: the GUI, CLI and MCP guides, with screenshots.
#
# GitHub creates a wiki repository only after the first page is saved in the
# web UI. Until then `git ls-remote` reports "Repository not found" and this
# script stops with instructions instead of failing halfway through.
#
# The repository's docs/ pages stay the reference for development; the wiki is
# deliberately just the manual, so the architecture, backlog and packaging
# pages are removed from it.
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
wiki=$tmp/wiki

# Drop the old developer-facing pages and replace them with the manual.
for page in GUI CLI MCP Features Accessibility Architecture Testing-the-GUI Packaging Backlog; do
	rm -f "$wiki/$page.md"
done
rm -rf "$wiki/images"

for page in Home Using-the-GUI Using-the-CLI Using-the-Phone Using-the-MCP _Sidebar; do
	cp "$root/docs/wiki/$page.md" "$wiki/$page.md"
done
cp -R "$root/docs/wiki/images" "$wiki/images"

git -C "$wiki" add -A
if git -C "$wiki" diff --cached --quiet; then
	echo "Wiki is already up to date."
	exit 0
fi
git -C "$wiki" commit -q -m "Publish the Flowlist manual"
GIT_TERMINAL_PROMPT=0 git -C "$wiki" push -q
echo "Published $wiki_url"
