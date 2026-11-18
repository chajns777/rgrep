# rgrep

A fast, lightweight command-line search tool written in Rust. `rgrep` takes the everyday Unix `grep` workflow and gives it a modern Rust makeover: linear-time regular expressions, parallel directory searching, context lines, and sensible terminal highlighting.

---

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Arguments and input behaviour](#arguments-and-input-behaviour)
  - [Options](#options)
  - [Examples](#examples)
  - [Exit codes](#exit-codes)
- [Architecture](#architecture)
  - [Project structure](#project-structure)
  - [Execution pipeline](#execution-pipeline)
- [Development and testing](#development-and-testing)
- [Licence](#licence)

---

## Features

- **Parallel file searching:** Uses [`rayon`](https://docs.rs/rayon) to search several files at once across all your CPU cores. Output is mutex-synchronised, so lines never get jumbled together.
- **Regex and literal search:** Built on Rust's [`regex`](https://docs.rs/regex) crate, which guarantees linear-time O(n) matching with no catastrophic backtracking. Need an exact string instead? Use `-F` / `--fixed-strings`.
- **Context windows (`-A`, `-B`, `-C`):** See the lines before and after a match. Overlapping windows are merged automatically, and separate blocks are divided with `--`.
- **Recursive search and glob filtering:** Dive into nested directories with `-r`, and narrow things down with `--include` and `--exclude`. Binary (non-UTF-8) files are quietly skipped.
- **Script-friendly quiet mode (`-q`):** Stops at the first match and exits with `0` without printing anything. Perfect for shell conditionals and CI pipelines.
- **Terminal-aware colours:** `rgrep` checks whether `stdout` is an interactive terminal (`--color auto`). Matches show in red, file paths in magenta, line numbers in green and separators in cyan. When you pipe the output, the ANSI codes are stripped out cleanly.

---

## Installation

### Prerequisites

- A Rust toolchain (`rustc` and `cargo`, edition 2021 or newer)

### Build from source

```bash
git clone https://github.com/chajns777/rgrep.git
cd rgrep
cargo build --release
```

The compiled binary will be at `./target/release/rgrep`.

To install it globally into your Cargo bin folder (`~/.cargo/bin`):

```bash
cargo install --path .
```

---

## Usage

```bash
rgrep [OPTIONS] <QUERY> [FILE]...
```

### Arguments and input behaviour

| Argument | Description |
| --- | --- |
| `<QUERY>` | The regular expression to search for (or a literal string when `-F` is set). |
| `[FILE]...` | Zero or more files or directories to search. |

A few things worth knowing:

- **No `[FILE]...` given:** `rgrep` reads from standard input (stdin).
- **No `[FILE]...` with `-r` / `--recursive`:** It searches the current directory (`.`).
- **An explicit `-`:** Reads stdin at that position in the argument list. When searching multiple sources, matches are labelled `(standard input)`.
- **A directory without `-r`:** Prints `rgrep: <path>: Is a directory` to stderr and carries on with the remaining targets.

### Options

| Short | Long | Description |
| --- | --- | --- |
| `-i` | `--ignore-case` | Match case-insensitively. |
| `-v` | `--invert-match` | Select lines that do **not** match `<QUERY>`. |
| `-n` | `--line-number` | Prefix each output line with its 1-based line number (`:` for matches, `-` for context). |
| `-c` | `--count` | Suppress normal output and print only the number of matching lines per file. |
| `-w` | `--word-regexp` | Match whole words only, by wrapping the pattern in `\b(?:...)\b`. |
| `-r` | `--recursive` | Recursively walk subdirectories and search all matching text files. |
| `-F` | `--fixed-strings` | Treat `<QUERY>` as a literal string, not a regular expression. |
| `-q` | `--quiet` | Suppress all normal output and exit with `0` as soon as a match is found. |
| `-A <NUM>` | | Print `<NUM>` lines of trailing context after each match. |
| `-B <NUM>` | | Print `<NUM>` lines of leading context before each match. |
| `-C <NUM>` | | Print `<NUM>` lines of context both before and after each match. |
| | `--include=<GLOB>` | Search only files whose names match `<GLOB>` (e.g. `*.rs`). |
| | `--exclude=<GLOB>` | Skip files whose names match `<GLOB>` (e.g. `*.log`). |
| | `--color <WHEN>` | Control ANSI colour highlighting: `auto` (default), `always` or `never`. |
| `-h` | `--help` | Print usage instructions and available options. |

### Examples

```bash
# Recursively search only Rust files for public items, with line numbers
rgrep -rn --include="*.rs" "^pub (fn|struct|enum)" src/

# Show 2 lines of context before and after every match
rgrep -n -C 2 "Mutex" src/lib.rs

# Search for an exact literal signature that contains regex characters
rgrep -rnF "Result<(), Box<dyn Error>>" src/

# Use quiet mode in a shell script to check whether a pattern exists
if rgrep -rq "TODO" src/; then
  echo "Found remaining TODOs!"
fi

# Count non-matching lines piped from standard input
cat access.log | rgrep -vc "200 OK"

# Force ANSI colours when piping into a pager like less
rgrep --color always -rn "Config" src/ | less -R
```

### Exit codes

`rgrep` follows the standard POSIX `grep` exit codes, so you can rely on it in scripts and automation:

| Code | Meaning |
| --- | --- |
| `0` | One or more matches were found (or `--help` was shown). |
| `1` | No matches were found in any target. |
| `2` | A CLI argument error, invalid regex syntax, or file I/O error occurred. |

---

## Architecture

The codebase is split into three focused modules: CLI parsing, target discovery plus parallel execution, and ANSI formatting. This keeps the core search logic separate from process exit handling.

### Project structure

```text
rgrep/
├── Cargo.toml         # Package manifest and dependencies (regex, rayon)
├── Cargo.lock         # Locked dependency graph
└── src/
    ├── main.rs        # Process entry point and exit code mapping
    ├── cli.rs         # Argument parser, combined flag expansion, ColorMode, and Config
    └── lib.rs         # Regex builder, recursive file filter, Rayon runner, and context search
```

### Execution pipeline

```text
  std::env::args()
         │
         ▼
  cli::Config::build() ──▶ Parse short/long flags, context counts, globs, and ColorMode
         │
         ▼
  lib::run(config)
    ├──▶ build_regex()         Compile Regex once (-F escape, -w \b boundaries, -i case)
    ├──▶ resolve_targets()     Walk directories via fs::read_dir (sorted) & apply --include/--exclude
    ├──▶ [If -q / --quiet]     Parallel short-circuit via par_iter().any() ──▶ exit(0) or exit(1)
    └──▶ Parallel Target Loop  (rayon::par_iter() + Mutex-synchronised stdout)
           ├── Read UTF-8 contents (silently skipping non-UTF-8 binary files)
           ├── search_with_context() ──▶ Merge overlapping -A/-B/-C windows & insert '--' separators
           └── Format & print with TTY-aware ANSI colours (file path, line number, highlights)
```

---

## Development and testing

```bash
# Check compilation and warnings
cargo check

# Run Clippy lints
cargo clippy

# Run unit and integration tests
cargo test
```

A quick smoke test:

```bash
cargo run -- -rn --include="*.rs" "^pub (fn|struct|enum)" src
```

---

## Licence

This project is licensed under the MIT Licence. See the [LICENSE](LICENSE) file for details.