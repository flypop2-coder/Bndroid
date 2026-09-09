# AndroidBox Scene-RPC-2

## 状态

`androidbox-scene-rpc2` 是 `androidbox-restart0` 的 opt-in ABI 49 child。
它已通过独立、离线、双启动 QEMU 门禁，但仍是一个严格有界的 Android
兼容实验，不是通用 Android 运行时。

ABI 49 首次把一个真实 APK 的完整有界 View 树从独立 AndroidApp EL0 worker
传给受信 App EL0 UI host。ABI 47/48 的 `BNDAPC01` wire 和既有行为保持不变；
只有启用本 profile 才使用 `BNDAPC02` version 2。

## 本次真实 APK

门禁使用第二个、结构不同的 Java APK：

```text
fixture=fixtures/androidbox-profile-demo/androidbox-profile-demo.apk
package=org.bndroid.profile
activity=Lorg/bndroid/profile/MainActivity;
version_code=1
apk_bytes=12569
apk_sha256=535d92a7a8f0a13a4d9138a535404033da4e8e4ee6221ea6fb355df050753507
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
apk_signature=v2-only
```

`build.sh` 只使用 Mac 上已经安装的 `aapt2`、`javac`、`d8` 与
`apksigner`，完整构建两次并要求最终 APK bytes 一致。门禁不下载 SDK、
AOSP、镜像或依赖。

APK 的编译布局是 5 个 preorder 节点：

1. root vertical `LinearLayout`;
2. title `TextView`，内容为 `Account profile`;
3. nested vertical `LinearLayout`;
4. status `TextView`，初始为 `Profile status: pending`;
5. callback `Button`，内容为 `Verify profile`。

真实 `View.OnClickListener` 只对 status `TextView` 执行
`setText(int)`，结果为 `Profile status: verified`。title 与 Button
不会被回调修改。

## BNDAPC02 pull 协议

一个 fresh Open 的规范顺序是：

```text
Ready(0)
Open(1)
SceneOpened(1, node_count=5)
DescribeNode(2, index=0) -> Node
DescribeNode(3, index=1) -> Node + NodeTextChunk(15)
DescribeNode(4, index=2) -> Node
DescribeNode(5, index=3) -> Node + NodeTextChunk(23)
DescribeNode(6, index=4) -> Node + NodeTextChunk(14)
```

协议边界：

- 最多 8 个节点；
- 每段 publisher text 最多 96 bytes，Button 最多 64 bytes；
- descriptor 固定 16 bytes，保留字段必须为零；
- 只接纳 `LinearLayout`、`TextView`、`Button`；
- `LinearLayout` 只允许 vertical orientation；
- parent 必须指向 preorder 中更早的 `LinearLayout`；
- 非零 View ID 不得重复；
- 至少一个 `TextView`，且恰好一个 callback `Button`；
- App 每次只发一个 `DescribeNode`，完整收到响应后才能请求下一个；
- 所有 publisher text 必须是有界 printable ASCII。

点击使用 request 7。worker 返回 status View ID、revision 1 与 24-byte
更新文本；App 只接受现有 `TextView` ID，并把后续更新锁定到该 ID。关闭使用
request 8。第二次普通 relaunch 继续使用 request 9–15。

## 权限与进程边界

AndroidApp 仍是独立 EL0 image，只持有一条 private
`READ|WRITE|WAIT` Channel。它没有 Surface、GraphicsBuffer、input、storage、
network 或 handle transfer/duplicate 权限。

APK bytes 只通过一次性、immutable、`READ`-only VMO 暴露给当前受认证
AndroidApp PID generation。Init、Launcher 与 App 只看到元数据，不看到 APK
bytes。worker 每次 Open 都重新检查 APK digest、APK-v2 signer、Manifest、
Resources 与有界 DEX/回调。

本 profile 继承 ABI 48 的一次性 worker 恢复。旧 worker 完整描述第一棵树后，
接收 `Crash(7)` 并产生预定 EL0 guard-page data abort。Init 只可创建同槽
generation+1 replacement；新 worker 必须 `Ready(0)`、重新 claim 同一个
VMO identity，并重新完成 request 1–6。任何旧 PID、错误 generation、错误
session、乱序或 replay 都 fail closed。

