extern crate embed_resource;

fn main() {
    // backup.ico is a placeholder. Replace with a real .ico file.
    embed_resource::compile("app-icon.rc");
}
