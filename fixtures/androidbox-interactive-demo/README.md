# InteractiveActivity-1 Mac fixture

This fixture is built offline with the installed Android SDK 36.1.0 toolchain:

```sh
./fixtures/androidbox-interactive-demo/build.sh
```

The deterministic output is
`androidbox-interactive-demo.apk`, exactly 12566 bytes with SHA-256
`0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee`.
Its package is `org.bndroid.interactive`, its title is
`Interactive Android app`, and its visible strings are
`Ready for a real APK click`, `Update text`, and
`Button callback executed`.

It is a real Java `Activity` which implements
`android.view.View.OnClickListener`. `onCreate` installs a compiled vertical
`LinearLayout`, finds the compiled Button ID, and binds `this` with
`setOnClickListener`. `onClick` finds the compiled TextView ID, performs the
real DEX `check-cast`, and calls `TextView.setText(int)` with a compiled string
resource ID.

The build script pins the d8 output used by InteractiveActivity-1:

- constructor: `invoke-direct Activity.<init>` and `return-void`;
- onCreate: `invoke-super`, `const/high16`, `setContentView(int)`,
  `const/high16`, `findViewById`, `move-result-object`,
  `setOnClickListener`, and `return-void`;
- onClick: full-width `const`, `findViewById`, `move-result-object`,
  `check-cast TextView`, full-width `const`, `TextView.setText(int)`, and
  `return-void`.

The APK contains exactly four STORED entries and one deterministic
RSA-2048/SHA-256 APK Signature Scheme v2 signer. Its checked-in key is the
repository-public test-only fixture key and is not a production trust anchor.

This proves only the bounded InteractiveActivity-1 API path. It is not ART,
Dalvik, Binder, ActivityThread, Android Framework compatibility, JNI, native
code, networking, general APK compatibility, or a real phone.

In the completed ABI 46 integration milestone, the existing App EL0 process retains
the bounded Activity session and dispatches a Button release through the real
`onClick(View)` subset, changing the TextView from
`Ready for a real APK click` to `Button callback executed`. The exact
720x1600 output is layered by syscall 62: SurfaceServer owns the top 64 and
bottom 88 system-chrome pixels, while App owns the 720x1448 content viewport.
An initial 32 KiB stack guard fault led to a feature-scoped ABI 46 correction
to 64 KiB plus one unmapped guard page at each end; ABI 45 is unchanged.

The completed automated checker is
`../../scripts/check-androidbox-interactive0.sh`; retained evidence is
`../../target/androidbox-interactive0/check.4sMhMR/`, with terminal marker
`ANDROIDBOX_INTERACTIVE0_QEMU_OK`. It performs one explicit-`fw_cfg` install
boot and one source-free recovery boot, both offline. The runtime proves three
graphics-buffer identities and six handles, one producer plus one server
handle for each of SurfaceServer, Launcher, and App.

Three Button samples produce two callback frames across 13 App layered
commits; Launcher completes 16 commits. The callback changes 4138 content
pixels, including 4020 text-label pixels, while the top 64 and bottom 88
system-chrome regions stay byte-identical. Initial and relaunched PPMs both
have SHA-256
`69e6e61e005c2762c6cf4f53168b8929783df402075d53d3ecdebc448efd8209`;
the clicked PPM has SHA-256
`941bac90d2e38de106420c8c1ed2797dd0670e2b1e83fb8114c3d90fed8f1baf`.
Top, Home, and bottom-corner probes route no samples to App; the corner probe
also leaves the claim count and raster unchanged. The recovery disk is
unchanged at SHA-256
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`.
The gate keeps networking disabled, reports no panic or user fault, and is
backed by 243 UI, 82 AndroidBox, 13 init runtime, and 377 kernel host tests.

## ABI 47 Process-0 integration

The completed opt-in `androidbox-process0` child moves this fixture's
immutable APK, bounded interpreter, and retained `ActivitySession` into a
distinct `AndroidApp` EL0 image. The existing App EL0 process remains the
trusted Surface/input/raster host and communicates with AndroidApp through the
authenticated fixed-64-byte `BNDAPC01` Ready/Open/Click/Close protocol.
AndroidApp has no Surface, graphics, input, storage, duplicate, or transfer
authority; its steady state is one private `READ|WRITE|WAIT` Channel handle.

The completed offline checker is
`../../scripts/check-androidbox-process0.sh`; retained evidence is
`../../target/androidbox-process0/check.sFpefj/`, with terminal marker
`ANDROIDBOX_PROCESS0_QEMU_OK`. It performs one explicit-`fw_cfg` generation-1
install boot and one source-free recovery boot, both with networking disabled.

The App ELF is 427528 bytes with SHA-256
`d133b012ff4939eeee783aec05a2c501895425be10b95276d6925a587fec8184`;
the AndroidApp ELF is 157856 bytes with SHA-256
`478eb4e647e7c1fd4a0e7d49dad6d771e1b0144e97079174a1cc96fa97e3d9b0`.
The evidenced App/AndroidApp PIDs are `4294967304/4294967305` and their ASIDs
are `8/9`. The private RPC has exactly two endpoints and one pair, uses request
order `0/1/2/3`, ends at revision 1 with zero errors, and has empty queues
after Close/Closed. App produces 7 layered commits, Launcher 11, AndroidApp 0,
and no input sample reaches AndroidApp.

The Process-0 pre-click and updated 720x1600 guest PPM SHA-256 values are
`69e6e61e005c2762c6cf4f53168b8929783df402075d53d3ecdebc448efd8209`
and
`941bac90d2e38de106420c8c1ed2797dd0670e2b1e83fb8114c3d90fed8f1baf`.
The Button callback changes 4138 content pixels, including 4020 text-label
pixels, while both SurfaceServer-owned chrome regions remain byte-identical.
The source-free recovery disk stays byte-identical at SHA-256
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`.