## 720×1600 手机 UI

受信 App host 在固定 720×1600、20:9 scanout 上通用渲染已接纳节点：

- `LinearLayout` 只影响层级、缩进与 leaf 排列，不伪造可见控件；
- 每个 `TextView` 有独立 publisher surface；
- callback `Button` 使用场景计算出的动态半开物理矩形；
- 非 callback Button 可显示为禁用，但不能命中；
- scene active 时动态 Button 优先，不能落回 ABI 48 的固定 Button target；
- trusted top 64 rows 与 bottom 88 rows 始终由 SurfaceServer 独立绘制。

Settings 与 Apps 同时使用固定 header、受 trusted chrome 裁剪的真实滚动
viewport、8px finger-follow、严格 scroll bounds 和变换后的 hit test。拖动取消
点击，Back、顶部下拉、软件亮度与 system Home 保持更高优先级。

UI 明确显示 `Scene-RPC-2 / 1-8 nodes only` 和兼容边界，不把该结果表述为
ART、Binder 或通用 Android。

## 独立双启动门禁

运行：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-scene-rpc2.sh
```

脚本只管理它自己从 `$!` 捕获的 QEMU PID。两次 QEMU 都恰好使用一个
`-nic none`：

1. fresh 16 MiB package disk + explicit local APK source，安装 generation 1；
2. 不提供任何 APK source，从磁盘恢复、启动 Activity、完成 worker 替换、
   点击、关闭、Overview relaunch 与再次关闭。

最新通过证据：

```text
target/androidbox-scene-rpc2/check.IkELmU/
```

终态摘要：

```text
ANDROIDBOX_SCENE_RPC2_QEMU_OK
abi=49 protocol=BNDAPC02 protocol_version=2
scene_nodes=5 scene_text_bytes=52 text_views=2 callback_buttons=1
rpc_request_order=0/1/2/3/4/5/6/7/8
post_restart_request_order=9/10/11/12/13/14/15
first_rpc_messages=21 relaunch_rpc_messages=17 rpc_errors=0
old_android_app_pid=4294967305 new_android_app_pid=8589934601
same_slot=1 generation_step=1 image_claims=3 image_reissues=1
app_layered_commits=12 restart_bound_app_commits=5
status_changed_pixels=3526 outside_status_publisher_changed_pixels=0
relaunch_ppm_identical=1 recovery_disk_unchanged=1 network=disabled
```

点击前与普通 relaunch 的 PPM SHA-256 完全相同：

```text
58d79e9aba1b5061108afa2bf0b24f996309a6b4918f0561128823c6c627eef2
```

点击后的 PPM SHA-256：

```text
e16170bbac20e18ecb9b39385622e4bc98dee1b9ac4335d5c74918d09f5a427e
```

publisher 域中只有 status 节点变化；renderer-owned scene revision 从 1
变为 2，并被证据单独统计。title、Button、package identity、trusted chrome
与其余 publisher pixels 保持逐字节一致。source-free boot 前后 package disk
SHA-256 均为
`d9b32d46919c63ffe331658daa4bbc9b54d24abc1ca8ca0486413731eccf9ede`。

回归门禁也已重新通过：

```text
ABI 48: target/androidbox-restart0/check.CssUbk/
ABI 47: target/androidbox-process0/check.TuITxg/
```

ABI 48 门禁原先错误地要求 supervisor `Rebound` read 与 private-worker claim
read 具有唯一跨 Channel 顺序。两条 Channel 可以合法交错；脚本现改为验证真实
因果边界：grant reissue < replacement claim < completed reopen，同时仍分别严格
验证 Rebound 与 worker RPC。

## 明确未实现

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0
jni=0 native_lib=0 permissions=0 services=0
package_manager_api=0 multiple_packages=0 arbitrary_apk=0
network=0 telephony=0 camera=0 audio=0 sensors=0
general_android_compatibility=0 physical_device_boot=0 real_phone=0
```

要运行一般 Android APK，后续仍需要 ART、Bionic、Binder、Android Framework
和更完整的系统服务。那一阶段可以选择经明确授权下载/构建 AOSP，或接入明确版本
与许可证的预构建 AOSP guest；本里程碑没有下载 AOSP，也没有暗示已经具备这些能力。
