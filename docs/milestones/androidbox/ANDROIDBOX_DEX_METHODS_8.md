# AndroidBox DEX Methods-8（ABI 58）

## 结论

opt-in profile `androidbox-dex-methods8` 是 ABI57
`androidbox-density-icons7` 的 child。它让真实 Android SDK/D8 生成的 APK 在
`onClick(View)` 中执行一次 APK 自己定义的方法，再把该方法返回的字符串资源 ID
交给已有的 `TextView.setText(int)` 路径。

本里程碑实际运行的调用形状是：

```java
private static int statusTextFor(int viewId) {
    return viewId == R.id.action_approve
            ? R.string.status_approved
            : R.string.status_rejected;
}
```

这不是把 Android Framework、ART 或 Dalvik 移植进 Bndroid，而是在现有无分配、
fail-closed DEX 执行器内增加一个明确受限、可审计的方法边界。ABI57 及更早 profile
继续接受原来的直接 callback 形状，ABI58 fixture 才启用该方法调用。

本轮只使用 Mac 已安装的 Android SDK 36/JDK 与项目已有代码。没有联网、没有下载
AOSP、没有修改 Android SDK，也没有操作用户正在运行的 ABI48 预览虚拟机。

## 接纳边界

ABI58 只接纳 callback 内的一次 `invoke-static`，目标必须同时满足：

- owner 是当前 Manifest 选中的同一个 Activity；
- access flags 精确为 `private static`；
- prototype 精确为 `(I)I`；
- 每次 callback 最多调用一次，最大调用深度为 1；
- helper 最多 8 个寄存器、96 个 code units，callback 与 helper 合计最多执行
  96 条指令；
- 整个 helper CFG 都必须从入口可达且无环，必须存在返回路径；
- 返回值必须是非零资源 ID，并继续通过现有资源表和目标 `TextView` 校验。

helper 只支持以下 DEX 指令：

```text
move
return
const/4
const/16
const
const/high16
goto
if-eq
if-ne
add-int/lit8
```

未知 opcode、跨类调用、virtual/interface/direct 调用、实例或静态字段、对象、数组、
异常、循环、递归、第二次调用、错误签名、错误 access flags、不可达指令或非法资源
结果都会 fail closed。helper 没有 Bndroid handle，不能发 syscall，也没有存储、
块设备或其他系统权限。

## ABI 与进程边界

系统 ABI 版本提升到 58。AndroidApp 隔离进程协议由 `BNDAPC03` v3 提升为
`BNDAPC04` v4，固定帧大小不变。`Updated` 消息的 `arg0` 被明确划分为：

```text
arg0[31:0]   changed TextView resource ID
arg0[63:32]  APK-defined call count，canonical 值为 0 或 1
```

AndroidApp worker 产生该 provenance；App 端重新校验；kernel tracer 在 ABI58
完整门禁中要求两个 callback transcript 都精确为 1。大于 1 的值被 ABI decoder
拒绝，不能伪装成合法更新。ABI57 父档仍使用 `BNDAPC03` v3 且调用计数为 0。

解释器的大对象继续避开 64-KiB EL0 栈。ABI58 调试中还把 `ActivityScene` 参数改成
借用传递；AArch64 release 反汇编中 `execute_on_click` 栈帧由 2,576 bytes 降至
480 bytes，原有 guard page 和栈大小均未放宽。

## 真实 SDK APK

Envelope 与 Catalog fixture 新增独立的 `src-methods` Java 源目录。历史
`BNDROID_ANDROID_DEX_METHODS=0` 输出保持原形状；ABI58 门禁显式设置
`BNDROID_ANDROID_DEX_METHODS=1`：

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=1 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-methods8/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12805
apk0_sha256=767c49d58d53698fefad2d072de52f506130553b3f3011618fab8af6cc912f7b

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12728
apk1_sha256=35f21ef72c456699a241eb1c8559e45f2499924a51d956db83cc7316710d51dd
```

两个 APK 都由 AAPT2、javac、D8、zipalign 与 apksigner 在本机离线构建；两个
callback 的静态 DEX 审计都包含一次 `invoke-static`。系统门禁安装、恢复并启动
两个包，并在 Catalog Activity 中实际点击两个 Button，分别执行同一个 APK-owned
helper 的两个分支。

## 自动验证

Host 与静态检查：

```text
bndr-androidbox=108/108
bndr-abi=59/59
bndr-ui=263/263
cargo fmt --all -- --check=passed
shell syntax=passed
AArch64 Clippy -D warnings:
  bndr-androidbox=passed
  bndr-abi=passed
  bndr-ui=passed
AArch64 release kernel/userspace QEMU build=passed
```

ABI58 三启动、全程 `-nic none`、720×1600 QEMU 门禁：

```text
terminal=ANDROIDBOX_DEX_METHODS8_QEMU_OK
evidence=target/androidbox-dex-methods8/coexist.RZo1VV
abi=58
capacity=2
rpc_protocol=BNDAPC04
rpc_protocol_version=4
app_defined_method_transcripts=2
app_defined_calls_per_transcript=1/1
dex_methods_profile_records=3
dex_method_owner=same-activity
dex_method_access=private-static
dex_method_proto=I-to-I
status_only_updates=4
trusted_chrome_unchanged=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
aosp_downloaded=0
storage_handle_granted=0
block_handle_granted=0
```

门禁在安装启动与无 source 恢复启动中都实际点击 Catalog 的两个 Button。四次
状态变化都只落在 status TextView 边界内，应用栏、图标、按钮与系统 chrome
逐像素不变：

```text
live_approve_changed_pixels=3740
live_reject_changed_pixels=4041
recovery_approve_changed_pixels=3740
recovery_reject_changed_pixels=4041
```

ABI57 父版本也用当前源码重新完成三启动门禁：

```text
terminal=ANDROIDBOX_DENSITY_ICONS7_QEMU_OK
evidence=target/androidbox-density-icons7/coexist.Zk3rxj
abi=57
rpc_protocol=BNDAPC03
rpc_protocol_version=3
app_defined_method_transcripts=0
source_free_recovery=1
source_free_writes=0
qemu_network=disabled
```

## 仍未完成

ABI58 证明的是一个真实 APK 的、受限 APK-owned Java 方法可以在 Bndroid 的隔离
AndroidApp 路径中被解析和执行。它不包含 ART/Dalvik、ActivityThread、Binder、
Bionic、JNI、native library、完整 Android Framework/system services、通用
PackageManager、权限实现、任意 APK 兼容或实体手机驱动。

因此当前仍不能宣称“Android App 普遍兼容”或“真实手机系统已完成”。如果未来选择
AOSP/ART 级路线，需要先明确 Android 基线、源码与预编译组件体积、许可证、构建
资源和系统架构，并在取得用户明确授权后才下载外部内容。
