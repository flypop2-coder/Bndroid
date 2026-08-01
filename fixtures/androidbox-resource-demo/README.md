# AndroidBox resource demo fixture

This directory owns an intentionally small Android APK fixture for offline
AndroidBox parser, signature-verifier, and interpreter tests. It is not a
third-party app and must not be described as evidence of general Android
application compatibility.

`MainActivity.onCreate()` calls `setContentView(R.layout.activity_main)`. The
compiled binary layout contains a root `TextView`, and its `android:text`
attribute references `@string/activity_message` in `resources.arsc`. The APK
also retains the allocation-free `Main.boot()` and `Main.onTap(int)` probes from
the earlier fixture.

Build it only with the repository script:

```sh
./scripts/build-androidbox-resource-demo.sh
```

The script uses the already-installed Android SDK without network access,
stores every APK entry without ZIP compression, and signs the aligned APK with
APK Signature Scheme v2 only (one RSA-2048/SHA-256 signer). It signs twice and
compares the results before replacing the fixture, then validates the exact
signature profile and writes the SHA-256 sidecar.

`test-only-apk-signing-key.pk8` and `test-only-apk-signing-cert.pem` are
repository-owned **test-only** material. The private key is intentionally
public to everyone who can read this repository. It is not a product trust
anchor, grants no operating-system authority, and must never sign production
or distributable applications. Its pinned test certificate SHA-256 is
`e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`;
the corresponding DER SubjectPublicKeyInfo SHA-256 is
`c04dfd1d94654aa6dc5c861c2f15f224d9721c07f6c6c2f5447c7efdba05c236`.
