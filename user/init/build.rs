use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo"),
    );
    let linker_script = manifest_dir.join("linker.ld");

    println!("cargo:rerun-if-changed={}", linker_script.display());
    let storage_binary = env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME")
        .is_some()
        .then_some("bndroid-storage-server");
    for binary in [
        "bndroid-init",
        "bndroid-service-manager",
        "bndroid-echo-provider",
        "bndroid-echo-client",
        "bndroid-surface-server",
        "bndroid-input-server",
        "bndroid-launcher",
        "bndroid-app",
    ]
    .into_iter()
    .chain(
        env::var_os("CARGO_FEATURE_ANDROIDBOX_PROCESS0")
            .is_some()
            .then_some("bndroid-android-app"),
    )
    .chain(storage_binary)
    {
        println!(
            "cargo:rustc-link-arg-bin={binary}=-T{}",
            linker_script.display()
        );
        for argument in [
            "--build-id=none",
            "-static",
            "-z",
            "max-page-size=4096",
            "-z",
            "common-page-size=4096",
            "--gc-sections",
            "--fatal-warnings",
        ] {
            println!("cargo:rustc-link-arg-bin={binary}={argument}");
        }
    }
}
