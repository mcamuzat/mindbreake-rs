//! Enables `cfg(official_cards)` when the (git-ignored) official catalog is present.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(official_cards)");
    println!("cargo::rerun-if-changed=src/cards_official.rs");
    if std::path::Path::new("src/cards_official.rs").exists() {
        println!("cargo::rustc-cfg=official_cards");
    }
}
