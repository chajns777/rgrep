use std::error::Error;
use std::fs;
pub mod cli;
pub fn run(config: cli::Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&config.filename)?;
    let results = if config.ignore_case {
        search_case_insensitive(&config.query, &contents)
    } else {
        search(&config.query, &contents)
    };
    if config.count_only {
        println!("{}", results.len());
    } else {
        for (line_num, line) in results {
            if config.line_numbers {
                println!("{line_num}:{line}");
            } else {
                println!("{line}");
            }
        }
    }
    Ok(())
}
pub fn search_case_insensitive<'a>(
    query: &str,
    contents: &'a str,
) -> Vec<(usize, &'a str)> {
    let query = query.to_lowercase();
    let mut results = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.to_lowercase().contains(&query) {
            results.push((index + 1, line))
        }
    }
    results
}
pub fn search<'a>(query: &str, contents: &'a str) -> Vec<(usize, &'a str)> {
    let mut results = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.contains(query) {
            results.push((index + 1, line));
        }
    }
    results
}