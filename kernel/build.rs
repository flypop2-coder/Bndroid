use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const IMAGE_ENVIRONMENTS: [(&str, &str); 10] = [
    ("BNDROID_INIT_ELF", "BNDR_INIT_ELF"),
    ("BNDROID_SERVICE_MANAGER_ELF", "BNDR_SERVICE_MANAGER_ELF"),
    ("BNDROID_ECHO_PROVIDER_ELF", "BNDR_ECHO_PROVIDER_ELF"),
    ("BNDROID_ECHO_CLIENT_ELF", "BNDR_ECHO_CLIENT_ELF"),
    ("BNDROID_SURFACE_SERVER_ELF", "BNDR_SURFACE_SERVER_ELF"),
    ("BNDROID_LAUNCHER_ELF", "BNDR_LAUNCHER_ELF"),
    ("BNDROID_APP_ELF", "BNDR_APP_ELF"),
    ("BNDROID_INPUT_SERVER_ELF", "BNDR_INPUT_SERVER_ELF"),
    ("BNDROID_STORAGE_SERVER_ELF", "BNDR_STORAGE_SERVER_ELF"),
    ("BNDROID_ANDROID_APP_ELF", "BNDR_ANDROID_APP_ELF"),
];

const IMAGE_BINARIES: [(&str, &str); 10] = [
    ("bndroid-init", "bndroid-init.rs"),
    ("bndroid-service-manager", "bndroid-service-manager.rs"),
    ("bndroid-echo-provider", "bndroid-echo-provider.rs"),
    ("bndroid-echo-client", "bndroid-echo-client.rs"),
    ("bndroid-surface-server", "bndroid-surface-server.rs"),
    ("bndroid-launcher", "bndroid-launcher.rs"),
    ("bndroid-app", "bndroid-app.rs"),
    ("bndroid-input-server", "bndroid-input-server.rs"),
    ("bndroid-storage-server", "bndroid-storage-server.rs"),
    ("bndroid-android-app", "bndroid-android-app.rs"),
];

const INPUT_SERVER_IMAGE_INDEX: usize = 7;
const STORAGE_SERVER_IMAGE_INDEX: usize = 8;
const ANDROID_APP_IMAGE_INDEX: usize = 9;

struct EmbeddedImageSources<'a> {
    user_library: &'a Path,
    user_bin_dir: &'a Path,
    linker: &'a Path,
    abi: &'a Path,
    service_manager: &'a Path,
    ui: &'a Path,
    input: &'a Path,
    compositor: &'a Path,
    androidbox: &'a Path,
    app_data: &'a Path,
    storage: &'a Path,
}

