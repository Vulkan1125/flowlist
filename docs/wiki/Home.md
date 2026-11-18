# Flowlist wiki

Flowlist is a local-first task manager written in Rust: a native GPUI window, a
scriptable CLI and a Model Context Protocol server over one shared JSON
library. Nothing leaves your machine.

## Start here

- **New to Flowlist?** Read the [GUI guide](GUI) for a tour of the window, then
  the [CLI reference](CLI) when you want to script it.
- **Connecting an assistant?** The [MCP tools](MCP) page lists every tool and
  its arguments.
- **Looking for a specific behaviour?** The [feature checklist](Features) is
  the exhaustive inventory.
- **Building or testing?** [Architecture](Architecture) and
  [Testing the GUI](Testing-the-GUI) cover the internals.

## The four surfaces

| Surface | Entry point | Shared behaviour |
| --- | --- | --- |
| Desktop window | `flowlist gui` or `Flowlist.app` | undo, validation, repeat rules |
| CLI | `flowlist <command>` | same command engine and undo history |
| MCP server | `flowlist mcp` | same tools over JSON-RPC stdio |
| Library file | `data.json` | atomic writes, backups, cross-process lock |

Every mutation goes through `flowlist-core`, so the GUI, CLI and MCP server
cannot drift apart. `flowlist undo` can undo a sidebar drag; `⌘Z` can undo a
tool call.

## Quick recipes

```sh
flowlist add 'Draft launch email #Work/Website ::Work/Launch due:tomorrow'
flowlist list --today --group project
flowlist done <id>
flowlist trash --restore <id>
FLOWLIST_HOME="$HOME/Library/Application Support/Flowlist" flowlist mcp
```

## Pages

- [GUI guide](GUI)
- [CLI reference](CLI)
- [MCP tools](MCP)
- [Feature checklist](Features)
- [Accessibility and keyboard routes](Accessibility)
- [Architecture](Architecture)
- [Testing the GUI](Testing-the-GUI)
- [Packaging](Packaging)
- [Backlog](Backlog)

## Publishing this wiki

The wiki is generated from `docs/` in the repository. After the first wiki page
exists on GitHub, run:

```sh
scripts/publish-wiki.sh
```

The script copies the source docs into the wiki working tree and pushes. It
fails with instructions when the wiki repository has not been initialized yet.
