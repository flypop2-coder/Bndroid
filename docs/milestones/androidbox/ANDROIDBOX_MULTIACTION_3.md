# AndroidBox MultiActionActivity-3

## Activity UI-6 当前回归补记

本页下方的 ABI 50 协议说明与原始 `check.LRqaci` 证据仍用于描述 wire 和 DEX
执行边界。Activity UI-6 移除前台 package/RPC/revision 开发诊断后，已用当前源码
重新运行完整双按钮门禁：

```text
terminal=ANDROIDBOX_MULTIACTION3_QEMU_OK
evidence=target/androidbox-multiaction3/check.gHhuen
approve_status_changed_pixels=2951
reject_status_changed_pixels=3006
outside_status_publisher_changed_pixels=0
scene_revision_pixels_hidden=1
relaunch_ppm_identical=1
recovery_disk_unchanged=1
qemu_network=disabled
```

两次真实 Button callback 仍分别改变 status TextView；内部 revision 推进不再产生
用户可见像素，系统 app bar、其他 publisher 节点与导航区域保持逐像素不变。

## 状态

`androidbox-multiaction3` 是 `androidbox-scene-rpc2` 的 opt-in ABI 50 child。
它已通过独立、离线、双启动 QEMU 门禁，但仍只是一个严格有界的 Android
兼容实验，不是 Android 运行时，也不是通用 Android App 支持。

ABI 49 Scene-RPC-2 的语义和证据保持不变：它仍要求恰好一个 callback
`Button`，并继续使用 `BNDAPC02` version 2。只有启用 ABI 50 profile 才把
callback cardinality 扩展到 1–4，并使用 `BNDAPC03` version 3。

## Mac 本地构建的真实 APK

门禁使用一个真实 Java APK，而不是伪造的内部应用记录：

```text
fixture=fixtures/androidbox-multiaction-demo/androidbox-multiaction-demo.apk
package=org.bndroid.multiaction
activity=Lorg/bndroid/multiaction/MainActivity;
version_code=1
apk_bytes=12573
apk_sha256=aecf0749e2c063f44945f6154b479714fa8ebead8da26b0ebef005d2737036c6
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
apk_signature=v2-only
```

`fixtures/androidbox-multiaction-demo/build.sh` 只使用 Mac 上已经安装的
Android SDK/JDK 工具：`aapt2`、`javac`、`d8`、`zipalign` 与
`apksigner`。它完整构建并签名两次，要求两个 12,573-byte APK 逐字节相同。
四个 APK entry 都是 STORED；Manifest 不请求 Android permission。签名只启用
APK Signature Scheme v2，v1/v3/v3.1/v4 均为 false。仓库内测试私钥不是产品
trust anchor。

构建与签名证据分别位于：

```text
target/androidbox-multiaction3/check.LRqaci/fixture-build-one.log
target/androidbox-multiaction3/check.LRqaci/fixture-build-two.log
target/androidbox-multiaction3/check.LRqaci/badging.txt
target/androidbox-multiaction3/check.LRqaci/apksigner.txt
target/androidbox-multiaction3/check.LRqaci/multiaction.first.apk
target/androidbox-multiaction3/check.LRqaci/multiaction.apk
```

## 两个真实动作与有界 DEX

APK 的编译布局是 5 个 preorder 节点：

1. root vertical `LinearLayout`;
2. title `TextView`：`Review request`;
3. status `TextView`：`Decision: pending`;
4. callback `Button`：`Approve`;
5. callback `Button`：`Reject`。

因此本 fixture 精确包含 2 个 `TextView`、2 个 `Button` 和 44 bytes scene
text。`onCreate(Bundle)` 对两个 Button 都执行真实
`setOnClickListener(this)`。真实 Java `onClick(View)` 调用
`View.getId()I`，用编译后的 View ID 选择分支，并把同一个已接纳 status
`TextView` 分别更新为 `Decision: approved` 或 `Decision: rejected`。

ABI 50 的 DEX 扩展仍是 allocation-free 的固定子集，不是 Dalvik：

- callback listener 固定为 1–4 个唯一、非零 ID 的已接纳 Button；
- 只解释有界的 `getId`、`if-eq`、`if-ne`、`goto`、resource const、
  `findViewById`、`check-cast`、`setText(int)`、move-result 与 return shape；
- method code 与实际执行都最多 96 instruction units/steps；
- conditional branch 必须为有效的 forward instruction target；`goto` 也必须
  落在有效 instruction start；
- 每条编码指令都必须从入口可达，完整 control-flow graph 必须有界、可返回且
  无环；因此即使 `d8` 用 backward `goto` 合并公共尾部，也不能形成循环；
