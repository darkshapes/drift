use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead, BufReader},
};

pub fn parse_env_file(path: &str) -> io::Result<HashMap<String, String>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut vars = HashMap::new();
    
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().to_string();
            vars.insert(key, val);
        }
    }
    
    Ok(vars)
}

pub fn filter_sensitive_keys(env_vars: HashMap<String, String>) -> HashMap<String, String> {
    let sensitive_patterns = ["KEY", "SECRET", "TOKEN", "PASSWORD", "PASS", "AUTH"];
    env_vars
        .into_iter()
        .filter(|(k, _)| !sensitive_patterns.iter().any(|p| k.contains(p)))
        .collect()
}