fn selected_image_indices() -> Vec<usize> {
    let include_input = env::var_os("CARGO_FEATURE_INPUT_SERVER_RUNTIME").is_some();
    let include_storage = env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some();
    let include_android_app = env::var_os("CARGO_FEATURE_ANDROIDBOX_PROCESS0").is_some();
    (0..IMAGE_ENVIRONMENTS.len())
        .filter(|index| {
            (*index != INPUT_SERVER_IMAGE_INDEX || include_input)
                && (*index != STORAGE_SERVER_IMAGE_INDEX || include_storage)
                && (*index != ANDROID_APP_IMAGE_INDEX || include_android_app)
        })
        .collect()
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_COOPERATIVE_BLOCK_RECOVERY");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_APP_DATA_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_APP_DATA_ASYNC_RECOVERY_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_MOBILE_UI_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_DEX0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_APK_INSTALL0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_EL0_RUNTIME0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_INTERACTIVE0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_PROCESS0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ANDROIDBOX_RESTART0");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_RECOVERY_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_FAULT_POLICY_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_PERSISTENT_HEALTH_RUNTIME");
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME"
    );
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME");
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME"
    );
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME");
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME"
    );
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME"
    );
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME");
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME"
    );
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME");
    println!(
        "cargo:rerun-if-env-changed=CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME"
    );
    println!("cargo:rerun-if-env-changed=BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX");
    println!("cargo:rerun-if-env-changed=BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX");
    println!("cargo:rerun-if-env-changed=BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX");
    println!("cargo:rerun-if-env-changed=BNDROID_M80_TEST_MODE");
    println!("cargo:rerun-if-env-changed=BNDROID_M81_TEST_MODE");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_IRQ_TIMEOUT_SELF_TEST");
    println!("cargo:rustc-check-cfg=cfg(bndroid_storage_irq_timeout_profile)");
    if env::var_os("CARGO_FEATURE_STORAGE_IRQ_TIMEOUT_SELF_TEST").is_some()
        && env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_none()
        && env::var_os("CARGO_FEATURE_APP_DATA_RUNTIME").is_none()
    {
        println!("cargo:rustc-cfg=bndroid_storage_irq_timeout_profile");
    }
    let linker_script = manifest_dir.join("linker.ld");
    println!("cargo:rerun-if-changed={}", linker_script.display());
    println!(
        "cargo:rustc-link-arg-bin=bndroid-kernel=-T{}",
        linker_script.display()
    );
    let kernel_bss_limit = if env::var_os("CARGO_FEATURE_ANDROIDBOX_INTERACTIVE0").is_some() {
        // Interactive-0 adds one fixed 4,608,000-byte SurfaceServer chrome
        // backing. Keep that opt-in profile inside an explicit 24 MiB
        // non-executable BSS window without relaxing predecessor profiles.
        "0x41e00000"
    } else if env::var_os("CARGO_FEATURE_MOBILE_UI_RUNTIME").is_some()
        && env::var_os("CARGO_FEATURE_ANDROIDBOX_APK_INSTALL0").is_none()
    {
        // The mapped mobile frame transaction owns four fixed 4,608,000-byte
        // backings (two each for Launcher and App). Keep this exact opt-in
        // profile inside a 34 MiB RW/NX window; all copy-backed and historical
        // profiles retain their narrower limits.
        "0x42200000"
    } else {
        "0x41800000"
    };
    println!(
        "cargo:rustc-link-arg-bin=bndroid-kernel=--defsym=__kernel_bss_limit={kernel_bss_limit}"
    );
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME").is_some() {
        configure_verified_manifest_artifact(&manifest_dir);
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME").is_some() {
        configure_maintenance_authorization_artifact(&manifest_dir);
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME").is_some() {
        configure_maintenance_plan_artifact(&manifest_dir);
    }

    if env::var("TARGET").as_deref() != Ok("aarch64-unknown-none") {
        return;
    }

    let workspace = manifest_dir
        .parent()
        .expect("kernel manifest must be inside the workspace");
    let user_dir = workspace.join("user/init");
    let user_library = user_dir.join("src/lib.rs");
    let user_implementation = user_dir.join("src/main.rs");
    let user_bin_dir = user_dir.join("src/bin");
    let user_linker = user_dir.join("linker.ld");
    let abi_source = workspace.join("crates/bndr-abi/src/lib.rs");
    let service_manager_source = workspace.join("crates/bndr-sm/src/lib.rs");
    let service_manifest_source = workspace.join("crates/bndr-sm/src/manifest.rs");
    let verified_manifest_source = workspace.join("crates/bndr-sm/src/verified_manifest.rs");
    let maintenance_plan_source = workspace.join("crates/bndr-sm/src/maintenance_plan.rs");
    let ui_source = workspace.join("crates/bndr-ui/src/lib.rs");
    let ui_mobile_source = workspace.join("crates/bndr-ui/src/mobile.rs");
    let input_source = workspace.join("crates/bndr-input/src/lib.rs");
    let input_protocol_source = workspace.join("crates/bndr-input/src/protocol.rs");
    let input_router_source = workspace.join("crates/bndr-input/src/router.rs");
    let compositor_source = workspace.join("crates/bndr-compositor/src/lib.rs");
    let androidbox_source = workspace.join("crates/bndr-androidbox/src/lib.rs");
    let androidbox_dex_source = workspace.join("crates/bndr-androidbox/src/dex.rs");
    let androidbox_digest_source = workspace.join("crates/bndr-androidbox/src/digest.rs");
    let androidbox_manifest_source = workspace.join("crates/bndr-androidbox/src/manifest.rs");
    let androidbox_resources_source = workspace.join("crates/bndr-androidbox/src/resources.rs");
    let androidbox_signature_source = workspace.join("crates/bndr-androidbox/src/signature.rs");
    let androidbox_zip_source = workspace.join("crates/bndr-androidbox/src/zip.rs");
    let androidbox_inline_fixture = workspace.join("fixtures/androidbox-demo/androidbox-demo.apk");
    let androidbox_resource_fixture =
        workspace.join("fixtures/androidbox-resource-demo/androidbox-resource-demo.apk");
    let app_data_source = workspace.join("crates/bndr-appdata/src/lib.rs");
    let storage_source = workspace.join("crates/bndr-storage/src/lib.rs");
    let input_server_runtime = user_dir.join("src/input_server_runtime.rs");
    let androidbox_runtime = user_dir.join("src/androidbox_runtime.rs");
    let androidapp_process = user_dir.join("src/androidapp_process.rs");
    let lifecycle_runtime = user_dir.join("src/lifecycle_runtime.rs");
    let multi_window_runtime = user_dir.join("src/multi_window_runtime.rs");
    let storage_server_runtime = user_dir.join("src/storage_server_runtime.rs");
    let resident_shutdown_runtime = user_dir.join("src/resident_shutdown_runtime.rs");

    let image_indices = selected_image_indices();
    let environments: Vec<_> = image_indices
        .iter()
        .map(|index| IMAGE_ENVIRONMENTS[*index])
        .collect();
    let binaries: Vec<_> = image_indices
        .iter()
        .map(|index| IMAGE_BINARIES[*index])
        .collect();
    for (external, _) in &environments {
        println!("cargo:rerun-if-env-changed={external}");
    }
    for source in [
        user_library.as_path(),
        user_implementation.as_path(),
        user_linker.as_path(),
        abi_source.as_path(),
        service_manager_source.as_path(),
        service_manifest_source.as_path(),
        verified_manifest_source.as_path(),
        maintenance_plan_source.as_path(),
        ui_source.as_path(),
        ui_mobile_source.as_path(),
        input_source.as_path(),
        input_protocol_source.as_path(),
        input_router_source.as_path(),
        compositor_source.as_path(),
        androidbox_source.as_path(),
        androidbox_dex_source.as_path(),
        androidbox_digest_source.as_path(),
        androidbox_manifest_source.as_path(),
        androidbox_resources_source.as_path(),
        androidbox_signature_source.as_path(),
        androidbox_zip_source.as_path(),
        androidbox_inline_fixture.as_path(),
        androidbox_resource_fixture.as_path(),
        androidbox_runtime.as_path(),
        androidapp_process.as_path(),
        input_server_runtime.as_path(),
        lifecycle_runtime.as_path(),
        multi_window_runtime.as_path(),
        resident_shutdown_runtime.as_path(),
    ] {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some() {
        for source in [
            app_data_source.as_path(),
            storage_source.as_path(),
            storage_server_runtime.as_path(),
        ] {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }
    for (_, source_name) in &binaries {
        println!(
            "cargo:rerun-if-changed={}",
            user_bin_dir.join(source_name).display()
        );
    }

    let overrides: Vec<_> = environments
        .iter()
        .map(|(external, _)| env::var_os(external))
        .collect();
    for override_path in overrides.iter().flatten() {
        println!(
            "cargo:rerun-if-changed={}",
            Path::new(override_path).display()
        );
    }
    let override_count = overrides.iter().filter(|value| value.is_some()).count();
    let images = if override_count == 0 {
        build_embedded_images(
            &EmbeddedImageSources {
                user_library: &user_library,
                user_bin_dir: &user_bin_dir,
                linker: &user_linker,
                abi: &abi_source,
                service_manager: &service_manager_source,
                ui: &ui_source,
                input: &input_source,
                compositor: &compositor_source,
                androidbox: &androidbox_source,
                app_data: &app_data_source,
                storage: &storage_source,
            },
            &binaries,
        )
    } else if override_count == environments.len() {
        (0..environments.len())
            .map(|index| {
                canonical_existing(
                    Path::new(overrides[index].as_deref().unwrap_or_else(|| {
                        panic!("complete userspace override set became partial")
                    })),
                    environments[index].0,
                )
            })
            .collect()
    } else {
        panic!(
            "userspace ELF overrides must be all-or-none; received {override_count} of {}",
            environments.len()
        );
    };
    reject_identical_images(&images);
    for (index, (_, internal)) in environments.iter().enumerate() {
        println!("cargo:rustc-env={internal}={}", images[index].display());
    }
}

fn canonical_existing(path: &Path, label: &str) -> PathBuf {
    path.canonicalize().unwrap_or_else(|error| {
        panic!("{label}={} is not a readable file: {error}", path.display())
    })
}

fn reject_identical_images(images: &[PathBuf]) {
    let contents: Vec<Vec<u8>> = images
        .iter()
        .map(|path| {
            fs::read(path).unwrap_or_else(|error| {
                panic!("cannot read userspace ELF {}: {error}", path.display())
            })
        })
        .collect();
    for left in 0..contents.len() {
        if contents[left].is_empty() {
            panic!("userspace ELF {} is empty", images[left].display());
        }
        for right in (left + 1)..contents.len() {
            if contents[left] == contents[right] {
                panic!(
                    "userspace ELFs must be pairwise distinct: {} and {}",
                    images[left].display(),
                    images[right].display()
                );
            }
        }
    }
}

fn build_embedded_images(
    sources: &EmbeddedImageSources<'_>,
    binaries: &[(&str, &str)],
) -> Vec<PathBuf> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    prepare_mobile_font_asset(
        &sources
            .ui
            .parent()
            .unwrap_or_else(|| panic!("userspace UI source lacked a parent directory"))
            .join("mobile_font_alpha4.b64"),
        &out_dir.join("mobile_font_alpha4.bin"),
    );
    let abi_output = out_dir.join("libbndr_abi.rlib");
    let service_manager_output = out_dir.join("libbndr_sm.rlib");
    let ui_output = out_dir.join("libbndr_ui.rlib");
    let input_output = out_dir.join("libbndr_input.rlib");
    let compositor_output = out_dir.join("libbndr_compositor.rlib");
    let androidbox_output = out_dir.join("libbndr_androidbox.rlib");
    let app_data_output = out_dir.join("libbndr_appdata.rlib");
    let storage_output = out_dir.join("libbndr_storage.rlib");
    let user_library_output = out_dir.join("libbndroid_init.rlib");
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());

    let mut abi_command = Command::new(&rustc);
    abi_command
        .arg("--crate-name")
        .arg("bndr_abi")
        .arg("--crate-type")
        .arg("rlib")
        .arg("--edition=2024")
        .arg("--target")
        .arg("aarch64-unknown-none")
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("opt-level=s")
        .arg("-o")
        .arg(&abi_output)
        .arg(sources.abi);
    if env::var_os("CARGO_FEATURE_MOBILE_UI_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"mobile-ui-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_APK_INSTALL0").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-apk-install0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_EL0_RUNTIME0").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-el0-runtime0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_INTERACTIVE0").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-interactive0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_PROCESS0").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-process0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_RESTART0").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-restart0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_SCENE_RPC2").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"androidbox-scene-rpc2\"");
    }
    if env::var_os("CARGO_FEATURE_APP_DATA_RUNTIME").is_some() {
        abi_command.arg("--cfg").arg("feature=\"app-data-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"storage-server-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"storage-server-shutdown-orchestration-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"resident-platform-shutdown-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_LIVENESS_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-liveness-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-multiservice-liveness-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-psci-shutdown-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-continuous-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-manifest-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-event-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-verified-manifest-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-persistent-rollback-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-key-rotation-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-authorization-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-execution-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_STEP_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-step-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_PLAN_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-plan-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME").is_some() {
        abi_command
            .arg("--cfg")
            .arg("feature=\"unified-product-signed-maintenance-plan-runtime\"");
    }
    run_rustc(&mut abi_command, "userspace ABI dependency");
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some() {
        run_rustc(
            Command::new(&rustc)
                .arg("--crate-name")
                .arg("bndr_appdata")
                .arg("--crate-type")
                .arg("rlib")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("-C")
                .arg("codegen-units=1")
                .arg("-o")
                .arg(&app_data_output)
                .arg(sources.app_data),
            "userspace AppData dependency",
        );
        run_rustc(
            Command::new(&rustc)
                .arg("--crate-name")
                .arg("bndr_storage")
                .arg("--crate-type")
                .arg("rlib")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("-C")
                .arg("codegen-units=1")
                .arg("-o")
                .arg(&storage_output)
                .arg(sources.storage),
            "userspace storage protocol dependency",
        );
    }
    run_rustc(
        Command::new(&rustc)
            .arg("--crate-name")
            .arg("bndr_sm")
            .arg("--crate-type")
            .arg("rlib")
            .arg("--edition=2024")
            .arg("--target")
            .arg("aarch64-unknown-none")
            .arg("-C")
            .arg("panic=abort")
            .arg("-C")
            .arg("opt-level=s")
            .arg("--extern")
            .arg(format!("bndr_abi={}", abi_output.display()))
            .arg("-L")
            .arg(format!("dependency={}", out_dir.display()))
            .arg("-o")
            .arg(&service_manager_output)
            .arg(sources.service_manager),
        "userspace ServiceManager dependency",
    );
    let mut ui_command = Command::new(&rustc);
    ui_command
        .arg("--crate-name")
        .arg("bndr_ui")
        .arg("--crate-type")
        .arg("rlib")
        .arg("--edition=2024")
        .arg("--target")
        .arg("aarch64-unknown-none")
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("opt-level=s")
        .arg("-o")
        .arg(&ui_output)
        .arg(sources.ui);
    if env::var_os("CARGO_FEATURE_MOBILE_UI_RUNTIME").is_some() {
        ui_command.arg("--cfg").arg("feature=\"mobile-ui-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_EL0_RUNTIME0").is_some() {
        ui_command
            .arg("--cfg")
            .arg("feature=\"androidbox-el0-runtime0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_INTERACTIVE0").is_some() {
        ui_command
            .arg("--cfg")
            .arg("feature=\"androidbox-interactive0\"")
            .arg("--cfg")
            .arg("feature=\"mobile-system-chrome0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_PROCESS0").is_some() {
        ui_command
            .arg("--cfg")
            .arg("feature=\"androidbox-process0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_RESTART0").is_some() {
        ui_command
            .arg("--cfg")
            .arg("feature=\"androidbox-restart0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_SCENE_RPC2").is_some() {
        ui_command
            .arg("--cfg")
            .arg("feature=\"androidbox-scene-rpc2\"");
    }
    run_rustc(&mut ui_command, "userspace UI protocol dependency");
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_DEX0").is_some() {
        run_rustc(
            Command::new(&rustc)
                .arg("--crate-name")
                .arg("bndr_androidbox")
                .arg("--crate-type")
                .arg("rlib")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("-C")
                .arg("codegen-units=1")
                .arg("--extern")
                .arg(format!("bndr_sm={}", service_manager_output.display()))
                .arg("-L")
                .arg(format!("dependency={}", out_dir.display()))
                .arg("-o")
                .arg(&androidbox_output)
                .arg(sources.androidbox),
            "userspace AndroidBox DEX-0 dependency",
        );
    }
    if env::var_os("CARGO_FEATURE_INPUT_SERVER_RUNTIME").is_some() {
        run_rustc(
            Command::new(&rustc)
                .arg("--crate-name")
                .arg("bndr_compositor")
                .arg("--crate-type")
                .arg("rlib")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("--extern")
                .arg(format!("bndr_ui={}", ui_output.display()))
                .arg("-L")
                .arg(format!("dependency={}", out_dir.display()))
                .arg("-o")
                .arg(&compositor_output)
                .arg(sources.compositor),
            "userspace compositor dependency",
        );
        run_rustc(
            Command::new(&rustc)
                .arg("--crate-name")
                .arg("bndr_input")
                .arg("--crate-type")
                .arg("rlib")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("--extern")
                .arg(format!("bndr_abi={}", abi_output.display()))
                .arg("--extern")
                .arg(format!("bndr_ui={}", ui_output.display()))
                .arg("-L")
                .arg(format!("dependency={}", out_dir.display()))
                .arg("-o")
                .arg(&input_output)
                .arg(sources.input),
            "userspace input protocol dependency",
        );
    }
    let mut user_library_command = Command::new(&rustc);
    user_library_command
        .arg("--crate-name")
        .arg("bndroid_init")
        .arg("--crate-type")
        .arg("rlib")
        .arg("--edition=2024")
        .arg("--target")
        .arg("aarch64-unknown-none")
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("opt-level=s")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("--extern")
        .arg(format!("bndr_abi={}", abi_output.display()))
        .arg("--extern")
        .arg(format!("bndr_sm={}", service_manager_output.display()))
        .arg("--extern")
        .arg(format!("bndr_ui={}", ui_output.display()))
        .arg("-L")
        .arg(format!("dependency={}", out_dir.display()));
    if env::var_os("CARGO_FEATURE_INPUT_SERVER_RUNTIME").is_some() {
        user_library_command
            .arg("--extern")
            .arg(format!("bndr_input={}", input_output.display()))
            .arg("--extern")
            .arg(format!("bndr_compositor={}", compositor_output.display()));
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some() {
        user_library_command
            .arg("--extern")
            .arg(format!("bndr_appdata={}", app_data_output.display()))
            .arg("--extern")
            .arg(format!("bndr_storage={}", storage_output.display()));
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_DEX0").is_some() {
        user_library_command
            .arg("--extern")
            .arg(format!("bndr_androidbox={}", androidbox_output.display()));
    }
    add_userspace_feature_cfgs(&mut user_library_command);
    user_library_command
        .arg("-o")
        .arg(&user_library_output)
        .arg(sources.user_library);
    run_rustc(&mut user_library_command, "shared userspace runtime");

    binaries
        .iter()
        .map(|&(binary_name, source_name)| {
            let output = out_dir.join(format!("{binary_name}.elf"));
            let mut command = Command::new(&rustc);
            command
                .arg("--crate-name")
                .arg(binary_name.replace('-', "_"))
                .arg("--crate-type")
                .arg("bin")
                .arg("--edition=2024")
                .arg("--target")
                .arg("aarch64-unknown-none")
                .arg("-C")
                .arg("panic=abort")
                .arg("-C")
                .arg("opt-level=s")
                .arg("-C")
                .arg("codegen-units=1")
                .arg("-C")
                .arg("relocation-model=static")
                .arg("--extern")
                .arg(format!("bndroid_init={}", user_library_output.display()))
                .arg("-L")
                .arg(format!("dependency={}", out_dir.display()))
                .arg("-C")
                .arg(format!("link-arg=-T{}", sources.linker.display()))
                .arg("-C")
                .arg("link-arg=--build-id=none")
                .arg("-C")
                .arg("link-arg=-static")
                .arg("-C")
                .arg("link-arg=-z")
                .arg("-C")
                .arg("link-arg=max-page-size=4096")
                .arg("-C")
                .arg("link-arg=-z")
                .arg("-C")
                .arg("link-arg=common-page-size=4096")
                .arg("-C")
                .arg("link-arg=--gc-sections")
                .arg("-C")
                .arg("link-arg=--fatal-warnings")
                .arg("-o")
                .arg(&output)
                .arg(sources.user_bin_dir.join(source_name));
            add_userspace_feature_cfgs(&mut command);
            run_rustc(&mut command, binary_name);
            canonical_existing(&output, binary_name)
        })
        .collect()
}

fn prepare_mobile_font_asset(source: &Path, destination: &Path) {
    const EXPECTED_BYTES: usize = 126_023;
    const EXPECTED_FNV1A64: u64 = 0x7794_4e20_f75a_453f;

    let encoded = fs::read(source)
        .unwrap_or_else(|error| panic!("reading mobile font atlas failed: {error}"));
    let mut decoded = Vec::with_capacity(EXPECTED_BYTES);
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;
    for byte in encoded {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("mobile font atlas contained invalid base64"),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            decoded.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    let digest = decoded
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    if decoded.len() != EXPECTED_BYTES || digest != EXPECTED_FNV1A64 {
        panic!("mobile font atlas failed its generated size/digest contract");
    }
    fs::write(destination, decoded)
        .unwrap_or_else(|error| panic!("writing embedded mobile font atlas failed: {error}"));
}

fn run_rustc(command: &mut Command, label: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to invoke rustc for {label}: {error}"));
    assert!(status.success(), "building {label} failed");
}

fn add_userspace_feature_cfgs(command: &mut Command) {
    if env::var_os("CARGO_FEATURE_MOBILE_UI_RUNTIME").is_some() {
        command.arg("--cfg").arg("feature=\"mobile-ui-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_DEX0").is_some() {
        command.arg("--cfg").arg("feature=\"androidbox-dex0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_APK_INSTALL0").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"androidbox-apk-install0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_EL0_RUNTIME0").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"androidbox-el0-runtime0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_INTERACTIVE0").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"androidbox-interactive0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_PROCESS0").is_some() {
        command.arg("--cfg").arg("feature=\"androidbox-process0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_RESTART0").is_some() {
        command.arg("--cfg").arg("feature=\"androidbox-restart0\"");
    }
    if env::var_os("CARGO_FEATURE_ANDROIDBOX_SCENE_RPC2").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"androidbox-scene-rpc2\"");
    }
    if env::var_os("CARGO_FEATURE_UI_STALE_PRESENT_EVIDENCE").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"ui-stale-present-evidence\"");
    }
    if env::var_os("CARGO_FEATURE_INPUT_SERVER_RUNTIME").is_some() {
        for feature in [
            "app-lifecycle-runtime",
            "mapped-graphics-runtime",
            "graphics-frame-clock-runtime",
            "multi-window-runtime",
            "persistent-window-runtime",
            "text-input-runtime",
            "soft-keyboard-runtime",
            "input-server-runtime",
        ] {
            command.arg("--cfg").arg(format!("feature=\"{feature}\""));
        }
        if env::var_os("CARGO_FEATURE_INPUT_SERVER_SURFACE_RESTART_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"input-server-surface-restart-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_INPUT_SERVER_RESTART_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"input-server-restart-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_SERVICE_SUPERVISOR_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"service-supervisor-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_SERVICE_DEPENDENCY_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"service-dependency-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_POST_RECOVERY_INTERACTION_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"post-recovery-interaction-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_POST_RECOVERY_FOCUS_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"post-recovery-focus-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_POST_RECOVERY_FOCUS_ROUNDTRIP_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"post-recovery-focus-roundtrip-runtime\"");
        }
        if env::var_os("CARGO_FEATURE_POST_RECOVERY_LIFECYCLE_FOCUS_RUNTIME").is_some() {
            command
                .arg("--cfg")
                .arg("feature=\"post-recovery-lifecycle-focus-runtime\"");
        }
    }
    if env::var_os("CARGO_FEATURE_APP_DATA_RUNTIME").is_some() {
        command.arg("--cfg").arg("feature=\"app-data-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_APP_DATA_ASYNC_RECOVERY_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"app-data-async-recovery-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_RECOVERY_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-recovery-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-repeated-recovery-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-async-recovery-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_FAULT_POLICY_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-fault-policy-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-owner-liveness-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_TERMINAL_QUARANTINE_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-terminal-quarantine-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"storage-server-shutdown-orchestration-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_RESIDENT_PLATFORM_SHUTDOWN_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"resident-platform-shutdown-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_LIVENESS_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-liveness-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-multiservice-liveness-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_PSCI_SHUTDOWN_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-psci-shutdown-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-continuous-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MANIFEST_SUPERVISION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-manifest-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_EVENT_SUPERVISION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-event-supervision-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_VERIFIED_MANIFEST_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-verified-manifest-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-persistent-rollback-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-key-rotation-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_AUTHORIZATION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-authorization-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-execution-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_STEP_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-step-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_MAINTENANCE_PLAN_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-maintenance-plan-runtime\"");
    }
    if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME").is_some() {
        command
            .arg("--cfg")
            .arg("feature=\"unified-product-signed-maintenance-plan-runtime\"");
    }
}

