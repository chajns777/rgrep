use std::path::{Path, PathBuf};
use std::error::Error;
use std::fs;
use std::io::{self, Read};
use regex::{Regex, RegexBuilder};
enum InputSource {
    Stdin,
    File(PathBuf),
}
pub mod cli;
pub fn run(config: cli::Config) -> Result<(), Box<dyn Error>> {
        let re = build_regex(
            &config.query,
            config.ignore_case,
            config.word_regexp,
            config.fixed_strings,
        )?;
    let use_color = config.color.should_color();
    let mut had_error = false;
    let targets = resolve_targets(&config.filenames, config.recursive, &mut had_error);
    let show_filename = config.recursive || targets.len() > 1;
    for target in &targets {
        let (label, contents) = match target {
            InputSource::Stdin => {
                let mut buf = String::new();
                if let Err(err) = io::stdin().read_to_string(&mut buf) {
                    eprintln!("rgrep: (standard input): {err}");
                    had_error = true;
                    continue;
                }
                ("(standard input)".to_string(), buf)
            }
            InputSource::File(path) => {
                let filename = path.to_string_lossy().into_owned();
                match fs::read_to_string(path) {
                    Ok(c) => (filename, c),
                    Err(err) if err.kind() == io::ErrorKind::InvalidData => continue,
                    Err(err) => {
                        eprintln!("rgrep: {filename}: {err}");
                        had_error = true;
                        continue;
                    }
                }
            }
        };
        let results = search(&re, &contents, config.invert_match);
        let file_prefix = if !show_filename {
            String::new()
        } else if use_color {
            format!("\x1b[35m{label}\x1b[36m:\x1b[0m")
        } else {
            format!("{label}:")
        };
        if config.count_only {
            println!("{file_prefix}{}", results.len());
        } else {
            for (line_num, line, spans) in results {
                let formatted = if use_color {
                    highlight_line(line, &spans)
                } else {
                    line.to_string()
                };
                if config.line_numbers {
                    let num_prefix = if use_color {
                        format!("\x1b[32m{line_num}\x1b[36m:\x1b[0m")
                    } else {
                        format!("{line_num}:")
                    };
                    println!("{file_prefix}{num_prefix}{formatted}");
                } else {
                    println!("{file_prefix}{formatted}");
                }
            }
        }
    }
    if had_error {
        return Err("One or more files could not be read".into());
    }
    Ok(())
}
pub fn search<'a>(
    re: &Regex,
    contents: &'a str,
    invert: bool,
) -> Vec<(usize, &'a str, Vec<(usize, usize)>)>{
    contents
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_num = idx + 1;
            let spans: Vec<(usize, usize)> = re
                .find_iter(line)
                .map(|m| (m.start(), m.end()))
                .collect();
            let matched = !spans.is_empty();
            if matched != invert {
                Some((line_num, line, if invert { Vec::new() } else { spans }))
            } else {
                None
            }
        })
        .collect()
}
pub fn search_case_insensitive<'a>(
    query: &str,
    contents: &'a str,
    invert: bool,
) -> Vec<(usize, &'a str)> {
    let query = query.to_lowercase();
    let mut results = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let is_match = line.to_lowercase().contains(&query);
        if is_match != invert {
            results.push((index + 1, line));
        }
    }
    results
}
pub fn build_regex(
    query: &str,
    ignore_case: bool,
    word_regexp: bool,
    fixed_strings: bool,
) -> Result<Regex, regex::Error> {
    let mut pattern = if fixed_strings {
        regex::escape(query)
    } else {
        query.to_string()
    };

    if word_regexp {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    RegexBuilder::new(&pattern)
        .case_insensitive(ignore_case)
        .build()
}
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
fn is_whole_word(line: &str, start: usize, end: usize) -> bool {
    let left_ok = line[..start]
        .chars()
        .next_back()
        .map_or(true, |c| !is_word_char(c));
    let right_ok = line[end..]
        .chars()
        .next()
        .map_or(true, |c| !is_word_char(c));
    left_ok && right_ok
}
fn find_matches(line: &str, query: &str, ignore_case: bool, word_regexp: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut spans = Vec::new();
    if !ignore_case {
        let mut offset = 0;
        while let Some(pos) = line[offset..].find(query) {
            let start = offset + pos;
            let end = start + query.len();
            if !word_regexp || is_whole_word(line, start, end) {
                spans.push((start, end));
                offset = end;
            } else {
                offset = start + line[start..].chars().next().map_or(1, char::len_utf8);
            }
        }
    } else {
        let query_lower = query.to_lowercase();
        let mut indices: Vec<usize> = line.char_indices().map(|(i, _)| i).collect();
        indices.push(line.len());
        let mut i = 0;
        while i < indices.len() - 1 {
            let start = indices[i];
            let mut matched_end = None;
            for j in (i + 1)..indices.len() {
                let end = indices[j];
                if line[start..end].to_lowercase() == query_lower {
                    matched_end = Some((j, end));
                    break;
                }
            }
            if let Some((next_i, end)) = matched_end {
                if !word_regexp || is_whole_word(line, start, end) {
                    spans.push((start, end));
                    i = next_i;
                    continue;
                }
            }
            i += 1;
        }
    }
    spans
}
fn highlight_line(line: &str, spans: &[(usize, usize)]) -> String {
    if spans.is_empty() {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + spans.len() * 11);
    let mut last = 0;
    for &(start, end) in spans {
        out.push_str(&line[last..start]);
        out.push_str("\x1b[1;31m");
        out.push_str(&line[start..end]);
        out.push_str("\x1b[0m");
        last = end;
    }
    out.push_str(&line[last..]);
    out
}
fn collect_files(path: &Path, files: &mut Vec<PathBuf>, had_error: &mut bool) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("rgrep: {}: {err}", path.display());
            *had_error = true;
            return;
        }
    };
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        match entry {
            Ok(e) => paths.push(e.path()),
            Err(err) => {
                eprintln!("rgrep: {}: {err}", path.display());
                *had_error = true;
            }
        }
    }
    paths.sort();
    for entry_path in paths {
        if entry_path.is_dir() {
            collect_files(&entry_path, files, had_error);
        } else if entry_path.is_file() {
            files.push(entry_path);
        }
    }
}
fn resolve_targets(
    inputs: &[String],
    recursive: bool,
    had_error: &mut bool,
) -> Vec<InputSource> {
    let mut targets = Vec::new();
    for raw in inputs {
        if raw == "-" {
            targets.push(InputSource::Stdin);
            continue;
        }
        let path = Path::new(raw);
        if path.is_dir() {
            if recursive {
                let mut dir_files = Vec::new();
                collect_files(path, &mut dir_files, had_error);
                targets.extend(dir_files.into_iter().map(InputSource::File));
            } else {
                eprintln!("rgrep: {raw}: Is a directory");
                *had_error = true;
            }
        } else {
            targets.push(InputSource::File(path.to_path_buf()));
        }
    }
    targets
}
