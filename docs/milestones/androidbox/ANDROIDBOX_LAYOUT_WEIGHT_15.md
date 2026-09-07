# AndroidBox Layout Weight-15（ABI 65）

## 结论

opt-in profile `androidbox-layout-weight15` 是 ABI64
`androidbox-layout-row14` 的 child。两份由本机 Android 36 SDK、AAPT2 和
D8 9.0.3-dev 离线构建的真实 APK，现在可以在 AAPT2 二进制 XML 中声明
`android:layout_width="0dp"` 与 `android:layout_weight="1"`。该属性从 APK
资源解析、AndroidApp worker、固定大小进程协议、可信 App scene model，一直传到
720×1600、20:9 UI 的比例布局和点击命中。

本轮没有下载 AOSP、没有修改 Android SDK、没有让 QEMU 联网，也没有停止或替换
持续运行的 ABI48 Mac 预览虚拟机。

## 严格支持范围

当前权重语义有意保持有界：

- 只接受 AAPT2 的 `TYPE_DIMENSION/data=1` 作为精确 `0dp`；
- 只接受 `TYPE_FLOAT` 编码的整数权重 `1..=8`；
- 带权重节点必须是 `Button`，宽度必须为 `0dp`，高度必须为
  `wrap_content`；
- 带权重节点的直接 parent 必须是横向 `LinearLayout`；
- 不带权重的节点不能使用 `0dp`，高度也不能使用 `0dp`；
- 横向直接子节点按 `child_weight / total_weight` 分配宽度；
- 同一个最终 rectangle 同时用于绘制、触控命中和按压反馈；
- 非整数、零、负数、超过 8 的权重，错误类型、错误 parent、未知 binary
  encoding、坏 descriptor 和协议降级全部 fail closed。

真实 fixture 使用 `1/1`，所以视觉宽度与 ABI64 相同：

```text
button 1 logical=34/320/142/100   physical=68/640/284/200
button 2 logical=180/320/142/100  physical=360/640/284/200
gap      physical=8
```

另有独立 `1/2` host test，证明实现不是继续按子节点数量平分：

```text
button 1 logical=34/320/93/100    physical=68/640/186/200
button 2 logical=131/320/191/100  physical=262/640/382/200
gap      physical=8
```

## BNDAPC11

系统 ABI 提升为 65，AndroidApp 协议升级为 `BNDAPC11` v11，scene node
descriptor 升级为 v2。16-byte descriptor 保持定长，byte 12 传递权重；
bytes 13..16 必须为零。`AndroidAppSceneDimension::Zero` 的 wire 值为 3。

六节点场景仍使用严格的单 outstanding pull transcript：

```text
Ready(0)
Open(1) → SceneOpened(node_count=6)
DescribeNode(2..7) → Node(v2)/TextChunk
Click(8) → Updated/TextChunk
Click(9) → Updated/TextChunk
Close(10) → Closed
```

kernel tracer 不只检查消息数量，还要求 Catalog 两个 Button 的 descriptor
分别为 `width=Zero, height=WrapContent, layout_weight=1`。首次完整 open 后的
受控 EL0 guard-page fault 仍要求同 slot、generation +1 的 AndroidApp worker
重绑只读 APK VMO并完整重放场景。

## 真实 APK 与门禁证据

最终门禁使用的两份真实 SDK APK：

```text
package0=org.bndroid.envelope
apk0_bytes=12804
apk0_sha256=666e316500236f2a0f66b24c59990ef5cb66de1ee61be76843d6f710e153bd4b

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=1173403d9cb213633693746bd4971a1d8c5ce97ef7691d08e86ef6d0e7b60320
```

运行：

```sh
./scripts/check-androidbox-layout-weight15.sh
```

最终源码对应的三启动、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_LAYOUT_WEIGHT15_QEMU_OK
evidence=target/layout-weight15/coexist.xC7DAf
abi=65
rpc_protocol=BNDAPC11
rpc_protocol_version=11
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

人工核对的 720×1600 截图：

```text
target/layout-weight15/coexist.xC7DAf/catalog-activity.png
target/layout-weight15/coexist.xC7DAf/catalog-activity-approved.png
```

Host tests 为 AndroidBox `110/110`、ABI `59/59`、UI `267/267`、kernel
`377/377`。fmt、shell syntax 和五组 AArch64 Clippy `-D warnings` 通过。
ABI64 直接父门以当前共享实现重新通过：

```text
terminal=ANDROIDBOX_LAYOUT_ROW14_QEMU_OK
evidence=target/layout-row14/coexist.Jsu7Eq
```

## 仍未完成

ABI65 证明的是 Android 原生 XML 权重属性在一个严格受限的
`LinearLayout/TextView/Button` 子集里真实运行，不是普遍 Android App 兼容。
仍未支持非整数 weight、`weightSum`、vertical weight、margin、padding、gravity、
更多 ViewGroup、滚动、RecyclerView、Compose、主题系统或 qualifier 布局选择。

系统整体仍没有 ART/Dalvik、通用 Java heap/GC、ActivityThread、Binder、
Bionic、JNI、native library、完整 Android Framework/system services、通用
PackageManager/权限、网络 App、任意 APK、硬件 BSP、基带、摄像头、音频、传感器
和实体触控驱动。因此仍不能宣称为可日用手机；下载 AOSP 或预构建 Android 运行时
仍须用户另行明确授权。
