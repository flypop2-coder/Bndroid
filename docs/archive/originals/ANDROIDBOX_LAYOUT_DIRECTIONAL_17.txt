# AndroidBox Layout Directional-17（ABI 67）

## 结论

opt-in profile `androidbox-layout-directional17` 是 ABI66
`androidbox-layout-spacing16` 的 child。两份由本机 Android 36 SDK、AAPT2 和
D8 9.0.3-dev 离线构建、并以仓库测试证书签名的真实 APK，现在可以在 AAPT2
二进制 XML 中分别声明：

```text
android:paddingLeft / paddingTop / paddingRight / paddingBottom
android:layout_marginLeft / layout_marginTop
android:layout_marginRight / layout_marginBottom
```

八个值从 APK 资源解析、独立 AndroidApp worker、定长进程协议和可信 App scene
model，一直传到 720×1600、20:9 UI 的光栅、按压反馈与点击命中。uniform
`padding` / `layout_margin` 仍可作为四边基值；同一节点的逐边属性覆盖对应边。

本轮没有下载 AOSP、没有修改 Android SDK、没有让 QEMU 联网，也没有停止、替换或
接管持续运行的 ABI48 Mac 预览虚拟机。

## 严格支持范围

当前逐边间距语义有意保持有界：

- 只接受 AAPT2 `TYPE_DIMENSION` 的精确整数 dp 编码；
- 每一边独立限制为 `0..=16dp`；
- 非零 padding 只允许在 `LinearLayout`；
- 非零 margin 只允许在 `Button`；
- 带 margin 的 Button 仍必须满足 ABI65 的 `width=0dp`、
  `height=wrap_content`、整数 `layout_weight=1..=8`；
- start/end、sp、px、fraction、负数、小数、超过 16dp、未知属性和重复属性全部
  fail closed；
- descriptor 中错误 View 类型、未知版本和非零 reserved byte 全部拒绝；
- 同一最终 rectangle 同时用于绘制、触控命中和按压反馈。

真实 fixture 的横向行使用 `padding L/T/R/B = 6/4/2/8dp`；左 Button 使用
`margin L/T/R/B = 2/1/4/3dp`，右 Button 使用 `6/5/2/1dp`：

```text
row logical       =34/320/292/100
row inner         =40/324/284/88
button 1 logical  =42/325/135/84
button 2 logical  =187/329/135/82

button 1 physical =84/650/270/168
button 2 physical =374/658/270/164
```

Host test 证明中间 `(360,740)` 不命中，而 QEMU 分别用 `(219,734)` 与
`(509,740)` 命中两个 APK callback Button。两者不同的 top/bottom margin 也会
产生不同的 y 坐标与高度，不再被折叠为 uniform 值。

## BNDAPC13

系统 ABI 提升为 67，AndroidApp 协议升级为 `BNDAPC13` v13，scene node
descriptor 升级为 v4。descriptor 扩展为 canonical 24 bytes：

```text
byte 12     = layout_weight
byte 13..16 = margin left/top/right/bottom dp
byte 17..20 = padding left/top/right/bottom dp
byte 21..23 = reserved zero
```

六节点场景仍使用严格的单 outstanding pull transcript：

```text
Ready(0)
Open(1) → SceneOpened(node_count=6)
DescribeNode(2..7) → Node(v4)/TextChunk
Click(8) → Updated/TextChunk
Click(9) → Updated/TextChunk
Close(10) → Closed
```

kernel tracer 要求节点 3 携带 `padding=6/4/2/8dp`，节点 4/5 分别携带
`margin=2/1/4/3dp` 与 `6/5/2/1dp`。AndroidBox 的共享解析结构扩大后，QEMU
保护页首先暴露了 64 KiB 交互式 EL0 栈余量不足；交互式 AndroidBox 用户栈现统一
为 20 页（80 KiB），上下保护页、W^X、进程权限、句柄和设备授权均未放宽。

## 真实 APK 与门禁证据

最终门禁使用的两份真实 SDK APK：

```text
package0=org.bndroid.envelope
apk0_bytes=12805
apk0_sha256=7aaa073dcb2ff1ba0a06e96db05ddbd5c18af15f16a105007c1d1de1def92245

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=7f58cb8c6a88d33eab33f2c74cb6e6fc7a9f49f11ed691006e04d8d83eadc4c6
```

运行：

```sh
./scripts/check-androidbox-layout-directional17.sh
```

最终源码对应的三启动、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_LAYOUT_DIRECTIONAL17_QEMU_OK
evidence=target/layout-directional17/coexist.3bHWU6
abi=67
rpc_protocol=BNDAPC13
rpc_protocol_version=13
app_defined_method_transcripts=2
two_distinct_packages=1
two_launcher_entries=1
two_activity_launches_same_boot=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
aosp_downloaded=0
storage_handle_granted=0
block_handle_granted=0
```

720×1600 截图：

```text
target/layout-directional17/coexist.3bHWU6/catalog-activity.png
target/layout-directional17/coexist.3bHWU6/catalog-activity-approved.png
target/layout-directional17/coexist.3bHWU6/catalog-activity-rejected.png
```

Host tests 为 AndroidBox `111/111`、ABI `59/59`、UI `268/268`、kernel
`377/377`。fmt、shell syntax 和五组 AArch64 Clippy `-D warnings` 通过。
ABI66 直接父门以当前共享实现和统一交互栈重新通过：

```text
terminal=ANDROIDBOX_LAYOUT_SPACING16_QEMU_OK
evidence=target/layout-spacing16/coexist.Mb4eRn
```

## AOSP 边界与仍未完成

ABI67 证明的是两个真实 APK 的原生 XML 逐边 margin/padding 在严格受限的
`LinearLayout/TextView/Button` 子集里真实运行，不是普遍 Android App 兼容。
当前阶段继续扩展 APK 解析、布局、framework shim 和进程协议，不需要下载 AOSP。

若后续目标变为完整 ART/Dalvik、ActivityThread、Binder、Bionic、JNI、native
library 和大量 Android Framework/system services，则应单独评估引入 AOSP 源码
或预构建 Android runtime/container。两条路线都会显著改变仓库体积、构建时间、
许可证审计、内存模型与安全边界，必须先取得用户明确授权；本轮未采取任何此类动作。

系统整体还缺少通用 ViewGroup、vertical weight、start/end 与 RTL、gravity、
滚动、RecyclerView、Compose、主题/qualifier 布局选择、完整 PackageManager/
权限、网络 App、任意 APK、硬件 BSP、基带、摄像头、音频、传感器和实体触控驱动。
因此仍不能宣称为可日用真机系统。
