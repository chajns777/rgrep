# rgrep

A fast, lightweight, POSIX-compliant command-line text search utility written in Rust. `rgrep` replicates the core workflow of Unix `grep` with regular expression matching, fixed-string literal search, recursive directory traversal, standard input piping, and terminal-aware ANSI color highlighting.

---

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Arguments & Input Behavior](#arguments--input-behavior)
  - [Options](#options)
  - [Examples](#examples)
  - [Exit Codes](#exit-codes)
- [Architecture](#architecture)
  - [Project Structure](#project-structure)
  - [Execution Pipeline](#execution-pipeline)
- [Development & Testing](#development--testing)
- [License](#license)

---

## Features

- **Regex & Fixed-String Engines:** Powered by Rust's [`regex`](https://docs.rs/regex) crate for guaranteed linear-time ($\mathcal{O}(n)$) matching immune to catastrophic backtracking, paired with `-F` / `--fixed-strings` (`regex::escape`) for exact literal searches.
- **Flexible Input Sources:** Searches single files, multiple files, recursive directory trees (`-r`), and standard input (`stdin` when paths are omitted or `-` is passed—even interleaved with real files).
- **Deterministic Directory Traversal:** Walks subdirectories via `std::fs::read_dir`, sorts entries lexicographically for reproducible output, and silently skips non-UTF-8 binary files (`io::ErrorKind::InvalidData`).
- **Terminal-Aware ANSI Highlighting:** Uses `std::io::IsTerminal` (`--color auto`) to highlight matches in yellow (`\x1b[43;30m`), filenames in magenta, line numbers in green, and separators in cyan when connected to an interactive TTY—automatically disabling escape codes when piped or redirected.
- **POSIX Flag Parsing:** Supports combined short flags (e.g., `-rinw`), long flags, explicit `-` stdin markers, and standard exit status conventions.

---

## Installation

### Prerequisites

- Rust toolchain (`rustc` and `cargo`, edition 2021+)

### Build from Source

```bash
git clone [https://github.com/yourusername/rgrep.git](https://github.com/yourusername/rgrep.git)
cd rgrep
cargo build --release
```

The compiled binary is written to `./target/release/rgrep`. To install it globally into your Cargo `bin` directory (`~/.cargo/bin`):

```bash
cargo install --path .
```

---

## Usage

```bash
rgrep [OPTIONS] <QUERY> [FILE]...
```

### Arguments & Input Behavior

| Argument | Description |
| :--- | :--- |
| `<QUERY>` | Regular expression pattern (or literal string when `-F` is set) to search for. |
| `[FILE]...` | Zero or more file or directory paths to search. |

- **Omitted `[FILE]...` (default mode):** Reads from standard input (`stdin`).
- **Omitted `[FILE]...` with `-r` / `--recursive`:** Defaults to searching the current directory (`.`).
- **Explicit `-`:** Reads from `stdin` at that position in the argument list and labels matches `(standard input)` when multiple targets are searched.
- **Directories without `-r`:** Prints `rgrep: <path>: Is a directory` to `stderr` and continues processing remaining targets.

### Options

| Short | Long | Description |
| :--- | :--- | :--- |
| `-i` | `--ignore-case` | Perform case-insensitive pattern matching. |
| `-v` | `--invert-match` | Select lines that do **not** match `<QUERY>`. |
| `-n` | `--line-number` | Prefix each matching line with its 1-based line number. |
| `-c` | `--count` | Suppress normal line output and print only the count of matching lines per target. |
| `-w` | `--word-regexp` | Match only whole words by wrapping the pattern in `\b(?:...)\b` word boundaries. |
| `-r` | `--recursive` | Recursively traverse subdirectories to search all text files. |
| `-F` | `--fixed-strings` | Interpret `<QUERY>` as a literal string rather than a regular expression. |
| | `--color <WHEN>` | Control ANSI color output: `auto` (TTY-aware default), `always`, or `never`. |
| `-h` | `--help` | Display usage instructions and flag reference. |

### Examples

```bash
# Search for a regex pattern across Rust source files with line numbers
rgrep -rn "^pub (fn|struct|enum)" src/

# Search for an exact literal signature containing regex metacharacters
rgrep -rnF "Result<(), Box<dyn Error>>" src/

# Case-insensitive whole-word search across specific files
rgrep -iw "config" src/cli.rs src/lib.rs

# Count non-matching lines piped from standard input
cat access.log | rgrep -vc "200 OK"

# Mix standard input (-) and a file in a single invocation
printf "pub fn piped()\n" | rgrep -n "pub fn" - src/lib.rs

# Force ANSI colors when piping into a pager like less
rgrep --color always -rn "TODO" src/ | less -R
```

### Exit Codes

`rgrep` follows standard POSIX `grep` exit status conventions so it integrates cleanly into shell scripts and CI pipelines:

| Code | Meaning |
| :--- | :--- |
| `0` | One or more matching lines were found (or `--help` was displayed). |
| `1` | No matching lines were found across any target. |
| `2` | A CLI syntax error, invalid regular expression, or I/O error occurred. |

---

## Architecture

The codebase separates CLI parsing, target resolution, regex compilation, and ANSI formatting into three focused modules so core search logic remains independent of process exit handling.

### Project Structure

```text
rgrep/
├── Cargo.toml         # Package manifest & regex crate dependency
├── Cargo.lock         # Locked dependency graph
└── src/
    ├── main.rs        # Process entry point,
    ├── cli.rs         # Argument parser, combined flag expansion, ColorMode, and Config
    └── lib.rs         # Regex compilation, target resolution, directory walker, and search runner
```

### Execution Pipeline

```text
  std::env::args()
         │
         ▼
  cli::Config::build() ──▶ Parse short/long flags, ColorMode, and positional args
         │
         ▼
  lib::run(config)
    ├──▶ build_regex()       Compile Regex once (-F escape, -w \b boundaries, -i case)
    ├──▶ resolve_targets()   Expand directories via fs::read_dir (sorted) & map '-' to Stdin
    └──▶ Per-Target Loop
           ├── Read UTF-8 contents (silently skipping non-UTF-8 binary files)
           ├── search(&re, &contents, invert) ──▶ Collect line numbers & byte match spans
           └── Format & print with TTY-aware ANSI colors (filename, line number, highlights)
```

---

## Development & Testing

### Build & Lint

```bash
# Check compilation and warnings
cargo check

# Run Clippy lints
cargo clippy

# Run unit and integration tests
cargo test
```

### Quick Smoke Test

```bash
cargo run -- -rn "^pub (fn|struct|enum)" src
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
