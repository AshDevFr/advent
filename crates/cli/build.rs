// include_dir! embeds templates/event at compile time but, on stable Rust,
// does not notice files added to it; watching the directory forces a rebuild.
fn main() {
    println!("cargo:rerun-if-changed=../../templates/event");
}
