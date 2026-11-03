//! Embeds the app icon as a Windows resource so Explorer and the taskbar show it.
fn main() {
    println!("cargo::rerun-if-changed=assets/app.rc");
    println!("cargo::rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/app.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("compile Windows resources");
    }
}
