# AndroidBox ProfileActivity fixture

This directory contains the second real Java APK fixture intended for the
bounded ABI 49 `SceneRPC-2 / MultiViewActivity-2` milestone.

Build it completely offline with the Android SDK already installed on the Mac:

```sh
./fixtures/androidbox-profile-demo/build.sh
```

The checked-in deterministic artifact is
`androidbox-profile-demo.apk`, exactly 12,569 bytes with SHA-256
`535d92a7a8f0a13a4d9138a535404033da4e8e4ee6221ea6fb355df050753507`.
Its test-only signer certificate SHA-256 is
`e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`.

The deterministic build compiles resources with `aapt2`, Java with `javac`,
DEX with `d8`, aligns the four-entry STORED archive, and signs it twice with
the repository-public test-only RSA-2048 key. The two signed outputs must be
byte-identical. It then validates package/component metadata, DEX calls,
compiled resources, the nested layout, APK Signature Scheme v2, signer
certificate, archive shape, and the 65,024-byte package ceiling.

The package is `org.bndroid.profile`; its launcher component is
`org.bndroid.profile.MainActivity`. The compiled scene is deliberately
different from the existing InteractiveActivity-1 fixture:

- one root vertical `LinearLayout`;
- one nested vertical `LinearLayout`;
- two independent `TextView` nodes for the title and status;
- one `Button`;
- one standard `View.OnClickListener` callback which changes only the status
  through `TextView.setText(int)`.

ABI 49 now transports and renders this complete bounded tree through
`BNDAPC02` Scene-RPC-2. The offline two-boot QEMU gate passed at
`target/androidbox-scene-rpc2/check.IkELmU/`: it proved five nodes, two
independent `TextView` values, one callback `Button`, a status-only update,
worker replacement, source-free relaunch, byte-identical initial/relaunch
rasters, and an unchanged package disk. See `ANDROIDBOX_SCENE_RPC_2.md`.

ABI 48 remains intentionally unchanged and still uses its one-TextView /
one-Button `BNDAPC01` profile.

This remains a tiny, intentionally bounded Java/DEX and Resources subset. It
does not provide ART, Dalvik, ActivityThread, Binder, Bionic, JNI, native
libraries, services, permissions, networking, multiple installed packages,
arbitrary APK compatibility, physical-device support, or a real-phone claim.
The signing key is test-only and is not a production trust anchor.
