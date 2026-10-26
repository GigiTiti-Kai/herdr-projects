//! Memory as the board measures it: what this machine has available for a new program, `MemAvailable` in
//! `/proc/meminfo`. `HERDR_PROJECTS_MEMINFO` names another file to read instead, for a test: the memory wait
//! (docs/memory.md) never depends on the machine the tests run on.

/// The bytes available for a new program; None where that cannot be read (no `/proc/meminfo`: not Linux).
pub fn available() -> Option<u64> {
    let at = std::env::var("HERDR_PROJECTS_MEMINFO").unwrap_or_else(|_| "/proc/meminfo".into());
    parse(&std::fs::read_to_string(at).ok()?)
}
/// `MemAvailable:   1843200 kB` in bytes.
pub fn parse(text: &str) -> Option<u64> {
    let line = text.lines().find_map(|l| l.strip_prefix("MemAvailable:"))?;
    line.split_whitespace().next()?.parse::<u64>().ok().map(|kb| kb * 1024)
}

#[cfg(test)]
mod tests {
    #[test]
    fn mem_available_is_read_in_bytes() {
        assert_eq!(super::parse("MemTotal:  8000000 kB\nMemFree:  100 kB\nMemAvailable:    1843200 kB\n"), Some(1_887_436_800));
        assert_eq!(super::parse("MemTotal:  8000000 kB\n"), None);
    }
}