fn configure_verified_manifest_artifact(manifest_dir: &Path) {
    const EXPECTED_BYTES: usize = 512;

    let workspace = manifest_dir
        .parent()
        .expect("kernel manifest must be inside the workspace");
    let source = env::var_os("BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_KEY_ROTATION_RUNTIME").is_some() {
                workspace.join("boot/product-service-manifest-v5-key4.bms1.hex")
            } else if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_RUNTIME")
                .is_some()
            {
                workspace.join("boot/product-service-manifest-v3.bms1.hex")
            } else {
                workspace.join("boot/product-service-manifest-v2.bms1.hex")
            }
        });
    let source = canonical_existing(&source, "BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX");
    println!("cargo:rerun-if-changed={}", source.display());
    let encoded = fs::read(&source).unwrap_or_else(|error| {
        panic!(
            "cannot read verified-manifest artifact {}: {error}",
            source.display()
        )
    });
    let mut decoded = Vec::with_capacity(EXPECTED_BYTES);
    let mut high_nibble = None;
    for byte in encoded {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!(
                "verified-manifest artifact {} contains non-hexadecimal input",
                source.display()
            ),
        };
        if let Some(high) = high_nibble.take() {
            decoded.push((high << 4) | nibble);
        } else {
            high_nibble = Some(nibble);
        }
    }
    if high_nibble.is_some() || decoded.len() != EXPECTED_BYTES {
        panic!(
            "verified-manifest artifact {} decoded to {} bytes, expected {EXPECTED_BYTES}",
            source.display(),
            decoded.len()
        );
    }
    let output =
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("product-service-manifest.bms1");
    fs::write(&output, decoded).unwrap_or_else(|error| {
        panic!(
            "cannot materialize verified-manifest artifact {}: {error}",
            output.display()
        )
    });
    println!(
        "cargo:rustc-env=BNDR_VERIFIED_MANIFEST_ARTIFACT={}",
        output.display()
    );
}

