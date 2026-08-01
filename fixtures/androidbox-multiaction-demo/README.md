# AndroidBox MultiActionActivity fixture

This directory owns a deterministic Java APK fixture for a future bounded
AndroidBox multi-action milestone. Build it completely offline with the
Android SDK and JDK already installed on the Mac:

```sh
./fixtures/androidbox-multiaction-demo/build.sh
```

An alternate source-only ABI58 shape can be built without replacing the
canonical APK:

```sh
BNDROID_ANDROID_DEX_METHODS=1 \
BNDROID_MULTIACTION_OUTPUT_APK="$PWD/target/androidbox-dex-methods8/multiaction.apk" \
./fixtures/androidbox-multiaction-demo/build.sh
```

It selects `src-methods/org/bndroid/multiaction/MainActivity.java`, where
`onClick(View)` invokes one same-Activity `private static (I)I` helper. The
alternate APK is 12,573 bytes with SHA-256
`4a37ff5ed5e1f07a803cbca3d6f21d94a2ff201c810fae3b06794d928a5a554a`.
The full ABI58 system gate uses the Envelope and Catalog fixtures; this
alternate MultiAction build is deterministic fixture evidence only.

The package is `org.bndroid.multiaction`; its exported launcher component is
`org.bndroid.multiaction.MainActivity`. The compiled vertical layout contains
exactly two `TextView` elements and two `Button` elements. Both Buttons call
`setOnClickListener(this)`. The real Java `onClick(View)` reads
`view.getId()` and updates the same status `TextView` to either
`Decision: approved` or `Decision: rejected`.

The checked-in output is `androidbox-multiaction-demo.apk`, exactly 12,573
bytes with SHA-256
`aecf0749e2c063f44945f6154b479714fa8ebead8da26b0ebef005d2737036c6`.
Its single test-only signer certificate SHA-256 is
`e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

The build script uses only local Android SDK 36.1.0 tools, `android-36`,
`/usr/bin/javac`, macOS archive/hash tools, and the repository-public
test-only key in `../androidbox-resource-demo/`. It compiles resources and
Java, produces DEX, creates an aligned four-entry STORED APK, signs the same
unsigned APK twice, and requires the two APK Signature Scheme v2 outputs to
be byte-identical. It also validates the manifest, no-permission boundary,
resource/layout shape, DEX call and branch counts, signer certificate,
archive shape, and package-size ceiling.

The signing key is intentionally public test material. It is not a product
trust anchor, grants no operating-system authority, and must never sign a
production or generally distributed application.

The canonical mode-0 fixture is admitted by the earlier MultiAction-3 profile.
The optional mode-1 artifact is source/build evidence; ABI58's independent
QEMU execution proof uses the two package fixtures documented above. Neither
shape proves ART, Dalvik, ActivityThread, Binder, Bionic, JNI, native
libraries, Android Framework compatibility, permissions, services,
networking, arbitrary APK compatibility, physical-device support, or a real
phone.