- 比较目标必须是本 Activity 已注册的 callback Button，更新目标必须是现有
  `TextView`；
- 发布 retained session 前会 dry-run 每个已注册 callback，任何分支、ID、
  resource 或 mutation 不规范都会 fail closed。

fixture build 还固定检查一个 `getId`、两个 View-ID conditional branch、一个
共享 `setText(int)` 与实际 `goto` shape。相关实现与测试位于
`crates/bndr-androidbox/src/dex.rs`；这不代表能解释任意 DEX bytecode。

## BNDAPC03 / version 3

ABI 50 使用固定 magic `BNDAPC03`、protocol version 3。Scene-RPC-2 的 pull
边界继续生效：最多 8 个 preorder 节点、每段 publisher text 最多 96 bytes、
16-byte descriptor、每次最多一个 outstanding `DescribeNode`、vertical
`LinearLayout`、唯一非零 View ID、有效 earlier-layout parent 和 printable
ASCII。ABI 50 唯一扩大的 scene 语义是 1–4 个 callback Button，以及由
`Click.view_id` 选择对应已验证 listener。

本 fixture 的 fresh scene 顺序为：

```text
Ready(0, abi=50)
Open(1)
SceneOpened(1, node_count=5)
DescribeNode(2, index=0) -> Node
DescribeNode(3, index=1) -> Node + NodeTextChunk(14)
DescribeNode(4, index=2) -> Node + NodeTextChunk(17)
DescribeNode(5, index=3) -> Node + NodeTextChunk(7)
DescribeNode(6, index=4) -> Node + NodeTextChunk(6)
Click(7, approve_button_id) -> Updated(status_id, revision=1)
                              + UpdateTextChunk(18)
Click(8, reject_button_id)  -> Updated(status_id, revision=2)
                              + UpdateTextChunk(18)
Close(9) -> Closed(9)
```

受信 App EL0 host 从每个 scene node 的 callback flag 计算独立物理命中矩形，
不会把第二个 Button 落回旧的单按钮 target。AndroidApp 仍是独立 EL0 worker，
只持有 private `READ|WRITE|WAIT` Channel；它没有 Surface、GraphicsBuffer、
input、storage、network、handle duplicate 或 handle transfer 权限。APK bytes
仍只通过 generation-bound、immutable、read-only VMO 给当前已认证 worker。

source-free recovery boot 中，旧 worker 的受控 fault、同槽 generation+1
replacement、same-VMO reissue 与重新校验继续沿用 ABI 48/49 的严格恢复边界。
replacement 的首轮 request order 为 `0/1/2/3/4/5/6/7/8/9`，共 25 条 RPC
message；普通 relaunch 为 request `10/11/12/13/14/15/16`，共 18 条。replacement
合计 43 条 message、2 次 Click、2 次 Close、0 error。

逐条 wire 证据位于：

```text
target/androidbox-multiaction3/check.LRqaci/protocol-evidence.txt
target/androidbox-multiaction3/check.LRqaci/recovery.serial.normalized.log
```

## 720×1600 双分支像素证据

QEMU ramfb 的四张 Activity 证据都是 720×1600、20:9：

```text
before:
  sha256=1719aa685f13b4a5fdc1491c068f5b77c9f80412e8f8cae3935a8d79fc78e35e
approved:
  sha256=1b8269cab057db466b2c2d655302af42bb91b11694c1cd69ee04fb81a9666d8e
  status_changed_pixels=2951
  outside_status_publisher_changed_pixels=0
rejected:
  sha256=aab5c0ba80e3056a0084c6c942a2f89e2b9ec06ffe35e6e5647a228567c2530c
  status_changed_pixels=3006
  outside_status_publisher_changed_pixels=0
relaunch:
  sha256=1719aa685f13b4a5fdc1491c068f5b77c9f80412e8f8cae3935a8d79fc78e35e
  relaunch_ppm_identical=1
```

两次点击都经过真实 Click→Updated→UpdateTextChunk RPC，并产生不同的最终
raster。除 status publisher 与 renderer-owned scene revision 外，title、两个
Button、top 64 rows、bottom 88 rows 都逐字节不变。详细像素统计和图像位于：

```text
target/androidbox-multiaction3/check.LRqaci/raster-evidence.txt
target/androidbox-multiaction3/check.LRqaci/activity.before.png
target/androidbox-multiaction3/check.LRqaci/activity.approved.png
target/androidbox-multiaction3/check.LRqaci/activity.rejected.png
target/androidbox-multiaction3/check.LRqaci/activity.relaunch.png
```

