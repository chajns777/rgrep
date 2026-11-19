use rayon::prelude::*;
use std::sync::Mutex;
use std::error::Error;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
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
    let targets = resolve_targets(
        &config.filenames,
        config.recursive,
        config.include_pattern.as_deref(),
        config.exclude_pattern.as_deref(),
        &mut had_error,
    );
    let show_filename = config.recursive || targets.len() > 1;
    if config.quiet {
        let found_match = targets.par_iter().any(|target| {
            match target {
                InputSource::Stdin => {
                    let mut buf = String::new();
                    if io::stdin().read_to_string(&mut buf).is_ok() {
                        !search_with_context(&re, &buf, config.invert_match, 0, 0).is_empty()
                    } else {
                        false
                    }
                }
                InputSource::File(path) => {
                    fs::read_to_string(path)
                        .map(|c| !search_with_context(&re, &c, config.invert_match, 0, 0).is_empty())
                        .unwrap_or(false)
                }
            }
        });
        if found_match {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }
    let stdout_lock = Mutex::new(());
    let shared_had_error = Mutex::new(had_error);
    targets.par_iter().for_each(|target| {
        let (label, contents) = match target {
            InputSource::Stdin => {
                let mut buf = String::new();
                if let Err(err) = io::stdin().read_to_string(&mut buf) {
                    eprintln!("rgrep: (standard input): {err}");
                    *shared_had_error.lock().unwrap() = true;
                    return;
                }
                ("(standard input)".to_string(), buf)
            }
            InputSource::File(path) => {
                let filename = path.to_string_lossy().into_owned();
                match fs::read_to_string(path) {
                    Ok(c) => (filename, c),
                    Err(err) if err.kind() == io::ErrorKind::InvalidData => return,
                    Err(err) => {
                        eprintln!("rgrep: {filename}: {err}");
                        *shared_had_error.lock().unwrap() = true;
                        return;
                    }
                }
            }
        };
        let results = search_with_context(
            &re,
            &contents,
            config.invert_match,
            config.before_context,
            config.after_context,
        );
        let file_prefix = if !show_filename {
            String::new()
        } else if use_color {
            format!("\x1b[35m{label}\x1b[36m:\x1b[0m")
        } else {
            format!("{label}:")
        };
        let _lock = stdout_lock.lock().unwrap();
        if config.count_only {
            println!("{file_prefix}{}", results.len());
        } else {
            for (line_num, line, spans, is_separator) in results {
                if is_separator {
                    println!("{file_prefix}--");
                    continue;
                }
                let formatted = if use_color {
                    highlight_line(line, &spans)
                } else {
                    line.to_string()
                };
                if config.line_numbers {
                    let num_separator = if spans.is_empty() { "-" } else { ":" };
                    let num_prefix = if use_color {
                        format!("\x1b[32m{line_num}\x1b[36m{num_separator}\x1b[0m")
                    } else {
                        format!("{line_num}{num_separator}")
                    };
                    println!("{file_prefix}{num_prefix}{formatted}");
                } else {
                    println!("{file_prefix}{formatted}");
                }
            }
        }
    });
    if *shared_had_error.lock().unwrap() {
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
fn matches_filter(path: &Path, include: Option<&str>, exclude: Option<&str>) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if let Some(inc) = include {
        let matches_inc = if inc.starts_with("*.") {
            file_name.ends_with(&inc[1..])
        } else {
            file_name.contains(inc)
        };
        if !matches_inc {
            return false;
        }
    }
    if let Some(exc) = exclude {
        let matches_exc = if exc.starts_with("*.") {
            file_name.ends_with(&exc[1..])
        } else {
            file_name.contains(exc)
        };
        if matches_exc {
            return false;
        }
    }
    true
}
fn collect_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
    include: Option<&str>,
    exclude: Option<&str>,
    had_error: &mut bool,
) {
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
            collect_files(&entry_path, files, include, exclude, had_error);
        } else if entry_path.is_file() {
            if matches_filter(&entry_path, include, exclude) {
                files.push(entry_path);
            }
        }
    }
}

fn resolve_targets(
    inputs: &[String],
    recursive: bool,
    include: Option<&str>,
    exclude: Option<&str>,
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
                collect_files(path, &mut dir_files, include, exclude, had_error);
                targets.extend(dir_files.into_iter().map(InputSource::File));
            } else {
                eprintln!("rgrep: {raw}: Is a directory");
                *had_error = true;
            }
        } else {
            if matches_filter(path, include, exclude) {
                targets.push(InputSource::File(path.to_path_buf()));
            }
        }
    }
    targets
}
pub fn search_with_context<'a>(
    re: &Regex,
    contents: &'a str,
    invert: bool,
    before: usize,
    after: usize,
) -> Vec<(usize, &'a str, Vec<(usize, usize)>, bool)> {
    let lines: Vec<&str> = contents.lines().collect();
    let mut match_indices = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let spans: Vec<(usize, usize)> = re
            .find_iter(line)
            .map(|m| (m.start(), m.end()))
            .collect();
        let matched = !spans.is_empty();
        if matched != invert {
            match_indices.push(idx);
        }
    }
    if match_indices.is_empty() {
        return Vec::new();
    }
    let mut included_lines = vec![false; lines.len()];
    for &m_idx in &match_indices {
        let start = m_idx.saturating_sub(before);
        let end = std::cmp::min(lines.len() - 1, m_idx + after);
        for i in start..=end {
            included_lines[i] = true;
        }
    }
    let mut results = Vec::new();
    let mut last_included = None;
    for (idx, &include) in included_lines.iter().enumerate() {
        if include {
            if let Some(last) = last_included {
                if idx > last + 1 {
                    results.push((0, "", Vec::new(), true));
                }
            }
            let line_num = idx + 1;
            let line = lines[idx];
            let spans: Vec<(usize, usize)> = re
                .find_iter(line)
                .map(|m| (m.start(), m.end()))
                .collect();
            results.push((line_num, line, spans, false));
            last_included = Some(idx);
        }
    }
    results
}