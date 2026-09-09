# Mobile mapped buffers and region rendering

Recorded work: August 24 through September 9, 2026. This English summary preserves the implementation contract and principal measurements. The [complete original record](../../archive/originals/MOBILE_MAPPED_BUFFERQUEUE.txt) retains every historical command, hash, artifact path, and limitation.

## Two paths, one display transaction

For `mobile-ui-runtime && !androidbox-apk-install0`, Launcher and App each own two 720x1600 XRGB8888 backings. Producers map them read/write; SurfaceServer maps them read-only. A producer may write only its Writable slot. SurfaceServer reads the exact queued/acquired allocation and generation.

Persistent-install, interactive, and higher AndroidBox profiles retain bounded copied content/chrome layers. They share the v6 region validator, regional copy, and compositor publication path. They now generate and transfer only the affected regions, while the kernel still validates complete sources' canonical XRGB high bytes and enforces layer ownership and generations.

## Mapping and ownership

| Producer | Slot 0 address | Slot 1 address |
| --- | --- | --- |
| Launcher | `0x0000000200400000` | `0x0000000200880000` |
| App | `0x0000000200d00000` | `0x0000000201180000` |

Each backing contains 4,608,000 bytes, or 1,125 4 KiB pages. Four backings consume 18,432,000 bytes. The lifecycle is `Writable -> Queued -> Submitted -> Writable`; a slot cannot be rewritten before acknowledgment or explicit release. Logical client slot selection identifies an exact allocation, not a hint permitting another backing to be acquired.

The applicable address-space configuration uses eleven L3 tables and fourteen private tables overall. Stack guards, W^X, and capability checks remain enforced.

## BufferPresent v6

The mobile `BUP1` transaction remains 64 bytes:

| Field | Encoding |
| --- | --- |
| Byte 7 | Full (1) or Damage (2) |
| Byte 20 | Logical client slot 0 or 1 |
| Byte 21 | Present or Discard disposition |
| Byte 22 | Damage-region count, 0 through 2 |
| Byte 23 | Reserved zero |
| Bytes 32..39 | First rectangle, four little-endian u16 geometry fields |
| Bytes 40..47 | System UI revision |
| Bytes 48..55 | Independent chrome generation |
| Bytes 56..63 | Second rectangle |

Damage regions are nonempty, surface-local, non-overlapping, and canonically ordered by `(y, x, height, width)`. Damage requires Present. Full and Discard zero the count and rectangle slots. The decoder accepts canonical v5 Full/single-region Damage and normalizes re-encoding to v6. Non-mobile profiles retain v2; this does not increment the global ABI beyond 69.

Discard acquires/releases the exact generation, returns PresentCancelled, consumes no display grant, and advances no visible frame ID. Initial slot registration queues and discards fresh black buffers through this same ownership protocol.

## Scheduling and damage

Depth-two asynchronous scheduling lets one slot rasterize and queue while the previous frame awaits acknowledgment. Stale prepared frames are explicitly discarded. Gates require observed dual queueing, `peak_in_flight >= 2`, and publication from both slots.

Full is required for the first frame, focus-baseline changes, and Discard. The shared planner uses input/render geometry for bounded pressed targets, clocks, Phone number/key changes, Calculator result/key changes, and supported Android text/button callbacks. Two separate rectangles do not imply permission to write their bounding-box gap. Unproved changes fall back to Full.

Mobile and layered AndroidBox share the kernel Display frame clock. A 100 Hz logical timer with divider two supplies nominal 50 Hz software grants. Accepted commits consume one consecutive epoch; cancellations do not. This is neither hardware VSync nor a measured FPS claim.

## Copied-layer producer rendering

`mobile_raster.rs` reuses the existing Canvas, pages, font, layout, and damage planner. Compact transfer buffers use region-width stride. Full-width strips transfer in batches; narrow rectangles use the existing copy ABI row by row, leaving horizontal and inter-region gaps untouched.

`MobileRasterCache` tracks written backing separately from acknowledged presented state. A cancelled submission changes only the written baseline; subsequent raster repairs actual backing while subsequent presentation compares against the displayed scene. SurfaceServer separately caches `MobileSystemChromeState` and reuses unchanged bars. These caches confer no focus, application, package, or device authority.

Dynamic Scene-RPC2 validates node structure, identity, geometry, text, and pressed state against the existing renderer. More than two regions, structural changes, or text overflow in undersized exact layouts fall back to Full.

## Recorded September 9 evidence

Historical host runs recorded 252 default UI tests and 298 highest-profile UI tests, including region/full-frame equivalence, boundary sentinels, cancelled-frame repair, chrome reuse, cross-owner rejection, and dynamic-scene changes.

The kernel reports actual GraphicsBufferWrite calls and bytes in `MOBILE_LAYER_COPY_COST_OK`. `scripts/verify-mobile-layer-copy.py` correlates frame, producer, and generation, checking copied bytes, region areas, and zero chrome writes. Modified counts, wrong generations/owners, and missing evidence are rejected.

| Recorded callback | Region area | Copy bytes | Copy calls | Chrome writes | Reduction from full two-layer copy |
| --- | ---: | ---: | ---: | ---: | ---: |
| Interactive-0 button | 177,536 px | 710,144 | 304 | 0 | 84.59% |
| ABI 69 fixed-width button | 80,512 px | 322,048 | 208 | 0 | 93.01% |
| ABI 69 weighted button | 77,632 px | 310,528 | 192 | 0 | 93.26% |

The comparison baseline is 4,608,000 copied bytes. Call count and byte volume are separate costs. These measurements establish copy savings, not GPU performance or display latency.

The recorded Interactive-0 run reused chrome on 34 of 35 commits and preserved source-free recovery disk bytes. The recorded ABI 69 three-boot run covered two SDK APKs, worker recovery, both callbacks, icon/Overview pixels, and source-free reboot; its final boot reused chrome on 33 of 34 commits. Substituting the old full-copy byte count into a real callback log was rejected by the verifier.

Recorded local artifact directories include `target/androidbox-interactive0/check.Aq9VJK`, `target/mobile-ui/check-overview.BsRdLr`, and `target/layout-mixed19/coexist.aqo6Ud`. These are ignored local evidence, not shipped assets or a fresh validation claim.

## Remaining work

Measure frame-time distributions, input latency, and memory before choosing more slots or triple buffering. Hardware VSync, GPU/display hosts, actual panels, full ART/Bionic/Binder/Framework compatibility, and physical-device performance remain unimplemented or unverified. Follow [the roadmap](../../../TODO.md).
