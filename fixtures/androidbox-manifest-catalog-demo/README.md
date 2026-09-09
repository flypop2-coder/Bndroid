# AndroidBox ManifestCatalog-3 SDK fixture

This fixture is built offline with the Android SDK already installed on the
development Mac. It is not hand-assembled binary XML and does not require an
AOSP checkout.

The signed APK declares two Activities, one Activity alias, one Service, one
BroadcastReceiver, one ContentProvider, two launcher entries, four intent
filters, and two requested permissions. Its `MainActivity` uses a real
resource-backed two-button listener flow supported by the current bounded
AndroidBox Activity profile. Every other component is metadata evidence;
parsing a declaration does not grant a permission or imply framework/runtime
support.

Rebuild it deterministically:

```sh
./fixtures/androidbox-manifest-catalog-demo/build.sh
```

The checked-in ABI54 fixture remains icon-free and byte-identical. ABI56's
Icon Resources-5 gate sets `BNDROID_ANDROID_ICON_RESOURCES=1`, which selects
`AndroidManifest.icon.xml`, adds the stored 16×16 RGBA8
`res/drawable/app_icon.png`, and writes only to its target evidence directory.
The PNG CRC32 is `9830e3a8`; the exact version-1 ABI56 APK is 12,640 bytes with
SHA-256
`2410127c7a3b6a9e041d8359b227a4e3968b66bf845bf716f955bca15b2ce21f`.

ABI57's Density Icons-7 gate sets `BNDROID_ANDROID_ICON_RESOURCES=2`. It
compiles the same `@drawable/app_icon` ID with stored
`res/drawable-mdpi-v4/app_icon.png` (48×48, PNG CRC32 `22b3a9c3`) and
`res/drawable-xhdpi-v4/app_icon.png` (96×96, PNG CRC32 `3bf2e092`). The exact
version-1 ABI57 APK is 12,728 bytes with SHA-256
`a85f3c6fb6741a9223fb82b72ada120a434dd881a6c4098d3c066bc63421d621`.
The mdpi/xhdpi pair is used to prove compiled density selection rather than ZIP
order.

ABI58's DEX Methods-8 gate additionally sets
`BNDROID_ANDROID_DEX_METHODS=1`. This selects
`src-methods/org/bndroid/catalog/MainActivity.java`, whose real D8
`onClick(View)` executes exactly one same-Activity
`private static (I)I` helper before the existing `TextView.setText(int)`
mutation:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=1 \
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-methods8/catalog.apk" \
./fixtures/androidbox-manifest-catalog-demo/build.sh
```

The exact ABI58 version-1 APK is 12,728 bytes with SHA-256
`35f21ef72c456699a241eb1c8559e45f2499924a51d956db83cc7316710d51dd`.
Mode `0` remains the historical direct-callback source.

ABI59's DEX Instance-9 gate sets `BNDROID_ANDROID_DEX_METHODS=2`, selecting
`src-instance/org/bndroid/catalog/MainActivity.java`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=2 \
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-instance9/catalog.apk" \
./fixtures/androidbox-manifest-catalog-demo/build.sh
```

The D8 callback passes its Activity receiver and clicked View to one
same-Activity `private (View)I` helper; the helper performs the only
`View.getId()` call. The exact ABI59 version-1 APK is 12,728 bytes with
SHA-256
`fb3626a14ec6d1ec2b69ff012098e894f068c9566b53b75acb00522310cece10`.
Modes `0` and `1` remain unchanged.

ABI60's Activity Fields-10 gate sets `BNDROID_ANDROID_DEX_METHODS=3`, selecting
`src-fields/org/bndroid/catalog/MainActivity.java`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=3 \
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-fields10/catalog.apk" \
./fixtures/androidbox-manifest-catalog-demo/build.sh
```

`onCreate` stores the status `TextView` in one private Activity field through
`iput-object`; `onClick` reads the retained reference through one
`iget-object`, then invokes the ABI59 instance helper. The exact ABI60
version-1 APK remains 12,728 bytes with SHA-256
`f198844798483f5df837cad061ccbbce8140fd0dd2f49b059ad947a64c03ca42`.
Modes `0`, `1`, and `2` remain unchanged.

ABI61's Activity State-11 gate sets `BNDROID_ANDROID_DEX_METHODS=4`, selecting
`src-state/org/bndroid/catalog/MainActivity.java`:

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=4 \
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-state11/catalog.apk" \
./fixtures/androidbox-manifest-catalog-demo/build.sh
```

The Activity adds private `int clickCount` to the ABI60 private TextView.
`onCreate` initializes it to zero; each real D8 callback executes
`iget`, exact `add-int/lit8 +1`, and `iput`, then reads and updates the
TextView. The exact ABI61 version-1 APK remains 12,728 bytes with SHA-256
`3308e2abc20696cbaed1b673ae27647dbd2d7b0cb4522ed3a777297e115332b6`.
Modes `0` through `3` remain unchanged.

ABI62's String Text-12 gate sets `BNDROID_ANDROID_DEX_METHODS=5`, selecting
`src-string/org/bndroid/catalog/MainActivity.java`. Its real D8 callback uses
retained int state, `const-string`, and the exact
`TextView.setText(CharSequence)` overload to display `First review recorded`
and then `Review state advanced again`.

The exact ABI62 version-1 APK remains 12,728 bytes with SHA-256
`760376cd8d4e408f6c08f3c13d5b3b91c0289928fc5240cec8dc8d66418666a9`.
Modes `0` through `4` remain unchanged.

ABI63's String Builder-13 gate sets `BNDROID_ANDROID_DEX_METHODS=6`, selecting
`src-concat/org/bndroid/catalog/MainActivity.java`. Javac/D8 compiles
`"Review count: " + next` into one `new-instance StringBuilder`,
`<init>(String)`, `append(int)`, `toString()`, and `move-result-object` chain
before `TextView.setText(CharSequence)`.

The exact ABI63 version-1 APK remains 12,728 bytes with SHA-256
`41d8c0639080b4b9f55a554811558d35ce4f16745543762758dbd926ae0f9118`.
Modes `0` through `5` remain unchanged.

ABI68's Layout Size-18 gate sets `BNDROID_ANDROID_LAYOUT_ROWS=5`. It selects
`res-size/layout/activity_catalog.xml`, which retains ABI67's directional
spacing and compiles title width `240dp`, row height `120dp`, and Button
heights `64dp/56dp`. The exact version-1 ABI68 APK remains 12,728 bytes with
SHA-256
`6425592a64ab1d4d82796dc888ef0a973d63ddffd9a5f6d474793d537cd60493`.
Modes `0` through `4` remain available for parent-profile regressions.

ABI69's Layout Mixed-19 gate sets `BNDROID_ANDROID_LAYOUT_ROWS=6`. It selects
`res-mixed/layout/activity_catalog.xml`, retaining ABI68's exact sizes and
ABI67's directional spacing while compiling the first Button as fixed
`132dp` with no weight and the second as `0dp + weight=1`. The exact
version-1 ABI69 APK is 12,728 bytes with SHA-256
`d9cad51cd361137ebc6c4e3ffdb10d8b86b964141592971065641656a95e6046`.
Modes `0` through `5` remain available for parent-profile regressions.

The build uses the repository's existing test-only v2 signing identity and
writes only the canonical APK/SHA file under this fixture plus disposable
artifacts under `target/androidbox-manifest-catalog-demo-build`.
