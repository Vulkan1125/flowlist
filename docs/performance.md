# Performance check

Flowlist keeps the whole library in memory and renders query results through
one shared query engine. The numbers below show that the domain layer stays
comfortable at ten thousand tasks and degrades linearly after that.

## Rerun it

```sh
cargo build --release
scripts/perf-check.sh              # 10,000 tasks
scripts/perf-check.sh 30000        # any size
KEEP=1 scripts/perf-check.sh       # keep the generated library
```

`scripts/perf-check.sh` writes a TaskPaper seed with projects, folders, tags,
due dates, flags and estimates, imports it, then times `list`, grouped `list`,
`forecast`, `flagged` and `stats`. Each measurement is a wall-clock subprocess
run, so it includes process start-up — the cost a CLI user actually pays.

The GUI renders the same `query::view` groups in-process, so these figures are
a useful upper bound for its domain work.

## Results

Machine: Mac mini (Apple M4, 16 GB), macOS 27.2.1, release build,
`lto = "thin"`, single run per cell. Treat them as indicative, not a benchmark
suite.

### CLI, milliseconds per command

| Library | Import | `list` | grouped `list` | Forecast | Flagged | Stats |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 tasks | 47 | 26 | 25 | 21 | 20 | 19 |
| 10,000 tasks | 86 | 159 | 161 | 116 | 99 | 89 |
| 30,000 tasks | 167 | 450 | 475 | 324 | 288 | 254 |

The 10,000-task library has 500 projects in 20 folders and 4 tags.

Reading (`list`, `forecast`, `flagged`, `stats`) scales roughly linearly.
Import is dominated by parsing and validation, and grows more slowly because
the seed spends proportionally more time on container creation at small sizes.
Sorting now caches its case-folded keys once per row and reuses tag usage counts
across the sidebar order, so the 10,000-task `list` check measures around 155 ms
in the same setup.

### Desktop app

| Library | Window visible | Resident memory |
| --- | ---: | ---: |
| 7 tasks | 0.55 s | 130 MB |
| 10,000 tasks | 0.7–1.0 s | 259 MB |

“Window visible” is measured from process spawn until the accessibility API
reports the Flowlist window, which is a conservative proxy for first paint.
Memory is resident size after the window settles; most of it is GPUI's renderer
and font rasterisation rather than the task data.

## What this means

- A 10,000-task library opens in about a second and answers a perspective in
  under 200 ms including process start-up.
- 30,000 tasks still feel usable for reading, but each command is noticeably
  slower; the linear cost is the whole-library in-memory model.
- The GUI keeps one store in memory and reloads only when another process
  changes the library, so typing, selection and repaint are independent of
  library size until a query or mutation runs.
- If libraries grow much beyond this, the next step is indexing search and
  paginating very large views rather than changing the storage format.
