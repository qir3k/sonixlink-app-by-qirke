fn main() {
    println!("cargo:rerun-if-changed=icon.rc");
    println!("cargo:rerun-if-changed=icon.ico");

    #[cfg(windows)]
    let _ = embed_resource::compile("icon.rc", embed_resource::NONE);
}
