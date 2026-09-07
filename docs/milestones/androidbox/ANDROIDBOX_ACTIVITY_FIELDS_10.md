# AndroidBox Activity Fields-10（ABI 60）

## 结论

opt-in profile `androidbox-activity-fields10` 是 ABI59
`androidbox-dex-instance9` 的 child。真实 Android SDK/D8 APK 现在可以在
`onCreate(Bundle)` 中把布局里的 `TextView` 保存到 Activity 私有实例字段，并在
后续 `onClick(View)` 中从同一个 Activity 会话取回该引用：

```java
private TextView statusView;

protected void onCreate(Bundle state) {
    super.onCreate(state);
    setContentView(R.layout.activity_catalog);
    statusView = (TextView) findViewById(R.id.status);
    // register Button listeners
}

public void onClick(View view) {
    statusView.setText(statusTextFor(view));
}
```

本机 D8 9.0.3-dev 的真实输出为：

```text
onCreate:
  findViewById(status)
  move-result-object
  check-cast TextView
  iput-object value, Activity, SameActivity.statusView:TextView

onClick:
  iget-object target, Activity, SameActivity.statusView:TextView
  invoke-direct {Activity, clickedView}, statusTextFor:(View)I
  move-result resourceId
  invoke-virtual {target, resourceId}, TextView.setText:(I)V
```

Catalog 的 `onCreate` 为 2 registers、2 inputs、2 outgoing slots、39 code
units；`onClick` 为 3 registers、2 inputs、2 outgoing slots、10 code units。
ABI59 的实例 helper 仍为 3 registers、2 inputs、1 outgoing slot、16 code
units。

这一步实现的是有所有权和生命周期验证的单个 Activity 对象字段，不是把
`iget/iput` 当作无类型整数。字段值在一次 retained Activity session 中保存，
安装启动和无 source 恢复启动的两个点击均读取同一个已验证 scene `TextView`
引用。

本轮只使用 Mac 已安装的 Android SDK 36/JDK 与仓库代码；没有联网、没有下载
AOSP、没有修改 Android SDK，也没有操作用户持续运行的 ABI48 预览虚拟机。

## 接纳边界

ABI60 的字段型 Activity 必须同时满足：

- 没有 static field，instance field 精确为一个；
- owner 是 Manifest 选中的同一个 Activity；
- access flags 精确为 `private`；
- field type 精确为 `Landroid/widget/TextView;`；
- `onCreate` 通过当前 Activity 的 `findViewById(int)` 得到 scene 内非零
  `TextView`，紧接受控 `check-cast` 后以一次 `iput-object` 写入；
- `iput-object` receiver 必须是当前 Activity，field ID 必须是已声明字段；
- 字段在发布 Activity session 前必须已初始化；
- `onClick` 以一次 `iget-object` 从当前 Activity 和同一 field ID 读取；
- 读出的引用必须仍指向 retained scene 中的同一个 `TextView`；
- 声明字段的 callback 必须精确读取一次，再执行既有受控
  `TextView.setText(int)` 路径。

无字段的 ABI59 实例 helper、ABI58 静态 helper 和更早直接 callback 在 ABI60
child 中继续接纳。宿主探针以 ABI60 当前解释器执行父 APK，分别得到：

```text
ABI59 parent: app_calls=1 instance_calls=1 field_reads=0
ABI58 parent: app_calls=1 instance_calls=0 field_reads=0
```

第二个字段、static field、primitive/array/其他对象字段、跨类 field、错误
receiver、未初始化读取、重复写入、callback 写字段、伪造 scene 引用，以及所有
未知 field opcode 都会 fail closed。DEX 代码仍没有 Bndroid handle、syscall、
存储或块设备权限。

## BNDAPC06 进程协议

系统 ABI 版本提升到 60。AndroidApp 隔离进程协议从 `BNDAPC05` v5 提升为固定
大小不变的 `BNDAPC06` v6。`Updated.arg0` 编码：

```text
arg0[31:0]   changed TextView resource ID
arg0[39:32]  total APK-defined call count，0..1
arg0[47:40]  instance call count，0..total
arg0[55:48]  Activity field read count，0..1
arg0[63:56]  reserved，必须为 0
```

AndroidApp worker、ABI decoder、App client 和 kernel tracer 独立解析并校验。
ABI60 门禁要求两个 callback transcript 都精确为
`total=1, instance=1, field_reads=1`。非 canonical count 或 reserved bits 在
到达 UI 前被拒绝。

## 真实 SDK APK

Envelope 与 Catalog fixture 新增独立 `src-fields` 源目录。
`BNDROID_ANDROID_DEX_METHODS=3` 只选择 ABI60 字段形状；mode 0、ABI58 mode 1
和 ABI59 mode 2 均不被替换：

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=3 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-fields10/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12805
apk0_sha256=386f69b770306d3ad999f4dd0731f1fafbcd96daab083ea67ee0f044049efd56

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12728
apk1_sha256=f198844798483f5df837cad061ccbbce8140fd0dd2f49b059ad947a64c03ca42
```

两个 APK 均由 AAPT2、javac、D8、zipalign 和 apksigner 在本机离线、确定性构建。
构建门禁静态审计字段 owner/type/access、精确一个 `iput-object` 和一个
`iget-object`；系统门禁安装、恢复并启动两个包，并实际点击 Catalog Activity
的两个 Button。

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

ABI60 三启动、全程 `-nic none`、720×1600 QEMU 门禁：

```text
terminal=ANDROIDBOX_ACTIVITY_FIELDS10_QEMU_OK
evidence=target/activity-fields10/coexist.bPNLji
abi=60
capacity=2
rpc_protocol=BNDAPC06
rpc_protocol_version=6
app_defined_method_transcripts=2
app_defined_calls_per_transcript=1/1
app_defined_instance_calls_per_transcript=1/1
activity_field_reads_per_transcript=1/1
dex_methods_profile_records=3
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

ABI59 父版本以当前源码重新完成三启动门禁：

```text
terminal=ANDROIDBOX_DEX_INSTANCE9_QEMU_OK
evidence=target/androidbox-dex-instance9/coexist.9XUTny
abi=59
rpc_protocol=BNDAPC05
rpc_protocol_version=5
activity_field_reads_per_transcript=0/0
source_free_recovery=1
source_free_writes=0
qemu_network=disabled
```

## 仍未完成

ABI60 只证明一个私有 `TextView` Activity 字段可跨 `onCreate` 与 callback 安全
保留。当前仍没有多个/通用字段、对象分配、构造任意对象、任意 method graph、
class initialization、garbage collection、线程、异常、reflection、ART/Dalvik、
ActivityThread、Binder、Bionic、JNI、native library、完整 Android
Framework/system services、通用 PackageManager、权限实现、任意 APK 兼容或
实体手机驱动。

因此仍不能宣称“普遍兼容 Android App”或“真实手机系统完成”。进入 AOSP/ART
路线前仍需用户明确授权下载外部源码或预编译组件。
