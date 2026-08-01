# AndroidBox Runtime Install-2（ABI 53）

## 结论

opt-in profile `androidbox-runtime-install2` 是 ABI 52
`androidbox-runtime-uninstall1` 的 child。它把此前“启动时自动安装 APK”的模型改为
显式的、由用户在“设置 → 应用”中确认的运行时安装事务，同时保留 ABI 52 的卸载能力。

本里程碑在现有单包 `Resources-1 / Envelope-4` 兼容范围内实现：

- 未格式化的 virgin package volume 可在不写盘的情况下发布 `Install` 候选。
- 已安装包遇到相同 package、activity 和 signer 的更高 `versionCode` 时发布
  `Update` 候选。
- 已卸载 tombstone 遇到匹配 identity/signer 且版本不回退的 APK 时发布
  `Reinstall` 候选。
- 第一次点击只显示确认框；取消和第一次点击都不会提交 syscall 或改变磁盘。
- 第二次确认才执行异步、crash-safe package-store 事务。
- 成功后 Settings 和 Launcher 在同一次启动中读取新的 live catalog，并可从
  All apps 启动新 generation。
- 随后的启动可以完全不提供 APK source，仍从 package store 恢复并启动已安装包，
  且稳定恢复为零 write/flush。

本轮继续只使用 Mac 已安装的 Android SDK/JDK，构建两个真实且 APK-v2-only 签名的
APK。没有下载 AOSP，也没有启用网络：

```text
package=org.bndroid.envelope
activity=Lorg/bndroid/envelope/MainActivity;
profile=Resources-1 / Envelope-4 / MultiActionActivity-3
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf

v1 versionCode=1 versionName=1.0
v1 bytes=12646
v1 sha256=a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5

v2 versionCode=2 versionName=2.0
v2 bytes=12644
v2 sha256=5578268c3f1ecf2649d0abb0a9c8c07f254c9636c8e9e8c4ed41525414386ff6
```

## 用户界面

Settings 的 Apps 页同时展示两个彼此独立的 runtime 事实：

1. kernel 发布的当前已安装 package catalog；
2. kernel 在本次启动中完整验证的只读 install candidate。

候选卡会显示 package、目标版本、APK size、签名/profile 状态以及
`Install`、`Update` 或 `Reinstall` 操作。状态机固定为：

```text
Idle -> Confirming -> Pending -> Installed
                         \----> Failed
```

用户第一次点击操作按钮只进入 `Confirming`。确认框的取消按钮返回 `Idle`；
第二次点击确认按钮才创建 operation ID 并提交 syscall 65。UI 只有在收到严格匹配
sequence、operation、candidate、generation、version 和 digest 的成功结果，
并再次读取到完全一致的 live catalog 后才显示完成。Stale、Busy、Storage 和
Verification 均是独立失败状态，renderer 不会从点击动作推断安装成功。

当 update candidate 存在或安装事务处于 pending 时，卸载按钮不可用，避免在同一
旧 generation 上同时形成更新与卸载事务。Launcher 不读取私有 candidate；
它只在重新获得 focus 时读取公开 installed snapshot，因此不会获得 source 身份或
安装权限，但能在同一次启动中看见成功安装后的新应用。

屏幕继续使用固定 `720×1600`、20:9 scanout，逻辑设计尺寸为 `360×800`、2× scale。
系统 chrome 保留 64-pixel status bar 和 88-pixel navigation bar；Settings 页面
具有真实滚动 viewport、圆角卡片、按压反馈、至少 44 设计像素触控目标，以及屏幕
四角的黑色物理圆角 mask。

## 启动候选与零写入边界

ABI 53 不再把 `fw_cfg` 中出现 source 等同于安装。启动阶段先对 immutable source
执行完整接纳：

- ZIP/APK envelope、central/local tuple、DEFLATE/data descriptor 和 CRC。
- 二进制 Manifest、package/activity/version、权限计数。
- APK Signature Scheme v2、单 signer 与 certificate digest。
- `resources.arsc`、compiled layout、资源引用和固定 `Resources-1` profile。
- DEX 指令、scene 与 callback 控制流。
- 用隔离 AndroidApp 实际创建一次受支持 Activity session，确保候选不是只能静态
  解析、却无法由当前运行时打开的 APK。