fn configure_maintenance_authorization_artifact(manifest_dir: &Path) {
    const EXPECTED_BYTES: usize = 512;

    let workspace = manifest_dir
        .parent()
        .expect("kernel manifest must be inside the workspace");
    let source = env::var_os("BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if env::var_os("CARGO_FEATURE_UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_RUNTIME")
                .is_some()
            {
                workspace.join("boot/maintenance-authorization-sequence2.bma1.hex")
            } else {
                workspace.join("boot/maintenance-authorization-sequence1.bma1.hex")
            }
        });
    let source = canonical_existing(&source, "BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX");
    println!("cargo:rerun-if-changed={}", source.display());
    let encoded = fs::read(&source).unwrap_or_else(|error| {
        panic!(
            "cannot read maintenance-authorization artifact {}: {error}",
            source.display()
        )
    });
    let mut decoded = Vec::with_capacity(EXPECTED_BYTES);
    let mut high_nibble = None;
    for byte in encoded {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!(
                "maintenance-authorization artifact {} contains non-hexadecimal input",
                source.display()
            ),
        };
        if let Some(high) = high_nibble.take() {
            decoded.push((high << 4) | nibble);
        } else {
            high_nibble = Some(nibble);
        }
    }
    if high_nibble.is_some() || decoded.len() != EXPECTED_BYTES {
        panic!(
            "maintenance-authorization artifact {} decoded to {} bytes, expected {EXPECTED_BYTES}",
            source.display(),
            decoded.len()
        );
    }
    let output =
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("maintenance-authorization.bma1");
    fs::write(&output, decoded).unwrap_or_else(|error| {
        panic!(
            "cannot materialize maintenance-authorization artifact {}: {error}",
            output.display()
        )
    });
    println!(
        "cargo:rustc-env=BNDR_MAINTENANCE_AUTHORIZATION_ARTIFACT={}",
        output.display()
    );
}

