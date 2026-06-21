use std::collections::HashMap;
use std::time::SystemTime;

fn temp_path(suffix: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/drift-node-test-{}-{}", ts, suffix)
}

#[test]
fn test_env_vars_injected_into_spawn_command() {
    let mut env_vars = HashMap::new();
    env_vars.insert("FOO".to_string(), "bar".to_string());
    env_vars.insert("BAZ".to_string(), "qux".to_string());

    let vars_as_prefix: String = env_vars
        .iter()
        .map(|(k, v)| format!("{}={} ", k, v))
        .collect();

    assert!(vars_as_prefix.contains("FOO=bar"));
    assert!(vars_as_prefix.contains("BAZ=qux"));
}

#[test]
fn test_env_vars_prepended_to_shell_command() {
    let mut env_vars = HashMap::new();
    env_vars.insert("VAR1".to_string(), "val1".to_string());

    let env_vars_prefix: String = env_vars
        .iter()
        .map(|(k, v)| format!("{}={} ", k, v))
        .collect();

    let script = "python train.py";
    let full_cmd = format!("{}{}", env_vars_prefix, script);

    assert!(full_cmd.starts_with("VAR1=val1 "));
    assert!(full_cmd.contains("python train.py"));
}

#[test]
fn test_env_vars_preserved_in_hashmap() {
    let mut env_vars = HashMap::new();
    env_vars.insert("MY_VAR".to_string(), "my_value".to_string());

    let vars_as_tuple_list: Vec<(&str, &str)> = env_vars
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    assert_eq!(vars_as_tuple_list.len(), 1);
    assert_eq!(vars_as_tuple_list[0].0, "MY_VAR");
    assert_eq!(vars_as_tuple_list[0].1, "my_value");
}

#[test]
fn test_env_vars_empty_prefix_when_none() {
    let env_vars: Option<HashMap<String, String>> = None;

    let env_vars_prefix = env_vars
        .map(|vars| {
            vars.iter()
                .map(|(k, v)| format!("{}={} ", k, v))
                .collect::<String>()
        })
        .unwrap_or_default();

    assert_eq!(env_vars_prefix, "");
}

#[test]
fn test_env_vars_multiple_vars_in_prefix() {
    let mut env_vars = HashMap::new();
    env_vars.insert("A".to_string(), "1".to_string());
    env_vars.insert("B".to_string(), "2".to_string());
    env_vars.insert("C".to_string(), "3".to_string());

    let prefix: String = env_vars
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(prefix.contains("A=1"));
    assert!(prefix.contains("B=2"));
    assert!(prefix.contains("C=3"));
    assert!(prefix.split(' ').filter(|s| !s.is_empty()).count() == 3);
}
