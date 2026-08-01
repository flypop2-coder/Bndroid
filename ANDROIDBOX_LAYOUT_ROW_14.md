# AndroidBox Layout Row-14（ABI 64）

## 结论

opt-in profile `androidbox-layout-row14` 是 ABI63
`androidbox-string-builder13` 的 child。两份由本机 Android 36 SDK、JDK 和
D8 9.0.3-dev 离线构建的真实 APK，现在可以从 `res/layout` 的 AAPT2 二进制
XML 中声明一个嵌套横向 `LinearLayout`，并把两个真实 `Button` 并排呈现在
720×1600、20:9 的 Bndroid 手机界面中。

本轮没有下载 AOSP、没有修改 Android SDK、没有联网运行 QEMU，也没有操作持续
运行的 ABI48 Mac 预览虚拟机。

## 有界布局语义

当前解释器接受的范围是：

- 根节点必须是纵向 `LinearLayout`；
- 嵌套 `LinearLayout` 可以是纵向或横向；
- 视图类型仍只限 `LinearLayout`、`TextView` 和 `Button`；
- 最多 8 个节点、最大深度 3，传输顺序为 preorder；
- 只接受当前 ABI 已定义的 `match_parent`、`wrap_content`、文本、ID 和 callback；
- 横向容器把直接可见子节点分成相等、有界的列；
- layout 节点本身不绘制伪造表面，leaf 的同一矩形同时用于光栅和点击命中；
- 根横向布局、坏 parent、重复 ID、未知 orientation、越界深度/节点数和非 canonical
  descriptor 全部 fail closed。

当前还没有 `layout_weight`、margin、padding、gravity、constraint、relative/frame
layout、滚动容器、RecyclerView、Compose、主题属性、资源 qualifier 布局选择或
任意 Android View。

## 精确几何与文字

Android Activity 内容使用 360×800 设计网格，再精确 2 倍输出到 720×1600：

```text
title       logical=34/216/292/48
status      logical=34/268/292/48
button 1    logical=34/320/142/100   physical=68/640/284/200
button 2    logical=180/320/142/100  physical=360/640/284/200
column gap  physical=8
```

两个按钮的命中区域就是上述 physical rectangle。横向按钮使用适合列宽的
`Label` 字号；`Show components` 和 `Show permissions` 的完整 raster advance
都必须小于或等于 244 physical pixels，不能通过截断伪装为适配。状态更新仍只改变
status `TextView` 的像素范围，系统状态栏、底部导航和可信 App header 不变。

## BNDAPC10

系统 ABI 提升为 64，固定大小进程协议升级为 `BNDAPC10` v10。
`AndroidAppSceneOrientation::Horizontal` 的 canonical wire 值为 2；旧
`None=0`、`Vertical=1` 保持不变。ABI decoder、AndroidApp worker、App client、
UI scene model 和 kernel tracer 分别解析并验证 orientation，不能把横向根节点或
未知值带入 UI。

六节点场景的严格开放转录为 19 条消息：

```text
Ready(0)
Open(1) → SceneOpened(node_count=6)
DescribeNode(2..7) → Node/TextChunk
Click(8) → Updated/TextChunk
Click(9) → Updated/TextChunk
Close(10) → Closed
```

一次故意的 EL0 guard-page fault 会发生在首次完整 scene open 后、UI commit 前。
kernel 只接受同 slot、generation +1 的新 AndroidApp worker，重新绑定同一只读
APK VMO，要求完整重放六节点转录后才允许 UI 提交。协议 magic、崩溃请求号、
第二次点击序号或 descriptor 只要沿用父 ABI 的旧值，门禁都会拒绝。

## 真实 APK 与自动门禁

横向布局 fixture：

```text
package0=org.bndroid.envelope
apk0_bytes=12803
apk0_sha256=653c3f3b782f072738852f1ee16561ba1c1cf2564abfb98acdaceb239771bd1c

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=9417b98b7ce0c9dfd5981a7d5b91c9a2609ef451d6f03d5e95662f01aff41631
```

运行：

```sh
./scripts/check-androidbox-layout-row14.sh
```

最终源码对应的三启动、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_LAYOUT_ROW14_QEMU_OK
evidence=target/layout-row14/coexist.ozW81O
abi=64
rpc_protocol=BNDAPC10
rpc_protocol_version=10
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

人工核对的 720×1600 截图在：

```text
target/layout-row14/coexist.ozW81O/catalog-activity.png
target/layout-row14/coexist.ozW81O/catalog-activity-approved.png
```

当前 host tests 为 AndroidBox `109/109`、ABI `59/59`、UI `265/265`、
kernel `377/377`。ABI63 直接父门也以当前共享实现重新通过：

```text
terminal=ANDROIDBOX_STRING_BUILDER13_QEMU_OK
evidence=target/string-builder13/coexist.IsZM9k
```

## 仍未完成

ABI64 证明的是两份真实 SDK APK 在受限二进制 XML、受限 DEX/Framework 子集中的
运行，不是普遍 Android App 兼容。当前仍不包含 ART/Dalvik、通用 Java heap/GC、
ActivityThread、Binder、Bionic、JNI、native library、完整 Android Framework 和
system services、通用 PackageManager/权限语义、网络 App、任意 APK、硬件 BSP、
PMIC、基带、摄像头、音频、传感器、实体触控或真机驱动。

因此不能宣称系统已经是可日用手机或兼容一般 Android App。若后续选择 AOSP/ART
路线，下载大型外部源码或预构建组件仍须用户另行明确授权。
