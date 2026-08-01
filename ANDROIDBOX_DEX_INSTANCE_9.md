# AndroidBox DEX Instance-9（ABI 59）

## 结论

opt-in profile `androidbox-dex-instance9` 是 ABI58
`androidbox-dex-methods8` 的 child。真实 Android SDK/D8 APK 的
`onClick(View)` 现在可以把 Activity receiver 和真实 clicked `View` 引用传入
APK 自己的私有实例方法；helper 内部执行 `View.getId()`，再把结果映射为字符串
资源 ID：

```java
private int statusTextFor(View view) {
    return view.getId() == R.id.action_approve
            ? R.string.status_approved
            : R.string.status_rejected;
}
```

真实 D8 生成的调用链为：

```text
onClick:
  invoke-direct {Activity, View}, MainActivity.statusTextFor:(View)I
  move-result resourceId
  invoke-virtual {TextView, resourceId}, TextView.setText:(I)V

statusTextFor:
  invoke-virtual {View}, View.getId:()I
  move-result viewId
  if-ne / const / return
```

相比 ABI58 的纯静态 `(I)I` helper，这一步增加了真实对象引用传参、Activity
receiver 校验、私有实例方法解析以及 helper 内的受控 framework virtual call。
它仍是无分配、fail-closed 的 DEX 子集，不是 ART 或 Dalvik。

本轮只使用 Mac 已安装的 Android SDK 36/JDK 和项目已有代码。没有联网、没有下载
AOSP、没有修改 Android SDK，也没有操作用户持续运行的 ABI48 预览虚拟机。

## 接纳边界

ABI59 的新调用必须同时满足：

- callback opcode 精确为 `invoke-direct`；
- receiver 必须是当前 Activity，参数必须是本次实际点击的 `View`；
- helper owner 必须是 Manifest 选中的同一个 Activity；
- access flags 精确为 `private`，不是 static、virtual、native 或 abstract；
- prototype 精确为 `(Landroid/view/View;)I`；
- code item 最多 8 个寄存器、精确 2 个 inputs、精确 1 个 outgoing slot、最多
  96 个 code units；
- 每个 callback 最多一个 APK-defined call，调用深度固定为 1；
- helper 内最多一次 framework call，且只能是 clicked `View.getId()`;
- callback 与 helper 合计最多执行 96 条指令；
- helper 的每条编码指令都必须可达，完整 CFG 必须无环且存在返回路径；
- 非零返回值继续经过 APK 资源表和目标 TextView 校验。

实例 helper 支持：

```text
move
move-result
return
const/4
const/16
const
const/high16
goto
if-eq
if-ne
add-int/lit8
invoke-virtual View.getId
```

ABI58 的同 Activity `private static (I)I` 和更早的直接 callback 在 ABI59 child
中继续保留。跨类调用、第二次调用、递归、循环、字段、对象分配、数组、异常、
monitor、反射、其他 framework 方法、错误 receiver/参数、伪造 View 引用或非法
资源结果都会 fail closed。DEX 代码仍没有 Bndroid handle、syscall、存储或块设备
权限。

## BNDAPC05 进程协议

系统 ABI 版本提升到 59。AndroidApp 隔离进程协议由 `BNDAPC04` v4 提升为固定大小
不变的 `BNDAPC05` v5。`Updated.arg0` 的低 32 位继续是 changed TextView ID；
高 32 位改为结构化 provenance：

```text
arg0[31:0]   changed TextView resource ID
arg0[39:32]  total APK-defined call count，0..1
arg0[47:40]  instance call count，0..total
arg0[63:48]  reserved，必须为 0
```

AndroidApp worker、ABI decoder、App client、App UI path 和 kernel tracer 都独立
校验这些关系。ABI59 门禁要求两个 callback transcript 都精确为
`total=1, instance=1`；静态父形状则是 `total=1, instance=0`。不 canonical 的
计数或 reserved bits 会在到达 UI 前被拒绝。

## 真实 SDK APK

Envelope 与 Catalog fixture 新增独立 `src-instance` 源目录。
`BNDROID_ANDROID_DEX_METHODS=2` 只选择该 ABI59 源形状；历史 mode 0 和 ABI58
mode 1 均不被替换：

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=2 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/androidbox-dex-instance9/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12805
apk0_sha256=00e653e6c66bfcdfb0616db08888f6b7e18e5b43673b388cb46ed5dca6d17a8b

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12728
apk1_sha256=fb3626a14ec6d1ec2b69ff012098e894f068c9566b53b75acb00522310cece10
```

两个 APK 均由 AAPT2、javac、D8、zipalign 和 apksigner 在本机离线、确定性构建。
静态 DEX 审计证明两个 callback 都是 `invoke-direct(Activity, View)`，helper
精确为 3 registers、2 inputs、1 outgoing、16 code units。系统门禁安装、恢复并
启动两个包，并实际点击 Catalog Activity 的两个 Button。

## 自动验证

```text
bndr-androidbox=109/109
bndr-abi=59/59
bndr-ui=263/263
cargo fmt --all -- --check=passed
shell syntax=passed
AArch64 Clippy -D warnings:
  bndr-androidbox=passed
  bndr-abi=passed
  bndr-ui=passed
  bndroid-init=passed
  bndroid-kernel=passed
AArch64 release kernel/nine-userspace build=passed
```

ABI59 三启动、全程 `-nic none`、720×1600 QEMU 门禁：

```text
terminal=ANDROIDBOX_DEX_INSTANCE9_QEMU_OK
evidence=target/androidbox-dex-instance9/coexist.9XUTny
abi=59
capacity=2
rpc_protocol=BNDAPC05
rpc_protocol_version=5
app_defined_method_transcripts=2
app_defined_calls_per_transcript=1/1
app_defined_instance_calls_per_transcript=1/1
dex_methods_profile_records=3
dex_method_owner=same-activity
dex_method_access=private-instance
dex_method_proto=View-to-I
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

安装启动和无 source 恢复启动都实际点击 Catalog 的两个 Button。四次变化只位于
status TextView；应用栏、真实 APK 图标、按钮和系统 chrome 逐像素不变：

```text
live_approve_changed_pixels=3740
live_reject_changed_pixels=4041
recovery_approve_changed_pixels=3740
recovery_reject_changed_pixels=4041
```

ABI58 父版本也以当前源码重新完成三启动门禁：

```text
terminal=ANDROIDBOX_DEX_METHODS8_QEMU_OK
evidence=target/androidbox-dex-methods8/coexist.RZo1VV
abi=58
rpc_protocol=BNDAPC04
rpc_protocol_version=4
app_defined_calls_per_transcript=1/1
app_defined_instance_calls_per_transcript=0/0
source_free_recovery=1
source_free_writes=0
qemu_network=disabled
```

## 仍未完成

ABI59 证明真实 APK 的 Activity 实例方法可以接收并使用一个真实 View 引用。当前
仍没有 Activity 字段、对象分配、任意 method graph、class initialization、
garbage collection、线程、异常、reflection、ART/Dalvik、ActivityThread、
Binder、Bionic、JNI、native library、完整 Android Framework/system services、
通用 PackageManager、权限实现、任意 APK 兼容或实体手机驱动。

因此仍不能宣称“普遍兼容 Android App”或“真实手机系统完成”。进入 AOSP/ART
路线前仍需用户明确授权下载外部源码或预编译组件，并先确定 Android 基线、体积、
许可证、构建资源和与 Bndroid 内核的隔离架构。
