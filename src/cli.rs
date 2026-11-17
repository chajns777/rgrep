use std::io::{self, IsTerminal};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}
impl ColorMode {
    pub fn should_color(self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => io::stdout().is_terminal(),
        }
    }
}
fn parse_color(val: &str) -> Result<ColorMode, String> {
    match val {
        "auto" => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never" => Ok(ColorMode::Never),
        _ => Err(format!(
            "Invalid --color value '{val}'. Expected: auto, always, never"
        )),
    }
}
pub struct Config {
    pub query: String,
    pub filenames: Vec<String>,
    pub ignore_case: bool,
    pub count_only: bool,
    pub line_numbers: bool,
    pub invert_match: bool,
    pub word_regexp: bool,
    pub recursive: bool,
    pub fixed_strings: bool,
    pub color: ColorMode,
}
pub enum ParseResult {
    Run(Config),
    Help,
}
pub fn print_help() {
    println!(
        "\
rgrep - Search for a pattern in files or standard input

Usage: rgrep [OPTIONS] <QUERY> [FILE]...

Arguments:
  <QUERY>          Text pattern to search for
  [FILE]...        Files or directories to search (reads from stdin if omitted,
                   or searches '.' when -r is enabled; use '-' for stdin)

Options:
  -i, --ignore-case    Case-insensitive search
  -c, --count          Print only the count of matching lines per file
  -n, --line-number    Prefix each matching line with its 1-based line number
  -v, --invert-match   Select non-matching lines
  -w, --word-regexp    Match only whole words
  -r, --recursive      Recursively search subdirectories
  -F, --fixed-strings  Interpret <QUERY> as a literal string, not a regex
      --color <WHEN>   Highlight matches: auto, always, or never [default: auto]
  -h, --help           Print this help message

Examples:
  rgrep -in \"pub fn\" src/lib.rs src/cli.rs
  rgrep -rnw \"Config\" src
  cat Cargo.toml | rgrep -n \"version\""
    );
}
impl Config {
    pub fn build(args: &[String]) -> Result<ParseResult, String> {
        let mut ignore_case = false;
        let mut fixed_strings = false;
        let mut recursive = false;
        let mut count_only = false;
        let mut line_numbers = false;
        let mut invert_match = false;
        let mut word_regexp = false;
        let mut color = ColorMode::Auto;
        let mut positional = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];
            if arg == "--help" {
                return Ok(ParseResult::Help);
            } else if arg == "--word-regexp" {
                word_regexp = true;
            } else if arg == "--fixed-strings" {
                fixed_strings = true;
            } else if arg == "--recursive" {
                recursive = true;
            } else if let Some(val) = arg.strip_prefix("--color=") {
                color = parse_color(val)?;
            } else if arg == "--color" {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "Option '--color' requires a value (auto, always, never)".to_string())?;
                color = parse_color(val)?;
            } else if arg.starts_with("--") {
                return Err(format!("Unknown option: {arg}"));
            } else if let Some(flags) = arg.strip_prefix('-') {
                if flags.is_empty() {
                    positional.push(arg);
                } else {
                    for ch in flags.chars() {
                        match ch {
                            'i' => ignore_case = true,
                            'c' => count_only = true,
                            'n' => line_numbers = true,
                            'v' => invert_match = true,
                            'r' => recursive = true,
                            'f' => fixed_strings = true,
                            'w' => word_regexp = true,
                            'h' => return Ok(ParseResult::Help),
                            _ => return Err(format!("Unknown flag: -{ch}")),
                        }
                    }
                }
            } else {
                positional.push(arg);
            }
            i += 1;
        }
        if positional.len() < 2 {
            return Err("Not enough arguments. Try 'rgrep --help' for usage.".to_string());
        }
        if positional.is_empty() {
            return Err("Not enough arguments. Try 'rgrep --help' for usage.".to_string());
        }
        let query = positional[0].clone();
        let filenames = if positional.len() == 1 {
            if recursive {
                vec![".".to_string()]
            } else {
                vec!["-".to_string()]
            }
        } else {
            positional[1..].iter().map(|s| (*s).clone()).collect()
        };
        Ok(ParseResult::Run(Config {
            query,
            filenames,
            ignore_case,
            count_only,
            line_numbers,
            invert_match,
            word_regexp,
            recursive,
            fixed_strings,
            color,
        }))
        }
    }