接纳成功后才分类候选。候选发布本身始终满足：

```text
boot_writes=0
boot_flushes=0
first_tap_mutation=0
source_used=0
```

对于相同 APK 的精确 replay 不发布候选；package/activity/signer 冲突、版本回退、
不支持的 profile 或无法创建 Activity session 都 fail closed。virgin volume 不会
因为“发现 source”而自动格式化，格式化只可能发生在用户第二次确认后的事务内。

## ABI 53 契约

### syscall 64：候选读取

`AndroidPackageInstallCandidateRead` 只允许内建 `UserImageId::App` 调用，输出固定
640-byte `BNDICS01` wire。它包含：

- candidate ID 与 `Install / Update / Reinstall` action；
- expected generation/version 和 candidate version；
- APK length、APK SHA-256、signer SHA-256；
- Resources/layout CRC、资源 ID 和 instruction count；
- package、activity、title 和 text 的有界 printable metadata。

空候选只有一种全零 canonical 表示。wire 不包含 APK bytes、指针、路径、block
address、storage handle、blob slot 或任意读取 source 的能力。

### syscall 65：安装事务

`AndroidPackageInstall` 使用固定 256-byte exchange：

```text
request magic=BNDIRQ01
result magic=BNDIRT01
wire version=1
flags=0
profile=Resources1
```

请求绑定：

- 非零、严格有序的 request sequence；
- 非零 operation ID 和 candidate ID；
- action、expected generation/version、candidate version；
- APK length、完整 APK SHA-256、完整 signer SHA-256；
- 1–96 bytes printable package name，未使用尾部必须全零。

成功结果必须复述 sequence、operation 和 candidate ID，证明
`installed_generation = previous_generation + 1`，返回实际 installed version、
APK length、action、两个 digest 与非零 read/write/flush 计数。

第一次提交和精确 pending retry 返回 `ShouldWait` 且不改写 exchange；只有成功才用
`BNDIRT01` 覆盖请求。stale sequence、身份漂移、错误 action、错误 generation、
候选改变或非 canonical bytes 都会在写盘前被拒绝。

## 权限与异步事务

候选读取和安装 syscall 都只授予内建 Settings 所在的 `UserImageId::App`。Launcher、
AndroidApp worker、SurfaceServer 和普通应用均不能调用。App 不获得 package-store、
block、任意路径或 APK blob handle。

SVC 路径只执行固定长度 usercopy、调用者认证、canonical decode、序列检查和入队；
不进行 block I/O。IRQ-enabled kernel monitor 与 relaunch、uninstall、一次性 APK
grant 和 restart escrow 串行化，然后：

1. 重新读取 immutable boot source；
2. 重新执行完整 APK/signature/resources/DEX/Activity-session 接纳；
3. 从 durable store 恢复当前 base；
4. 重新计算候选并与用户确认的完整 candidate identity 比较；
5. 如为 virgin volume，在此时才格式化；
6. 执行 install/update/reinstall 的 generation+1 crash-safe 写入；
7. 完整读回 APK，并再次校验 Resources/Envelope 和 Activity session；
8. 清零 kernel scratch；
9. 只有上述步骤全部成功才原子发布 live catalog，并清除 candidate。

任一 I/O、revalidation 或 identity 错误都不会发布成功。如果底层 I/O outcome
unknown，系统 fail closed，必须通过下次启动的 durable recovery 确认最终状态。

## 更新与重装策略

`Update` 必须保持 package、activity 和 signer certificate identity，且
`candidate.versionCode > installed.versionCode`。成功后 generation 增加 1，
旧 package handle 与旧 Launcher press token 都变为 stale。

`Reinstall` 只允许匹配当前 removed tombstone 的 package/activity/signer，
并要求版本不低于 tombstone 中最后安装版本。它不会绕过 tombstone，也不会把旧
APK blob 重新变为可达；确认后产生一个新的 generation。

ABI 53 当前仍是单包 store。它没有文件选择器、DownloadProvider、content URI、
USB sideload、multi-user package database、per-app managed data 或后台自动更新。
immutable `fw_cfg` 只是当前本地、无网络验证 source，并不是最终手机安装入口。

## 自动门禁

