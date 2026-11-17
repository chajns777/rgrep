# rgrep

A fast and lightweight command line text search utility written in Rust, designed to replicate the core functionality of Unix `grep` with modern regular expressions, buffered I/O, and safe concurrency.

## Features

* **Regex & Literal Search:** Powered by Rust's `regex` crate for guaranteed linear-time matching without catastrophic backtracking.
* **Unix Pipeline Ready:** Reads from single files, multiple files, or `stdin`, and adheres to standard POSIX exit codes (`0` for match, `1` for no match, `2` for error).
* **Smart Terminal Output:** Automatic TTY detection enables colored match highlighting in interactive terminals and strips ANSI codes when piped or redirected.
* **Memory Efficient:** Streams files line-by-line while reusing a single heap buffer, keeping memory usage flat regardless of file size.
* **Binary File Safety:** Peeks at initial file buffers for NUL (`0x00`) bytes to skip binary files cleanly without corrupting terminal state.

## Installation

### From Source

Ensure you have the Rust toolchain installed (`rustc` and `cargo`), then clone and build the release binary:

```bash
git clone [https://github.com/yourusername/rgrep.git](https://github.com/yourusername/rgrep.git)
cd rgrep
cargo build --release
```

The compiled binary will be located at `./target/release/rgrep`. Optionally, install it directly into your Cargo binary path:

```bash
cargo install --path .
```

## Usage

```bash
rgrep [OPTIONS] <PATTERN> [PATH]...
```

### Examples

```bash
# Search for a pattern in a single file
rgrep "fn main" src/main.rs

# Case-insensitive recursive search with line numbers
rgrep -rin "todo" src/

# Match whole words only across multiple files
rgrep -w "error" logs/app.log logs/server.log

# Count non-matching lines from standard input
cat access.log | rgrep -vc "200 OK"
```

### Supported Flags

| Flag | Long Flag | Description |
| :--- | :--- | :--- |
| `-i` | `--ignore-case` | Perform case-insensitive matching. |
| `-v` | `--invert-match` | Select non-matching lines. |
| `-n` | `--line-number` | Prefix each line of output with its 1-based line number. |
| `-c` | `--count` | Suppress normal output and print a count of matching lines per file. |
| `-r` | `--recursive` | Recursively search subdirectories. |
| `-w` | `--word-regexp` | Match only whole words surrounded by word boundaries. |
| | `--color <WHEN>` | Highlight matched strings (`auto`, `always`, `never`). Defaults to `auto`. |
| `-h` | `--help` | Print help information. |

## Architecture

The codebase separates CLI parsing, input traversal, pattern matching, and output formatting into decoupled modules so core logic can be tested independently of the filesystem and terminal.

### Directory Structure

```text
rgrep/
├── Cargo.toml
└── src/
    ├── main.rs        # Entry point, error handling, exit code mapping
    ├── lib.rs         # Public library interface & high-level orchestration
    ├── cli.rs         # Argument parsing & Config definition
    ├── matcher.rs     # Regex compilation & line evaluation
    ├── walker.rs      # File/directory traversal & binary detection
    └── printer.rs     # Buffered terminal formatting & color highlighting
```

### Data Flow

```text
[CLI Args] ──▶ cli::parse_args() ──▶ Config
                                       │
              ┌────────────────────────┼────────────────────────┐
              ▼                        ▼                        ▼
       Matcher::new()        walker::resolve_inputs()     Printer::new()
              │                        │                        │
              │               ┌────────┴────────┐               │
              │               ▼                 ▼               │
              │         [File (BufRead)]  [Stdin (BufRead)]     │
              │               │                 │               │
              │               └────────┬────────┘               │
              │                        ▼                        │
              └──────────────▶ Line-by-Line Loop ───────────────┘
                             (Reuses String buffer)
```

## Running Tests

Run the unit and integration test suites using Cargo:

```bash
cargo test
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
