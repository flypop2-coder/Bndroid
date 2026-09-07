# AndroidBox APK Envelope-4（ABI 51）

## 结论

ABI 51 的 opt-in profile `androidbox-apk-envelope4` 已通过完整离线门禁。
它在 ABI 50 `androidbox-multiaction3` 上增加了一个严格有界的常见 APK
ZIP 外壳读取器，使未经“全部解压并重打包为 STORED”处理的 Android SDK
构建产物可以进入现有 AndroidBox 兼容路径。

本里程碑实际运行的 APK 是：

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

它由本机已有 Android SDK/JDK 完全离线构建两次，两份签名 APK
逐字节相同。构建完成后没有用自定义 ZIP 工具改写、解压或重新打包 APK。

## 被接纳的 APK 外壳

fixture 的中央目录顺序和压缩方法为：

```text
AndroidManifest.xml                 method=8
res/layout/activity_envelope.xml    method=8
resources.arsc                      method=0
classes.dex                         method=0
assets/envelope-note.txt            method=8
```

其中 Manifest、layout 和无关 asset 使用原始 DEFLATE；三个条目带 bit-3
data descriptor。Android SDK/zipalign 生成的本 fixture 在 local header 中保留
与 central directory 完全一致的 CRC/size tuple，读取器同时只接纳另一种标准形状：
local tuple 全零。`classes.dex` 与 `resources.arsc` 继续要求 STORED，且 STORED
数据仍要求 4-byte alignment。

Envelope-4 的 ZIP 边界为：

- 最多 32 个条目，拒绝 ZIP64、加密、重复必需条目和 local-data 范围重叠。
- local header 的 name、method、flags 和 bit-3 tuple 必须与 central directory
  严格一致；descriptor 可有或没有 `0x08074b50` signature，但 CRC 和两个 size
  必须完全匹配。
- Manifest 和已选择 layout 的解压输出各自最多 32,768 bytes。
- DEFLATE 必须恰好消费声明的 compressed input、恰好产生声明的 output size，
  并通过解压后 CRC-32。
- 无关的 DEFLATE 条目只做结构计数，不解压、不解释，也不会获得运行权限。
- APK v2 验签仍在容器解释之前完成；Envelope reader 本身不替代真实性策略。

## 工作区与栈安全

DEFLATE 使用调用者拥有的 `AndroidBoxEnvelopeScratch`：

```text
xml_output_bytes=32768
miniz_oxide_0.8.9_state_bytes=10504
scratch_total_bytes=43272
storage=AndroidApp-private BSS
```

整个 43,272-byte 工作区在每次加载前后原地清零，成功和失败路径相同。
`miniz_oxide` 精确固定为 `=0.8.9`；64 位 inflater layout 固定为 10,504 bytes。
这是为了让经审计的全零初始化保持有效，并防止约 10 KiB inflater 状态再次进入
64 KiB EL0 调用栈。

第一次 QEMU 验证确实发现了两个独立的栈问题，均保留了失败证据：

1. `target/androidbox-envelope4/check.GvqoKX/` 中，inflater 状态位于
   `decode_envelope_entry` 栈帧。
2. inflater 移出栈后，`target/androidbox-envelope4/check.iqxrJ1/` 暴露出
   `ActivitySession` 同时保存完整 `AndroidBox.cached_layout` 和独立运行时
   `scene` 的重复所有权。

最终 `ActivitySession` 只保留点击运行真正需要的 `Program`、
`ApkResources` 和独立 scene，从 5,912 bytes 降为 3,232 bytes，并有
`<= 4 KiB` 编译期断言。正式 release ELF 的最深已知路径为：

```text
worker_loop              42992 bytes
load_with_scratch         8416 bytes
manifest::parse           1072 bytes
entry/call overhead         48 bytes
known path total         52528 bytes
64-KiB stack headroom    13008 bytes
```

更深的 layout 路径仍保留约 6 KiB 余量。未经优化的 debug AndroidApp ELF
不属于 ABI 51 支持配置：其固定容量临时值仍会使同一 64 KiB 栈超限。自动门禁与
发布镜像均使用 release。

## 已运行的 Android 行为

APK 的真实二进制 Manifest 选择 launcher Activity，真实 `resources.arsc`
解析出 5 个布局节点：

- 1 个 vertical `LinearLayout`
- 2 个 `TextView`
- 2 个注册 callback 的 `Button`

真实 D8 `onClick(View)` 读取 `View.getId()`，在有界无环 CFG 中分别执行：

```text
Approve -> Decision: approved
Reject  -> Decision: rejected
```

场景由独立 AndroidApp EL0 worker 通过 `BNDAPC03/3` pull RPC 交给受信 App
renderer。AndroidApp 不拥有 Surface、graphics、input、storage 或 network
authority；Launcher 和 App 只获得包目录元数据，不获得 APK bytes。只有绑定
当前 session/generation 的 AndroidApp 可以消费一次性只读 APK VMO grant。

## QEMU 证据

门禁命令：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-envelope4.sh
```

最新成功证据：

```text
target/androidbox-envelope4/check.FaIeFG/
terminal=ANDROIDBOX_APK_ENVELOPE4_QEMU_OK
qemu_boots=2
qemu_nic_none_per_boot=1
install_source=qemu-fw_cfg
recovery_source=none
recovery_disk_unchanged=1
unexpected_faults=0
```

第一次启动从显式 `fw_cfg` source 安装 APK。第二次完全不提供 APK source，
仍从同一 16 MiB package disk 读取、复验并启动。门禁还完成：

- 5-node scene、45 bytes scene text、2 个 callback Button。
- Approve/Reject 两个 Click→Updated 分支，RPC errors 为 0。
- 两次更新只改变 status publisher，publisher 外变化为 0 pixels。
- 一次受控旧 worker fault、same-slot generation+1 replacement、same-VMO
  reissue、重新 Open 和再次普通 relaunch。
- 初始与 relaunch 720×1600 raster 逐字节相同。
- source-free recovery 前后 package disk SHA-256 相同。

当前源码还重新通过了父档位：

```text
ABI 50  target/androidbox-multiaction3/check.Upli7z/
ABI 49  target/androidbox-scene-rpc2/check.dvsBl3/
ABI 48  target/androidbox-restart0/check.iSFPZG/
ABI 47  target/androidbox-process0/check.JMErXz/
```

主机离线测试为 AndroidBox ABI 51 `99/99`、init ABI 51 `20/20`、
AndroidBox ABI 50 `89/89`、init ABI 50 `18/18`。

## 明确不声称的能力

```text
art=0 dalvik=0 activitythread=0 android_framework=0
binder=0 bionic=0 jni=0 native_lib=0
permissions=0 services=0 arbitrary_apk=0
general_android_compatibility=0 aosp_system=0
physical_device_boot=0 real_phone=0
```

因此，这证明的是一个常见 SDK APK 外壳能够在现有有界兼容层中安装、持久恢复、
交互和恢复进程，不是“所有 Android APK 已兼容”。本里程碑没有下载 AOSP。
进入 ART/Bionic/Binder/Framework 或实体设备适配前，必须先确定具体范围、磁盘
预算和授权。

