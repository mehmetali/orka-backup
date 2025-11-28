extern crate embed_resource;

fn main() {
    // Tell Cargo to rerun this script if the resource file changes.
    println!("cargo:rerun-if-changed=app-icon.rc");
    embed_resource::compile("app-icon.rc");
}