门禁命令：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-runtime-install2.sh
```

脚本只管理自己通过 `$!` 捕获的 QEMU PID，每次 QEMU 都显式使用 `-nic none`。
四次全新启动必须依次证明：

1. virgin volume + v1 source：候选阶段磁盘不变，第一次点击磁盘不变，第二次确认
   安装 generation 1，同启动 Launcher 刷新并真实启动 Activity；
2. 无 source：只读恢复 generation 1，并再次启动 Activity；
3. v2 source：相同 signer 的 update candidate，第一次点击磁盘不变，第二次确认
   更新 generation 2，同启动 Launcher 刷新并启动新 generation；
4. 再次无 source：只读恢复 generation 2，最终磁盘与 update 后完全相同。

通过条件还包括 canonical `720×1600` PPM、关键阶段像素不相同、四次启动全部无网络、
无 panic/fatal/worker failure，以及终端记录
`ANDROIDBOX_RUNTIME_INSTALL2_QEMU_OK`。

最新成功证据位于：

```text
target/androidbox-runtime-install2/check.XFneWQ/
terminal=ANDROIDBOX_RUNTIME_INSTALL2_QEMU_OK
abi=53
qemu_starts=4
qemu_network=disabled
confirmation=two-step
first_tap_mutation=0
install_generation=1
update_generation=2
same_boot_launcher_refresh=1
isolated_android_app_launch=1
source_free_recovery=1
source_free_writes=0
storage_handle_granted=0
block_handle_granted=0
arbitrary_path=0
```

持久事务与恢复 I/O：

```text
install v1: reads=1032 writes=130 flushes=3
source-free v1 recovery: reads=389 writes=0 flushes=0
update v2: reads=905 writes=129 flushes=2
source-free v2 recovery: reads=645 writes=0 flushes=0
```

磁盘 SHA-256 证明 candidate/首次点击没有 mutation、install 和 update 各自改变
durable state，而最后的无 source 恢复保持 update 结果：

```text
initial=b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801
installed=bc102fb86c811bac5f0380dfa05d0b6df78d27dc433faf829ca7d5d84c3fbe6b
updated=f1fd13cec1a0c06c8a2d22172423ab88e89f36cc8b4c1609992d7f9841e8bf38
final=f1fd13cec1a0c06c8a2d22172423ab88e89f36cc8b4c1609992d7f9841e8bf38
```

门禁保存 12 张 canonical 720×1600 raster，覆盖 install ready/confirm/done、
同启动 drawer/Activity、无源恢复 drawer/Activity，以及 update
ready/confirm/done/drawer/Activity。关键 Settings 阶段具有不同 SHA-256；
install、recovery-v1 和 update 三次真实 Activity 启动都完成 5 个 layered commits，
RPC errors 为 0。

## 回归与静态检查

当前源码已通过：

```text
bndr-abi ABI 53 host tests: 53/53
bndr-abi ABI 52 parent host tests: 51/51
bndr-ui ABI 53 host tests: 255/255
bndr-ui ABI 52 parent host tests: 253/253
bndr-package-store host tests: 43/43
cargo fmt --all -- --check
ABI 53 kernel/init/ABI/UI Clippy: passed with -D warnings
scripts/check-androidbox-runtime-install2.sh: bash syntax passed
```

ABI 53 允许 `source_present=1, source_used=0`，用于表达“候选已完整接纳，但用户尚未
确认”。ABI 52 及更早父 profile 继续拒绝该状态，避免悄悄改变历史 wire 语义。

## 明确不声称的能力

```text
art=0 dalvik=0 activitythread=0 android_framework=0
binder=0 bionic=0 jni=0 native_lib=0
permissions=0 services=0 arbitrary_apk=0
multi_package=0 managed_app_data=0 background_update=0
general_android_compatibility=0 aosp_system=0
physical_device_boot=0 real_phone=0
```

因此，ABI 53 证明的是一个真实 SDK APK 在现有严格有界兼容 profile 中，由用户明确
确认完成安装、更新、持久恢复和启动；它不是通用 Android PackageManager，也不代表
一般 Android App 已经可运行。迈向广泛 Android 兼容最终仍需要明确授权后评估
AOSP/ART/Bionic/Binder/Framework 的集成路线。
