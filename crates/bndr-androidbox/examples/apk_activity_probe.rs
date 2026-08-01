use std::{env, fs, process};

use bndr_androidbox::{AndroidBox, AndroidBoxEnvelopeScratch, AndroidBoxManifestCatalogScratch};

fn main() {
    let path = env::args_os().nth(1).unwrap_or_else(|| {
        eprintln!("usage: apk_activity_probe <apk>");
        process::exit(2);
    });
    let apk = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("failed to read {path:?}: {error}");
        process::exit(1);
    });
    let mut envelope_scratch = AndroidBoxEnvelopeScratch::new();
    let mut catalog_scratch = AndroidBoxManifestCatalogScratch::new();
    let image = AndroidBox::load_with_manifest_catalog_scratch(
        &apk,
        &mut envelope_scratch,
        &mut catalog_scratch,
    )
    .unwrap_or_else(|error| {
        eprintln!("failed to load {path:?}: {error:?}");
        process::exit(1);
    });
    let mut session = image.launch_activity_session().unwrap_or_else(|error| {
        eprintln!("failed to launch {path:?}: {error:?}");
        process::exit(1);
    });
    let ids = session.listener_button_ids().to_vec();
    println!(
        "package={} activity={} listeners={}",
        image.manifest_info().package.as_str(),
        image.manifest_info().activity_descriptor.as_str(),
        ids.len()
    );
    for id in ids {
        let update = session.dispatch_click(id).unwrap_or_else(|error| {
            eprintln!("failed to dispatch view {id:#010x}: {error:?}");
            process::exit(1);
        });
        #[cfg(feature = "androidbox-activity-state11")]
        let (field_writes, int_state) = (
            update.activity_field_write_count,
            update.activity_int_state_value,
        );
        #[cfg(not(feature = "androidbox-activity-state11"))]
        let (field_writes, int_state) = (0, 0);
        #[cfg(feature = "androidbox-string-text12")]
        let direct_string_text = update.direct_string_text;
        #[cfg(not(feature = "androidbox-string-text12"))]
        let direct_string_text = false;
        #[cfg(feature = "androidbox-string-builder13")]
        let dynamic_string_text = update.dynamic_string_text;
        #[cfg(not(feature = "androidbox-string-builder13"))]
        let dynamic_string_text = false;
        println!(
            "clicked={:#010x} changed={:#010x} text={:#010x} direct_string={} dynamic_string={} revision={} app_calls={} instance_calls={} field_reads={} field_writes={} int_state={}",
            update.clicked_view_id,
            update.changed_view_id,
            update.text_resource_id,
            direct_string_text,
            dynamic_string_text,
            update.revision,
            update.app_defined_call_count,
            update.app_defined_instance_call_count,
            update.activity_field_read_count,
            field_writes,
            int_state,
        );
    }
}
