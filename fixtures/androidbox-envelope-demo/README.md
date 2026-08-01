# AndroidBox APK Envelope-4 fixture

This directory owns a deterministic Java APK fixture for the opt-in ABI 51
AndroidBox APK Envelope-4 profile. Build it completely offline with the
Android SDK and JDK already installed on the Mac:

```sh
./fixtures/androidbox-envelope-demo/build.sh
```

The historical checked-in ABI51 fixture intentionally remains icon-free and
byte-identical. ABI56's Icon Resources-5 gate builds the alternate real
Manifest/PNG shape without replacing it:

```sh
BNDROID_ANDROID_ICON_RESOURCES=1 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-icon-resources5/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

That opt-in APK adds a stored `res/drawable/app_icon.png` and
`android:icon="@drawable/app_icon"`. The deterministic 16×16 RGBA8 artwork has
PNG CRC32 `f63d0b72`; its exact ABI56 build is 12,717 bytes with SHA-256
`2c2b8fa5e318babd10a0b35114ea7843eb7bdd5cc916e8629784468c850b1a33`.

ABI57's Density Icons-7 gate uses mode `2`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-density-icons7/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

It compiles stored `res/drawable-mdpi-v4/app_icon.png` (48×48, PNG CRC32
`fc4d4dc6`) and `res/drawable-xhdpi-v4/app_icon.png` (96×96, PNG CRC32
`53d44a01`) under the same `@drawable/app_icon` resource ID. The exact ABI57
version-1 APK is 12,805 bytes with SHA-256
`ad7260288e06f12fd2216bfa203c719114eff086b0645ff3cb738cd1d1f05c65`.
Having both configurations proves the runtime selects mdpi by compiled
density metadata rather than ZIP order.

ABI58's DEX Methods-8 gate combines icon mode `2` with a separate Java source
shape:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=1 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-methods8/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

Mode `1` selects
`src-methods/org/bndroid/envelope/MainActivity.java`. Its real D8 callback
executes exactly one `invoke-static` to the same Activity's
`private static int statusTextFor(int)` before the existing
`TextView.setText(int)` call. The ABI58 APK remains 12,805 bytes and has
SHA-256
`767c49d58d53698fefad2d072de52f506130553b3f3011618fab8af6cc912f7b`.
The historical mode `0` source and canonical fixture are not replaced.

ABI59's DEX Instance-9 gate uses mode `2`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=2 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-instance9/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

It selects
`src-instance/org/bndroid/envelope/MainActivity.java`. Its real D8 callback
uses one `invoke-direct {Activity, View}` to a same-Activity
`private int statusTextFor(View)` helper; the helper itself executes the only
`View.getId()` call. The exact ABI59 APK remains 12,805 bytes with SHA-256
`00e653e6c66bfcdfb0616db08888f6b7e18e5b43673b388cb46ed5dca6d17a8b`.
Modes `0` and `1` remain unchanged.

ABI60's Activity Fields-10 gate uses mode `3`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=3 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-fields10/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

It selects `src-fields/org/bndroid/envelope/MainActivity.java`. `onCreate`
stores the layout status `TextView` in one private Activity field through
`iput-object`; `onClick` reads it through one `iget-object` before calling the
ABI59 instance helper. The exact ABI60 version-1 APK remains 12,805 bytes with
SHA-256
`386f69b770306d3ad999f4dd0731f1fafbcd96daab083ea67ee0f044049efd56`.
Modes `0`, `1`, and `2` remain unchanged.

ABI61's Activity State-11 gate uses mode `4`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=4 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-state11/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

It selects `src-state/org/bndroid/envelope/MainActivity.java`. Alongside the
ABI60 private `TextView`, the Activity declares a private `int clickCount`.
`onCreate` writes zero; every real D8 callback executes one
`iget`, exact `add-int/lit8 +1`, and one `iput` before reading the TextView.
The exact ABI61 version-1 APK remains 12,805 bytes with SHA-256
`d3692f30f8dcb14d24ca5a52098f37a2266e6d2474e4687108669ed7a80026a1`.
Modes `0` through `3` remain unchanged.

ABI62's String Text-12 gate uses mode `5`, selecting
`src-string/org/bndroid/envelope/MainActivity.java`. The real D8 callback
increments retained `clickCount` with `const/4 1` plus `add-int/2addr`, then
selects `First envelope action` or `Envelope action repeated` with
`const-string` and calls `TextView.setText(CharSequence)`.

The exact ABI62 version-1 APK remains 12,805 bytes with SHA-256
`b0c133b302ada2e336060259b142c2b7fd27b2d2db587c208553057f35505d2d`.
Modes `0` through `4` remain unchanged.

ABI63's String Builder-13 gate uses mode `6`, selecting
`src-concat/org/bndroid/envelope/MainActivity.java`. Javac/D8 compiles
`"Envelope count: " + next` into one `new-instance StringBuilder`,
`<init>(String)`, `append(int)`, `toString()`, and `move-result-object` chain
before `TextView.setText(CharSequence)`.

The exact ABI63 version-1 APK remains 12,805 bytes with SHA-256
`9a891a3d188e4225803519713c29d02b56eaeebacfafcbba36656193964e2b53`.
Modes `0` through `5` remain unchanged.

ABI68's Layout Size-18 gate sets `BNDROID_ANDROID_LAYOUT_ROWS=5`. It selects
`res-size/layout/activity_envelope.xml`, retaining ABI67's directional
spacing while compiling title width `240dp`, row height `120dp`, and Button
heights `64dp/56dp`. The exact version-1 gate APK is 12,806 bytes with
SHA-256
`3764e8b594b686b12695d031589011eedf9735941783afb2693a8d7a2ab7b117`.
Modes `0` through `4` remain available for parent-profile regressions.

The package is `org.bndroid.envelope`; its exported launcher component is
`org.bndroid.envelope.MainActivity`. The compiled vertical layout contains
two `TextView` elements and two `Button` elements. Its title is
`Envelope review`; both real Buttons bind `View.OnClickListener` and update
the same status view through distinct `view.getId()` branches.

Unlike the earlier all-STORED fixtures, this build starts with the archive
emitted by `aapt2 link`. It does not extract and repackage the compiled binary
XML. The final entry order and methods are:

```text
AndroidManifest.xml                    DEFLATE
res/layout/activity_envelope.xml       DEFLATE
resources.arsc                         STORED
classes.dex                            STORED
assets/envelope-note.txt               DEFLATE
```

The ABI56 opt-in form inserts `res/drawable/app_icon.png` as a STORED entry
immediately after the Manifest. ABI57 mode inserts the mdpi and xhdpi STORED
entries there instead; all historical entries keep their prior method.

`assets/envelope-note.txt` is not referenced by the Manifest, resources,
layout, or Java code and is outside the bounded Activity interpreter. Its ZIP
general-purpose bit 3 is set and a signed 16-byte data descriptor follows its
compressed payload. The SDK/zipalign-produced local CRC/compressed-size/
uncompressed-size tuple remains exact to the central directory; the build does
not rewrite it. The build independently parses local and central headers,
requires that central-exact tuple and the descriptor signature/CRC/sizes to
agree, rejects ZIP64 and encryption, checks four-byte alignment of the two
STORED runtime entries, and proves that aapt2's compiled
Manifest/layout/resource payloads are unchanged.

The checked-in output is `androidbox-envelope-demo.apk`, exactly 12,646 bytes
with SHA-256
`a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5`.
Its adjacent `.sha256` sidecar is generated and verified by the build. Its
single test-only signer certificate SHA-256 is
`e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

The script uses only local Android SDK 36.1.0 tools, `android-36`,
`/usr/bin/javac`, `/usr/bin/jar`, and local macOS archive/hash tools. It keeps
the APK below the 65,024-byte package-store ceiling, requests no Android
permission, uses exactly one `classes.dex`, runs `zipalign`, enables only APK
Signature Scheme v2, and signs the same aligned artifact twice to require
byte-identical output. The ABI 51 gate additionally executes two independent
full fixture builds and compares their signed APK bytes.

The signing key in `../androidbox-resource-demo/` is public test material. It
is not a product trust anchor, grants no operating-system authority, and must
never sign a production or generally distributed application.

This is a bounded archive-envelope and Activity compatibility fixture. The
extra asset being ignored is intentional. It does not prove Android asset
APIs, ART, Dalvik, ActivityThread, Binder, Bionic, JNI, native libraries,
permissions, services, networking, arbitrary APK compatibility, physical
device support, or a real phone.