720×1600 只是固定 QEMU guest scanout；它不证明实体屏幕尺寸、DPI、刷新率、
硬件合成、真实触控或真机适配。

## 移动时钟

SurfaceServer 的手机时钟不再只在启动时冻结取样。它保留当前 PL031 seconds 与
单调 revision，用到下一分钟边界的有限 `ObjectWaitManyArray` timeout；timeout 后和
正常 UI 事件后都会重新读取 PL031。只有可见 minute 改变时才向 Launcher 与 App
广播同一份 `ClockChanged`，revision 严格递增；同一分钟内的 seconds 变化不产生
多余 UI event，RTC 向后跨分钟也会产生新 revision。revision 溢出会保持旧状态并
fail closed。

这次 ABI 50 QEMU 门禁运行时间短于一分钟，只在 serial 中留下 initial
`MOBILE_UI_CLOCK_READ_OK` / `ClockChanged(revision=1)`；因此本证据目录不把一次
真实 QEMU minute rollover 或对应 pixel diff 作为已证明事实。分钟边界、双向跨
分钟和 revision exhaustion 由 `user/init/src/main.rs` 中的固定状态机与 host tests
覆盖。它仍不是 secure time、battery-backed RTC、timer pacing、硬件 VSync 或真实
手机时钟服务。

## 独立双启动门禁

运行：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-multiaction3.sh
```

门禁只管理它自己从 `$!` 捕获的 QEMU PID。两次 QEMU 都恰好使用一个
`-nic none`：

1. fresh 16 MiB package disk + 显式本地 APK source，安装 generation 1；
2. 不提供任何 APK source，从同一磁盘恢复、替换 worker、启动 Activity、
   依次点击 Approve/Reject、关闭、Overview relaunch 并再次关闭。

门禁没有下载 SDK、AOSP、系统镜像或依赖。最新通过证据的仓库相对路径和绝对
路径分别为：

```text
target/androidbox-multiaction3/check.LRqaci/
/Users/apple/Desktop/Bndroid/target/androidbox-multiaction3/check.LRqaci/
```

终态摘要：

```text
ANDROIDBOX_MULTIACTION3_QEMU_OK
abi=50 protocol=BNDAPC03 protocol_version=3
apk_bytes=12573 callback_buttons=2 scene_nodes=5 scene_text_bytes=44
first_rpc_messages=25 relaunch_rpc_messages=18 rpc_errors=0
same_slot=1 generation_step=1 image_claims=3 same_vmo=1 reopened=1
app_layered_commits=14 restart_bound_app_commits=5
relaunch_ppm_identical=1 recovery_disk_unchanged=1
recovery_disk_sha256=4c0c3972ea471ab2f7c3914587089af360d7e9edbfb3104f0c6c1ce93d177e9c
network=disabled unexpected_faults=0 panic_fatal_free=1
```

机器可读的完整摘要位于
`target/androidbox-multiaction3/check.LRqaci/summary.txt`。

用同一份当前源码完成的父档位回归证据为：

```text
ABI 49: target/androidbox-scene-rpc2/check.SePHIM/
ABI 48: target/androidbox-restart0/check.lKVQvP/
ABI 47: target/androidbox-process0/check.kzHejR/
```

这些回归门禁同样使用 `-nic none`，并且只清理各自 `$!` 捕获的新 QEMU PID。

## 明确未实现

**这个里程碑不是 ART、Dalvik、Binder、Bionic、JNI、通用 Android、AOSP
系统或真机。** 它只是 Bndroid 自己的固定容量 APK/Resources/DEX 解释器、独立
worker、受信 UI host 与 QEMU 证据。

```text
art=0 dalvik=0 activitythread=0 android_framework=0
binder=0 bionic=0 jni=0 native_lib=0
permissions=0 services=0 package_manager_api=0
multiple_packages=0 arbitrary_apk=0 general_android_app=0
general_android_compatibility=0 aosp_runtime=0 aosp_system=0
network=0 telephony=0 camera=0 audio=0 sensors=0
hardware_compositor=0 hardware_vsync=0 physical_device_boot=0
real_phone=0
```

一般 Android APK 仍需要 ART、Bionic、Binder、Android Framework 和大量系统
服务。本里程碑没有下载 Android/AOSP 源码，也没有授权或暗示后续可以自动下载；
如未来选择 AOSP 路线，仍必须先明确版本、许可证、存储/构建成本与用户授权。
ABI 49 的历史边界和证据继续见 `ANDROIDBOX_SCENE_RPC_2.md`。
