fn main() {
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rustc-link-search={}", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=link.x");
    println!("cargo:rerun-if-changed=memory.x");
}
