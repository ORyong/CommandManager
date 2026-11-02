fn main() {
    println!("cargo:rerun-if-changed=packaging/app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var_os("CARGO_FEATURE_DESKTOP").is_some()
    {
        embed_resource::compile_for(
            "packaging/app.rc",
            ["command-manager"],
            embed_resource::NONE,
        )
        .manifest_required()
        .expect("Windows application icon/version resources must compile");
    }
}
