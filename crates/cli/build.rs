// include_dir! embeds templates/ and web/dist at compile time but, on
// stable Rust, does not notice files added to them; watching the
// directories forces a rebuild.
//
// The frontend is only embedded when web/dist exists, so Rust-only work
// does not need Node. Without it, `advent build` explains how to fix that.
fn main() {
    println!("cargo:rerun-if-changed=../../templates");
    println!("cargo:rerun-if-changed=../../web/dist");
    println!("cargo::rustc-check-cfg=cfg(embedded_web)");
    if std::path::Path::new("../../web/dist/index.html").is_file() {
        println!("cargo::rustc-cfg=embedded_web");
    }
}
