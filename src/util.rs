pub fn strip_prefix_0x(s: &str) -> &str {
    if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    }
}