This process split does not broaden the fixture's compatibility scope. The
worker still runs only the exact InteractiveActivity-1 subset; the kernel
also retains its bounded admission/profile pass. A successful no-panic run is
not crash recovery: there is no AndroidApp restart/rebind or
availability-isolation claim. This remains neither ART, Dalvik,
ActivityThread, Android Framework compatibility, Binder, Bionic, JNI, native
code, networking, general APK compatibility, nor a real phone.

```text
art=0 binder=0 bionic=0 jni=0 network=0 general_apk=0 real_phone=0
abi46_runtime_host=existing-app-el0 abi46_independent_android_app_process=0
abi47_execution_host=independent-android-app-el0
abi47_trusted_ui_host=app-el0 abi47_crash_recovery=0
standalone_general_android_runtime=0
```

## ABI 48 Restart-0 integration

The completed opt-in `androidbox-restart0` child adds one bounded,
authenticated restart of this fixture's `AndroidApp` worker. Its latest
offline two-boot gate is `../../scripts/check-androidbox-restart0.sh`;
retained evidence is `../../target/androidbox-restart0/check.rEGfhV/`, with
terminal marker `ANDROIDBOX_RESTART0_QEMU_OK`.

The recovery boot records exactly one controlled synchronous exception for old
worker PID `4294967305`: `reason=2`, `ESR=0x0000000092000047`,
`FAR=0x00000002001ee000`, `ELR=0x000000020001d4d4`, and
`SP=0x00000002001f7f90`. The reaper accepts it only after binding that witness
to the authenticated Crash request, the old PID, the AndroidApp executable
range, and the mapped stack range. Replacement PID `8589934601` reuses the
same process slot with exactly generation+1, receives the retained private
endpoint, and claims the same immutable APK VMO identity.

Restart success is not published merely when the new worker becomes ready.
After its authenticated `Open(1)`, the App must complete five real
720x1600 layered display presents whose content producer is the original App
PID. The Button `Click(2)` then executes the bounded callback and contributes
two further frames, changing 4138 content pixels, including 4020 label pixels.

Home followed by Overview performs a second ordinary Activity lifecycle on
the already recovered worker. It uses `Open(4)` and `Close(5)`, makes the
third total package-image claim through a fresh ordinary Launcher grant rather
than another restart reissue, and presents another five App frames without
incrementing the restart error ledger. The initial and post-relaunch PPMs are
byte-identical, both with SHA-256
`9ea7c996652114b3544a11a1c52f72ff9a9213383f7c00cf033515d862e8c0fa`;
the clicked PPM has SHA-256
`25aba1af8d3400f36d22d9215797a8d54dd129fbaf99a0fd69cca47f449452cc`.

Every restart phase, including the rebound-to-visible-surface closure, has a
1,000-tick fail-stop progress deadline. Normal completion consumes and clears
the ordinary grant and same-VMO proof; if a replacement exits after reissue
but before claiming, owner-death cleanup drops both the unclaimed grant and
the restart proof so later ordinary publication is not left permanently
busy. The passing run has three claims, one escrow, one reissue, one VMO
identity, 12 App layered commits, request orders `0/1/2/3` and `4/5`, zero RPC
errors, no timeout, and an unchanged recovery package disk with SHA-256
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`.

This remains only the pinned InteractiveActivity-1/Resources-1 execution and
single controlled worker-restart path. It is not ART or Dalvik, Binder,
ActivityThread, Android Framework or general APK compatibility, a general
process supervisor, arbitrary crash recovery, or evidence of operation on a
physical/real phone.

```text
abi48_art=0 abi48_dalvik=0 abi48_binder=0
abi48_general_apk=0 abi48_general_android_compatibility=0
abi48_general_process_supervisor=0 abi48_real_phone=0
```
