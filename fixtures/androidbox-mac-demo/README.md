# Mac-built Android APK fixture

This directory owns a new Android application built locally on the Mac with
the already-installed Android SDK. Its manifest package is
`org.bndroid.macdemo`, its launcher is
`org.bndroid.macdemo.MainActivity`, its version code is `1`, and its compiled
resource-backed `TextView` displays `Hello from a Mac-built APK`.

Build it offline from this repository with:

```sh
./fixtures/androidbox-mac-demo/build.sh
```

The script resolves Android SDK 36.1.0 tools and `android-36/android.jar` by
absolute path, compiles the Java and resources from this directory, creates
exactly four STORED and aligned APK entries, and signs the same unsigned APK
twice to prove deterministic APK Signature Scheme v2 output. It then validates
the one-signer RSA-2048/SHA-256 profile, binary manifest badging, compiled
resource IDs and strings, binary `TextView` layout, DEX descriptors, entry
set, alignment, and the 65,024-byte package-store limit.

This APK deliberately contains no
`Lorg/bndroid/demo/Main;`, `boot()`, or `onTap(int)` compatibility probe.
AndroidBox admits it from the unique exported `MAIN`/`LAUNCHER` component in
the binary manifest, binds that exact descriptor to
`Lorg/bndroid/macdemo/MainActivity;` in `classes.dex`, interprets its exact
public no-argument constructor, and then executes its bounded `onCreate`. The
build fails if the retired fixed probe class reappears.

No signing material is copied into this directory. The build references the
repository-public **test-only** key and certificate in
`../androidbox-resource-demo/`. That key is not a product trust anchor, grants
no operating-system authority, and must never sign production or generally
distributed applications.

The generated artifact is 12,566 bytes. Its SHA-256 is
`a604d16298e939d728aa805a636bb119c906bc1b9c224949da445954a4f8363e`,
also pinned by `androidbox-mac-demo.apk.sha256`. Its test certificate SHA-256
is `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

This fixture proves only the repository's narrow Resources-1 compatibility
profile. The ABI-44 local-QEMU gate installs it, recovers it without an APK
source, launches it from All apps with request sequence 1, and activates its
capacity-one compatible recent with request sequence 2. Each entry freshly
rereads and reverifies the durable APK, executes the exact two-instruction
constructor followed by the four-instruction `onCreate`, and displays
`Hello from a Mac-built APK` with `reads=389 writes=0 flushes=0`; the
whole-disk SHA-256 remains unchanged.

`CompatibleActivitySession-0` is completed in
`../../target/androidbox-local-apk/check.4rO29p/`. Its identity is boot-local
and capacity one. Home retains only that identity and performs no background
execution. Overview displays system-owned identity copy but no Activity
pixels, thumbnail, screenshot, or live preview. Selecting the recent performs
another fresh syscall 60 pass. Back finishes the session and clears the recent,
so the following Overview is empty and its card is inert.

The exact control sequence is `Reserve → fresh verify → commit or abort`.
`BUC1` v8 carries the request and `BUE1` v6 explicitly acknowledges
`accepted`, `conflict`, or `capture-busy`; only `accepted` permits the
subsequent bounded step. `BUP1` v2 binds a mobile present to a nonzero
`system_ui_revision`, and SurfaceServer rejects stale raster content.

The two Activity PPMs are byte-identical at SHA-256
`e1a68c2627692d3f1eb31042cdaaaefa0c3f6be43ef44226bcbe2540a31d9c84`.
The identity-only compatible Overview SHA-256 is
`ff4012ba8d990bb6defc2944fc44021679ce5da72835d456a5446589f610e500`;
the empty Overview before and after its inert tap is
`19363d900d0a489676b1ca24ada784a167609a626fd899a3b263d3ccd3bcef44`.

The opt-in ABI-45 child is separately covered by
`../../scripts/check-androidbox-el0-runtime0.sh`; its completed evidence is in
`../../target/androidbox-el0-runtime0/check.R6tuKT/`. It performs two one-shot
App-only syscall-61 claims of an immutable `READ` VMO, rechecks and executes
the admitted subset in the resident App EL0 process, and proves that Activity
pixels come from App while Home/Overview pixels come from Launcher. The two
Activity rasters remain byte-identical, Back clears the recent, and the whole
package disk remains unchanged.

The ABI-44 result above is an EL1 pure-data Resources-1 execution; the ABI-45
child moves bounded foreground re-verification/execution and Activity raster
ownership to the existing App EL0 host. Neither is a standalone Android
application process or evidence of ART/Dalvik, ActivityThread, Binder, Bionic,
JNI, native-library loading, Android permissions, Android networking, general
framework behavior, general APK or Android application compatibility,
background execution, phone hardware, or a real-device/real-phone port.

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
permissions=0 general_apk_claim=0 android_compatibility_claim=0
standalone_android_process=0 compatible_activity_session=1
compatible_activity_capacity=1 compatible_activity_persistence=boot-local
compatible_activity_background_execution=0 overview_activity_pixels=0
overview_thumbnail=0 overview_live_preview=0 real_phone_claim=0
```
