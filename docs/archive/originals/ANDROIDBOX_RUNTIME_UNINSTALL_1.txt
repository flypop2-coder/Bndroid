# AndroidBox Runtime Uninstall-1（ABI 52）

## 结论

opt-in profile `androidbox-runtime-uninstall1` 已通过离线 ABI 52 门禁。它是
ABI 51 `androidbox-apk-envelope4` 的 child，在现有单包、Resources-1 兼容范围内，
第一次允许用户从真实的“设置 → 应用”界面卸载当前 APK，并在没有 APK source 的
下一次启动中恢复持久化删除状态。

本里程碑继续使用本机 Android SDK/JDK 构建的真实 APK：

```text
package=org.bndroid.envelope
activity=Lorg/bndroid/envelope/MainActivity;
versionCode=1
bytes=12646
sha256=a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
signature=APK Signature Scheme v2 only, one signer
permissions=0
```

整个构建和验证过程设置了 `CARGO_NET_OFFLINE=true`，QEMU 均使用 `-nic none`。
没有下载 AOSP，也没有向系统引入网络能力。

## 用户界面

设置应用现在从 kernel 发布的实时、只读包目录显示：

- 已安装包名称、package、version、APK size、generation。
- APK v2 验签状态、Resources-1 profile。
- APK 与 signer digest 前缀。
- All apps 启动入口和 Launcher Activity。
- 独立的橙色“Uninstall app”操作卡。

卸载不是单击即执行：

1. 第一次点击只进入本地 `Confirming` 状态并显示确认对话框。
2. 用户可取消；取消不会调用 syscall，也不会写盘。
3. 第二次点击“Uninstall”才进入 `Pending` 并提交完整包身份。
4. 内核成功返回且实时目录已经为空后，才显示 `App uninstalled`。
5. Stale、Busy、Storage、Verification 各有独立失败状态，UI 不会从按钮点击推断成功。

页面继续使用真机尺寸的固定 `720×1600`、20:9 scanout，保留圆角屏幕、
64-pixel status bar、88-pixel navigation bar、滚动裁剪和至少 44px 设计尺寸的
触控目标。

## ABI 52 契约

syscall 63 `AndroidPackageUninstall` 只接受一个可读写的精确 256-byte exchange：

```text
request magic=BNDURQ01
result magic=BNDURT01
wire version=1
flags=0
profile=Resources1
data disposition=no-managed-package-data
```

请求绑定：

- 非零、严格有序的 request sequence。
- 非零 operation ID。
- 当前 generation、version code、APK length。
- 完整 APK SHA-256 和 signer certificate SHA-256。
- 1–96 bytes printable package name，尾部必须全零。

请求不含路径、block address、storage handle、blob slot 或任意数据删除选择器。
成功结果必须复述 sequence/operation ID，证明
`removal_generation = last_installed_generation + 1`，并包含非零
read/write/flush 计数。Pending 和失败调用不得改写请求 bytes。

## 权限与异步 I/O

syscall 63 只允许唯一存活的内建 `UserImageId::App` 调用。Launcher、
独立 AndroidApp worker 和 SurfaceServer 均被拒绝。App 没有获得 package-store
handle、block handle、任意路径权限或 APK blob 地址。

SVC 路径只做固定长度 usercopy、身份认证、请求校验和入队，不进行 block I/O。
IRQ-enabled mobile monitor 才执行恢复、完整 durable identity 复验和双 registry
tombstone 写入；App 对原请求做精确重试并以 `wfi` 让出执行。以下状态会 fail closed：

- 普通一次性 APK grant 仍存在。
- restart escrow/reissue 仍存在。
- relaunch service 非空。
- live catalog、boot durable readback 与请求的任意身份字段不一致。
- I/O outcome unknown；此时 live publication 标记失败，必须重启恢复磁盘后再使用。

成功后 live catalog 原子切换为 `installed=None, package=None, removed=Some(...)`。
Launcher 与 Settings 随后的 snapshot 读取立即看不到包；重启则从双 tombstone 恢复
相同 removal generation。

## 当前删除语义

当前系统还没有每应用 managed data directory。因此 ABI 52 的数据处置固定为：

```text
data_disposition=no-managed-package-data
logical_apk_reachable=0
apk_blob_erased=0
```

也就是说，包目录和启动权限立即撤销，旧 APK blob 不再可达，但本里程碑没有声称
对旧 blob 做物理擦除。以后实现包数据目录、空间回收或安全擦除时，需要新的明确
存储事务和权限模型，不能把这些能力隐含进 syscall 63。

## QEMU 证据

门禁命令：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-runtime-uninstall1.sh
```

最新成功证据：

```text
target/androidbox-runtime-uninstall1/check.XIPcDQ/
terminal=ANDROIDBOX_RUNTIME_UNINSTALL1_QEMU_OK
abi=52
qemu_starts=3
qemu_network=disabled
caller=system-app
request_sequence=1
last_generation=1
removal_generation=2
durable_io=reads:264,writes:2,flushes:2
```

三个全新启动分别证明：

1. 由显式 `fw_cfg` source 安装 generation 1。
2. 完全没有 APK source，从 Settings 打开 Apps，第一次点击只显示确认框，
   第二次点击才执行卸载；live snapshot 立即变为 `installed=0`。
3. 再次没有 source 或卸载请求，从磁盘恢复
   `operation=removed-recovery removal_generation=2`，reads 非零而 writes/flushes
   均为零，磁盘 SHA-256 与卸载完成后完全相同。

磁盘证据：

```text
installed=bc102fb86c811bac5f0380dfa05d0b6df78d27dc433faf829ca7d5d84c3fbe6b
removed=f3e964519f7faf11ca4332ba50b9545fbbd524081c946f4677d4a72610f8976b
reboot=f3e964519f7faf11ca4332ba50b9545fbbd524081c946f4677d4a72610f8976b
```

门禁保存四张 canonical 720×1600 raster：已安装页、确认框、当前会话卸载完成页、
重启后的空目录页。截图等待条件同时检查 Apps/确认页固定像素签名，避免只按帧数
抓到前一页。

## 回归与静态检查

当前源码已通过：

```text
bndr-abi ABI 52 host tests: 51/51
bndr-ui ABI 52 host tests: 253/253
cargo fmt --all -- --check
ABI 52 kernel/init/ABI/UI clippy: passed with -D warnings
ABI 51 parent QEMU: target/androidbox-envelope4/check.bTyNM1/
```

Clippy 命令只保留父 profile 已存在的有限 lint baseline；ABI 52 新增的
`large_enum_variant` 没有被屏蔽，而是把 terminal completion 缩成紧凑的
sequence/operation/generation/I/O 结构。持久写入前的完整 package/version/length/
digest/signer identity 验证不变。

## 明确不声称的能力

```text
art=0 dalvik=0 activitythread=0 android_framework=0
binder=0 bionic=0 jni=0 native_lib=0
permissions=0 services=0 arbitrary_apk=0
managed_app_data=0 physical_blob_erasure=0
general_android_compatibility=0 aosp_system=0
physical_device_boot=0 real_phone=0
```

因此，ABI 52 证明的是已有有界 APK profile 的真实 Settings 卸载与持久恢复，
不是通用 Android PackageManager，也不是所有 Android App 已兼容。进入
ART/Bionic/Binder/Framework、通用 PackageManager 或实体设备适配前，仍需先明确
范围、磁盘预算和授权。
