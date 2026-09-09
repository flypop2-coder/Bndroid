# AndroidBox Layout Mixed-19 (ABI 69)

The opt-in `androidbox-layout-mixed19` profile extends ABI 68 Layout Size-18 with fixed-and-weighted Button children in the same horizontal LinearLayout. Two real APKs built offline with the local Android SDK, AAPT2, and D8 use the repository-owned test certificate. The [complete original record](../../archive/originals/ANDROIDBOX_LAYOUT_MIXED_19.txt) preserves all gate output, hashes, and screenshots.

## Accepted layout contract

Each direct horizontal child must be one of:

- Button with exact integer width 1..255dp and weight zero.
- Button with canonical 0dp width and integer weight 1..8.

wrap_content children, exact width plus weight, horizontal TextView children, zero-weight 0dp, unknown attributes, and duplicate attributes fail closed. Exact dimensions remain positive integer AAPT2 TYPE_DIMENSION dp encodings. Checked arithmetic rejects fixed widths, margins, or cumulative weights that cannot fit the parent rectangle.

The parser, independent AndroidApp worker, fixed process protocol, trusted scene, renderer, and kernel tracer independently enforce the contract. Final rectangles are shared by rasterization, semantic text fitting, pressed feedback, and hit testing. Both full button labels must fit; clipping text is not a substitute for correct geometry.

## Remaining-space allocation

```text
distributable = parent_inner_width
              - sum(fixed_content_width + fixed_left/right_margin)
              - sum(weighted_left/right_margin)
```

Cumulative integer weights allocate the remainder without orphan pixels from independent rounding. Fixed children may precede or follow weighted children; preceding fixed widths, allocated weighted widths, and margins determine each origin.

The fixture uses a 240dp title width, a 120dp row height, a 132x64dp fixed Button, and a 0dp/weight-one Button with 56dp height. Row padding is 6/4/2/8dp (LTRB), with button margins 2/1/4/3dp and 6/5/2/1dp.

```text
title logical       = 34/216/240/40
status logical      = 34/260/292/40
row logical         = 34/304/292/120
row inner           = 40/308/284/108
fixed logical       = 42/309/132/64
weighted logical    = 184/313/138/56
fixed physical      = 84/618/264/128
weighted physical   = 368/626/276/112
```

Physical point (358,682) lies in the margin gap and must miss. QEMU clicks (216,682) and (506,682) invoke the two real APK callbacks.

## Protocol

Global ABI becomes 69 without new syscalls, wire fields, or authority. AndroidApp retains `BNDAPC14` v14 and canonical 24-byte scene descriptors v5. Existing exact/zero width, weight, and per-side margin fields express the mixed layout.

```text
Ready(0)
Open(1) -> SceneOpened(node_count=6)
DescribeNode(2..7) -> Node(v5)/TextChunk
Click(8) -> Updated/TextChunk
Click(9) -> Updated/TextChunk
Close(10) -> Closed
```

Only one pull request is outstanding at a time. The tracer verifies node 4 as Exact(132dp), weight zero, and node 5 as Zero, weight one, plus dimensions, side spacing, callback IDs, dynamic StringBuilder text, and the final revision. Earlier pure-weight and spacing profiles retain their isolated contracts.

## Validation

Run from the repository root with the fixture SDK prerequisites installed:

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-layout-mixed19.sh
```

The recorded three-boot gate verifies two APKs, installation, independent worker recovery, both callbacks, package icons, and source-free persistent recovery with QEMU networking disabled. Later September 9 region-rendering measurements are summarized in [the graphics report](../ui/MOBILE_MAPPED_BUFFERQUEUE.md).

This remains a strict LinearLayout/TextView/Button subset on the 720x1600 QEMU UI. General ViewGroup, vertical weights, RTL/start/end, gravity, scrolling, RecyclerView, Compose, resource-qualifier layout selection, full PackageManager/permissions, arbitrary APKs, ART/Bionic/Binder/Framework, and physical-device support remain outside the verified scope.