fn configure_maintenance_plan_artifact(manifest_dir: &Path) {
    const EXPECTED_BYTES: usize = 512;

    let workspace = manifest_dir
        .parent()
        .expect("kernel manifest must be inside the workspace");
    let source = env::var_os("BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("boot/maintenance-plan-sequence2.bmp1.hex"));
    let source = canonical_existing(&source, "BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX");
    println!("cargo:rerun-if-changed={}", source.display());
    let encoded = fs::read(&source).unwrap_or_else(|error| {
        panic!(
            "cannot read maintenance-plan artifact {}: {error}",
            source.display()
        )
    });
    let mut decoded = Vec::with_capacity(EXPECTED_BYTES);
    let mut high_nibble = None;
    for byte in encoded {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!(
                "maintenance-plan artifact {} contains non-hexadecimal input",
                source.display()
            ),
        };
        if let Some(high) = high_nibble.take() {
            decoded.push((high << 4) | nibble);
        } else {
            high_nibble = Some(nibble);
        }
    }
    if high_nibble.is_some() || decoded.len() != EXPECTED_BYTES {
        panic!(
            "maintenance-plan artifact {} decoded to {} bytes, expected {EXPECTED_BYTES}",
            source.display(),
            decoded.len()
        );
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("maintenance-plan.bmp1");
    fs::write(&output, decoded).unwrap_or_else(|error| {
        panic!(
            "cannot materialize maintenance-plan artifact {}: {error}",
            output.display()
        )
    });
    println!(
        "cargo:rustc-env=BNDR_MAINTENANCE_PLAN_ARTIFACT={}",
        output.display()
    );
}
