use std::fs;
pub fn load_proxies(path: &str) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(contents) => contents.lines().map(|s| s.to_string()).collect(),
        Err(_) => vec![],
    }
}
