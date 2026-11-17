pub struct Config {
    pub query: String,
    pub filename: String,
    pub ignore_case: bool,
    pub count_only: bool,
    pub line_numbers: bool,
}
impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        let mut line_numbers = false;
        let mut ignore_case = false;
        let mut count_only = false;
        let mut positional = Vec::new();
        for arg in &args[1..] {
            if let Some(flags) = arg.strip_prefix("-") {
                if flags.is_empty() {
                    positional.push(arg);
                    continue;
                }
                for ch in flags.chars() {
                    match ch {
                        'n' => line_numbers = true,
                        'i' => ignore_case = true,
                        'c' => count_only = true,
                        _ => return Err(format!("Unknown flag: -{ch}")),
                    }
                }
            } else {
                positional.push(arg);
            }
        }
        if positional.len() < 2 {
            return Err("Not enough arguments".to_string());
        }
        Ok(Config {
            query: positional[0].clone(),
            filename: positional[1].clone(),
            ignore_case,
            count_only,
            line_numbers,
        })
    }
}