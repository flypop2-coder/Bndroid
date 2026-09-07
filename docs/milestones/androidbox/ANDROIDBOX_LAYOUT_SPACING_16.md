# AndroidBox Layout Spacing-16（ABI 66）

## 结论

opt-in profile `androidbox-layout-spacing16` 是 ABI65
`androidbox-layout-weight15` 的 child。两份由本机 Android 36 SDK、AAPT2 和
D8 9.0.3-dev 离线构建并以仓库测试证书签名的真实 APK，现在可以在 AAPT2
二进制 XML 中声明 uniform `android:padding="4dp"` 和
`android:layout_margin="2dp"`。这些值从 APK 资源解析、独立 AndroidApp
worker、定长进程协议和可信 App scene model，一直传到 720×1600、20:9 UI 的
光栅与点击命中。

本轮没有下载 AOSP、没有修改 Android SDK、没有让 QEMU 联网，也没有停止、替换或
接管持续运行的 ABI48 Mac 预览虚拟机。

## 严格支持范围

当前间距语义有意保持有界：

- 只接受 AAPT2 `TYPE_DIMENSION` 的精确整数 dp 编码；
- uniform `padding` 和 `layout_margin` 的范围均为 `0..=16dp`；
- 非零 `padding` 只允许在 `LinearLayout`；
- 非零 `layout_margin` 只允许在 `Button`；
- fixture 中带 margin 的 Button 仍必须满足 ABI65 的
  `width=0dp`、`height=wrap_content`、整数 `layout_weight=1..=8`；
- directional margin/padding、sp、px、fraction、负数、小数和超过 16dp 全部拒绝；
- descriptor 中错误 View 类型、未知版本和非零 reserved byte 全部 fail closed；
- 同一最终 rectangle 同时用于绘制、触控命中和按压反馈。

真实 fixture 的横向行使用 `padding=4dp`，两个权重 Button 各使用
`layout_margin=2dp`：

```text
row logical      =34/320/292/100
row inner        =38/324/284/92
button 1 logical =40/326/138/88
button 2 logical =182/326/138/88

button 1 physical=80/652/276/176
button 2 physical=364/652/276/176
middle gap       =8 physical pixels
```

Host test 还证明 gap 中心 `(360,740)` 不命中，而 `(218,740)` 和
`(502,740)` 分别命中两个 APK callback Button。

## BNDAPC12

系统 ABI 提升为 66，AndroidApp 协议升级为 `BNDAPC12` v12，scene node
descriptor 升级为 v3。16-byte descriptor 保持定长：

```text
byte 12 = layout_weight
byte 13 = uniform layout_margin dp
byte 14 = uniform padding dp
byte 15 = reserved zero
```

六节点场景仍使用严格的单 outstanding pull transcript：

```text
Ready(0)
Open(1) → SceneOpened(node_count=6)
DescribeNode(2..7) → Node(v3)/TextChunk
Click(8) → Updated/TextChunk
Click(9) → Updated/TextChunk
Close(10) → Closed
```

kernel tracer 要求节点 3 是 `padding=4dp` 的横向 `LinearLayout`，节点 4/5
是 `margin=2dp`、`weight=1`、`width=0dp` 的 Button。首次完整 open 后的受控
EL0 guard-page fault 仍要求同 slot、generation +1 的 AndroidApp worker 重绑
只读 APK VMO 并完整重放场景。

## 真实 APK 与门禁证据

最终门禁使用的两份真实 SDK APK：

```text
package0=org.bndroid.envelope
apk0_bytes=12805
apk0_sha256=1e7fbcbe1a2ba89fbd292975ac2cf28129bab25b8fd886c165aff3e1e3dfd0da

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=9838e74ff3bb022327607b7bebccd27deb1a48de6f5cfbe92bf925fa2b1fb28a
```

运行：

```sh
./scripts/check-androidbox-layout-spacing16.sh
```

最终源码对应的三启动、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_LAYOUT_SPACING16_QEMU_OK
evidence=target/layout-spacing16/coexist.Mb4eRn
abi=66
rpc_protocol=BNDAPC12
rpc_protocol_version=12
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
target/layout-spacing16/coexist.Mb4eRn/catalog-activity.png
target/layout-spacing16/coexist.Mb4eRn/catalog-activity-approved.png
target/layout-spacing16/coexist.Mb4eRn/catalog-activity-rejected.png
```

Host tests 为 AndroidBox `111/111`、ABI `59/59`、UI `267/267`、kernel
`377/377`。fmt、shell syntax 和五组 AArch64 Clippy `-D warnings` 通过。
ABI65 直接父门以当前共享实现重新通过：

```text
terminal=ANDROIDBOX_LAYOUT_WEIGHT15_QEMU_OK
evidence=target/layout-weight15/coexist.lHJezZ
```

## AOSP 边界与仍未完成

ABI66 证明的是两个真实 APK 的原生 XML uniform spacing 在严格受限的
`LinearLayout/TextView/Button` 子集里真实运行，不是普遍 Android App 兼容。
当前阶段继续扩展 APK 解析、布局、framework shim 和进程协议，不需要下载 AOSP。

若后续目标变为完整 ART/Dalvik、ActivityThread、Binder、Bionic、JNI、native
library 和大量 Android Framework/system services，则应单独评估两条路线：
引入 AOSP 源码构建，或把预构建 Android runtime/container 作为兼容域。两者都会
显著改变仓库体积、构建时间、许可证审计、内存模型与安全边界，必须先取得用户明确
授权；本轮未采取任何此类动作。

系统整体还缺少通用 ViewGroup、vertical weight、方向性 spacing、gravity、
滚动、RecyclerView、Compose、主题/qualifier 布局选择、完整 PackageManager/
权限、网络 App、任意 APK、硬件 BSP、基带、摄像头、音频、传感器和实体触控驱动。
因此仍不能宣称为可日用真机系统。
