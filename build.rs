//! Enables `cfg(official_cards)` when the (git-ignored) official catalog is present.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(official_cards)");
    // The whole directory, not just the file: a catalog moved away and back
    // keeps its old mtime, but the directory's own mtime changes.
    println!("cargo::rerun-if-changed=src");
    if std::path::Path::new("src/cards_official.rs").exists() {
        println!("cargo::rustc-cfg=official_cards");
    }
}
