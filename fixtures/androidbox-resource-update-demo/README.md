# AndroidBox Resources-1 Update-0 fixture

This directory owns the version 3 update fixture for the repository's narrow
AndroidBox Resources-1 compatibility profile. It intentionally retains package
`org.bndroid.demo` and launcher Activity
`org.bndroid.demo.MainActivity`, while changing the version to `3` / `3.0` and
the resource-backed `TextView` text to `AndroidBox updated resource view`.

This is a repository-owned test fixture, not a third-party application and not
evidence of general Android application compatibility. It must be built only
with:

```sh
./scripts/build-androidbox-resource-update-demo.sh
```

The build is offline and deterministic. It creates exactly four STORED APK
entries, aligns them, signs the same unsigned APK twice, requires byte-for-byte
identical results, and verifies an APK Signature Scheme v2-only profile with
one RSA-2048 signer.

No signing material is stored in this update directory. The build references
the existing repository-public **test-only** key and certificate in
`../androidbox-resource-demo/` without copying them. That signer is not a
product trust anchor, grants no operating-system authority, and must never be
used for production or distributable applications.
