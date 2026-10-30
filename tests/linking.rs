//! [real programs] The linker of the builds (`scripts/linker.sh`, which the C compiler runs as `scripts/ld/ld`): mold
//! when it is installed; the linker Rust uses by itself when it is not, for a release build and when turned off.
use std::path::{Path, PathBuf};

/// A home with a cache folder, and a `rust-lld`, an `ld` and (once installed) a mold that write down their name and
/// the arguments they were given, one a line.
struct Bench { base: PathBuf }
impl Bench {
    fn new(name: &str) -> Bench {
        let base = std::env::temp_dir().join(format!("sw-linking-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("cache")).unwrap();
        let b = Bench { base };
        b.linker(&b.base.join("bin/rust-lld"), "rust-lld");
        b.linker(&b.base.join("bin/ld"), "ld");
        b
    }
    fn linker(&self, p: &Path, name: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, format!("#!/bin/sh\n{{ echo {name}; for a; do echo \"$a\"; done; }} > '{}'\n", self.saw().display())).unwrap();
        std::fs::set_permissions(p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    fn saw(&self) -> PathBuf { self.base.join("saw") }
    /// Installs a stand-in where scripts/linker.sh looks for the mold it installed.
    fn install_mold(&self) {
        let script = std::fs::read_to_string(root().join("scripts/linker.sh")).unwrap();
        let version = script.lines().find_map(|l| l.strip_prefix("ln_version=")).unwrap();
        self.linker(&self.base.join(format!("cache/herdr-projects/mold-{version}/bin/mold")), "mold");
    }
    /// The linker as the C compiler runs it: which linker it ended in, and with what.
    fn link(&self, args: &[&str], env: &[(&str, &str)]) -> Vec<String> {
        let _ = std::fs::remove_file(self.saw());
        let o = std::process::Command::new(root().join("scripts/ld/ld")).args(args).env_clear()
            .env("HOME", self.base.join("home")).env("XDG_CACHE_HOME", self.base.join("cache"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.base.join("bin").display())).envs(env.iter().copied()).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        std::fs::read_to_string(self.saw()).unwrap().lines().map(String::from).collect()
    }
}
impl Drop for Bench { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.base); } }

fn root() -> &'static Path { Path::new(env!("CARGO_MANIFEST_DIR")) }
const LINK: &[&str] = &["--eh-frame-hdr", "-m", "elf_x86_64", "main.o", "-o", "/w/target/quick/deps/herdr_projects-1"];
fn ran(linker: &[&str], args: &[&str]) -> Vec<String> { linker.iter().chain(args).map(|s| s.to_string()).collect() }
/// The linker Rust uses by itself on this machine: its own lld on x86_64, the system's ld elsewhere.
fn as_rust(args: &[&str]) -> Vec<String> {
    if std::env::consts::ARCH == "x86_64" { ran(&["rust-lld", "-flavor", "gnu"], args) } else { ran(&["ld"], args) }
}

#[test]
fn real_programs_without_mold_a_build_links_as_rust_does_by_itself() {
    let _env = crate::common::env_kept(); // no world moves HOME or PATH under the programs it starts
    let b = Bench::new("none");
    assert_eq!(b.link(LINK, &[]), as_rust(LINK));
    std::fs::remove_file(b.base.join("bin/rust-lld")).unwrap();
    assert_eq!(b.link(LINK, &[]), ran(&["ld"], LINK), "a Rust with no lld of its own links with the system's ld");
}

#[test]
fn real_programs_with_mold_installed_a_build_links_with_it() {
    let _env = crate::common::env_kept(); // no world moves HOME or PATH under the programs it starts
    let b = Bench::new("mold");
    b.install_mold();
    assert_eq!(b.link(LINK, &[]), ran(&["mold"], LINK));
}

#[test]
fn real_programs_a_release_build_and_a_build_with_mold_turned_off_link_as_before() {
    let _env = crate::common::env_kept(); // no world moves HOME or PATH under the programs it starts
    let b = Bench::new("release");
    b.install_mold();
    let release = ["main.o", "-o", "/w/target/x86_64-unknown-linux-gnu/release/deps/herdr_projects-1"];
    assert_eq!(b.link(&release, &[]), as_rust(&release));
    assert_eq!(b.link(LINK, &[("HERDR_PROJECTS_NO_MOLD", "1")]), as_rust(LINK));
}

#[test]
fn the_cargo_config_is_the_same_words_in_every_worktree() {
    // cargo keeps what it built by these words: a path of one worktree in them would make a build copied from another
    // (scripts/warm-start.sh), and every library in the shared build cache, out of date
    let config = std::fs::read_to_string(root().join(".cargo/config.toml")).unwrap();
    assert!(config.contains(r#""-Clink-arg=-Bscripts/ld/""#), "{config}");
    assert!(!config.contains("linker ="), "a linker named in the config is given to cargo as its whole path: {config}");
    assert_eq!(std::fs::read_link(root().join("scripts/ld/ld")).unwrap(), Path::new("../linker.sh"));
}

/// The platforms the program is built on (Linux and macOS, x86_64 and ARM) and for (the release files, musl on Linux).
const TARGETS: &[&str] = &["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "aarch64-apple-darwin"];

/// The rustc that built the tests, by rustup's words for it as they were then: the tests' worlds move HOME and PATH.
fn rustc() -> std::process::Command {
    let (home, toolchain) = (option_env!("RUSTUP_HOME"), option_env!("RUSTUP_TOOLCHAIN"));
    if let (Some(h), Some(t)) = (home, toolchain) {
        let own = Path::new(h).join("toolchains").join(t).join("bin/rustc");
        if own.exists() { return std::process::Command::new(own); }
    }
    let mut c = std::process::Command::new("rustc");
    if let (Some(h), Some(t)) = (home, toolchain) { c.env("RUSTUP_HOME", h).env("RUSTUP_TOOLCHAIN", t); }
    c
}

/// The flags cargo gives every build for `target` from .cargo/config.toml: `[build]`'s and then its own `[target.…]`'s.
fn flags_for(config: &toml::Table, target: &str) -> Vec<String> {
    let of = |t: Option<&toml::Value>| -> Vec<String> {
        match t.and_then(|t| t.get("rustflags")) {
            Some(toml::Value::String(s)) => s.split_whitespace().map(String::from).collect(),
            Some(toml::Value::Array(a)) => a.iter().map(|f| f.as_str().unwrap().to_string()).collect(),
            _ => vec![],
        }
    };
    let mut flags = of(config.get("build"));
    flags.extend(of(config.get("target").and_then(|t| t.get(target))));
    flags
}

#[test]
fn real_programs_rustc_takes_the_cargo_configs_flags_on_every_platform_with_stable_rust() {
    let _env = crate::common::env_kept(); // no world moves HOME or PATH under the programs it starts
    // a flag that is unstable on one target fails every build there at once (`-C linker-features=-lld` on ARM Linux):
    // rustc says so before it needs that target's library, so this holds on a machine of any platform
    let config: toml::Table = std::fs::read_to_string(root().join(".cargo/config.toml")).unwrap().parse().unwrap();
    let named: Vec<String> = config.get("target").and_then(|t| t.as_table()).map(|t| t.keys().cloned().collect()).unwrap_or_default();
    assert!(named.iter().all(|t| !t.starts_with("cfg(")), "teach this test the `cfg(…)` sections: {named:?}");
    for target in TARGETS.iter().map(|t| t.to_string()).chain(named.iter().cloned()) {
        let flags = flags_for(&config, &target);
        assert!(flags.iter().all(|f| !f.starts_with("-Z")), "{target}: stable Rust takes no -Z flag: {flags:?}");
        let o = rustc().args(["--print", "cfg", "--target", &target]).args(&flags)
            .env_remove("RUSTC_BOOTSTRAP").output().unwrap();
        assert!(o.status.success(), "{target} with {flags:?}: {}", String::from_utf8_lossy(&o.stderr));
    }
    for target in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        assert!(flags_for(&config, target).iter().any(|f| f == "-Clink-arg=-Bscripts/ld/"), "{target} links with mold");
    }
}
