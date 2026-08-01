# AndroidBox DEX-0 + ActivityLifecycle-1 demo fixture

This directory owns every source byte used to build `androidbox-demo.apk`.
`scripts/build-androidbox-demo.sh` uses only `/usr/bin/javac`, Android SDK
36.1.0 `d8`/`aapt2`, `android-36/android.jar`, and macOS archive/hash tools.
It does not discover, read, or copy user APKs.

The generated APK has a binary Android manifest with a conventional exported
`MAIN`/`LAUNCHER` activity. It is deliberately unsigned. DEX-0 executes the
actual DEX `code_item` bodies for these pure-static fixed entries:

- `Lorg/bndroid/demo/Main;->boot()I`
- `Lorg/bndroid/demo/Main;->onTap(I)I`

ActivityLifecycle-1 additionally parses the committed binary manifest,
resolves `org.bndroid.demo.MainActivity`, interprets its exact public
no-argument constructor, and then executes its real
`onCreate(Landroid/os/Bundle;)V`. Its fail-closed pure-data shim admits only
`Activity.<init>`, `Activity.onCreate`, `TextView.<init>`,
`TextView.setText`, and `Activity.setContentView`; the resulting visible text
is `AndroidBox DEX-0 fixture`.

This does not implement package installation, signature policy,
`ActivityThread`, Android Framework behavior, ART/Dalvik, JNI, Binder,
resources, permissions, or a general Activity lifecycle.

The committed APK is a reproducible test fixture, not evidence of general
Android application compatibility.
