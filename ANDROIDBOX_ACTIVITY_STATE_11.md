# AndroidBox Activity State-11（ABI 61）

## 结论

opt-in profile `androidbox-activity-state11` 是 ABI60
`androidbox-activity-fields10` 的 child。真实 Android SDK/D8 APK 现在可以在
同一个 retained Activity session 中同时保留：

- 一个指向布局 `TextView` 的私有实例字段；
- 一个由 `onCreate` 初始化、由连续 `onClick` 读取、递增和写回的私有 `int`
  实例字段。

fixture 源码的关键形状为：

```java
private TextView statusView;
private int clickCount;

protected void onCreate(Bundle state) {
    super.onCreate(state);
    setContentView(R.layout.activity_catalog);
    statusView = (TextView) findViewById(R.id.status);
    clickCount = 0;
    // register Button listeners
}

public void onClick(View view) {
    clickCount++;
    statusView.setText(statusTextFor(view));
}
```

本机 D8 9.0.3-dev 的真实 callback 输出为：

```text
iget        v0, v1, SameActivity.clickCount:I
add-int/lit8 v0, v0, 1
iput        v0, v1, SameActivity.clickCount:I
iget-object v0, v1, SameActivity.statusView:TextView
invoke-direct {v1, v2}, SameActivity.statusTextFor:(View)I
move-result  v1
invoke-virtual {v0, v1}, TextView.setText:(I)V
return-void
```

两次真实点击分别返回 `activity_int_state_value=1` 和 `2`。这不是根据 revision
推断的计数：解释器实际执行 D8 的 `iget`、`add-int/lit8` 和 `iput`，下一次
callback 再从同一个 Activity 字段读回上一次写入值。

本轮只使用 Mac 已安装的 Android SDK 36/JDK 和仓库源码；全程离线，没有下载
AOSP、没有修改 Android SDK，也没有操作持续运行的 ABI48 Mac 预览虚拟机。

## 接纳与事务边界

ABI61 在 ABI60 的单个私有 `TextView` 字段之外，最多接纳一个私有 `int` 字段：

- 两个字段的 owner 都必须是 Manifest 选中的同一个 Activity；
- access flags 必须精确为 `private`；
- 字段类型必须精确为一个 `Landroid/widget/TextView;` 和可选的一个 `I`；
- 不接纳 static、array、wide、其他 primitive、其他 object 或第三个字段；
- `onCreate` 必须先以既有受控路径写入 `TextView`，并以常量 `0` 对精确的
  `int` 字段执行一次 `iput`；
- `onClick` 必须从当前 Activity 和精确 field ID 执行一次 `iget`；
- `add-int/lit8` 的 literal 必须精确为 `+1`，输入必须是刚读出的 retained
  字段值；
- 随后必须对同一 Activity/field 执行一次 `iput`；
- callback 还必须执行一次 ABI60 `iget-object`，得到同一 retained scene 的
  `TextView`；
- 状态限定为 `1..=255`；溢出、重复读写、错误 receiver、错误 field ID、
  未初始化值或伪造引用全部 fail closed。

scene、两个字段值和 revision 是一个事务。解释或资源解析的任何后续步骤失败时，
不会只提交一部分字段或提前推进 revision。

ABI60 的单 `TextView` 字段、ABI59 实例 helper、ABI58 静态 helper 和更早直接
callback 仍被 child 接纳。对不声明 `int` 字段的父 APK，状态保持 0、字段写次数
保持 0，不会被伪装成 ABI61 状态型 callback。

## BNDAPC07 进程协议

系统 ABI 版本提升到 61。AndroidApp 隔离进程协议从 `BNDAPC06` v6 提升为固定
大小不变的 `BNDAPC07` v7。`Updated.arg0` 编码：

```text
arg0[31:0]   changed TextView resource ID
arg0[39:32]  total APK-defined call count，0..1
arg0[47:40]  instance call count，0..total
arg0[55:48]  Activity field read count，0..2
arg0[63:56]  committed int Activity state，0..255
```

非零 int state 只有在 `total=1`、`instance=1`、`field_reads=2` 时才是
canonical。worker 在封帧前还独立要求真实执行证据
`field_writes=1`；write count 不占用 wire 位。ABI decoder、App client 和
kernel tracer 再分别解码和验证。最终内核门禁精确要求：

```text
first click:  total=1 instance=1 field_reads=2 int_state=1
second click: total=1 instance=1 field_reads=2 int_state=2
```

## 真实 SDK APK

Envelope 与 Catalog fixture 新增独立 `src-state` 源目录。
`BNDROID_ANDROID_DEX_METHODS=4` 只选择 ABI61 双字段形状，不替换 mode 0–3：

```sh
BNDROID_ANDROID_ICON_RESOURCES=2 \
BNDROID_ANDROID_DEX_METHODS=4 \
BNDROID_ENVELOPE_OUTPUT_APK="$PWD/target/activity-state11/envelope.apk" \
./fixtures/androidbox-envelope-demo/build.sh
```

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12805
apk0_sha256=d3692f30f8dcb14d24ca5a52098f37a2266e6d2474e4687108669ed7a80026a1

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12728
apk1_sha256=3308e2abc20696cbaed1b673ae27647dbd2d7b0cb4522ed3a777297e115332b6
```

两个 APK 均由 AAPT2、javac、D8、zipalign 和 apksigner 使用本机已安装工具离线、
确定性构建。构建门禁静态审计字段 owner/type/access、`onCreate` 初始化以及
callback 的 `iget/add-int-lit8/iput/iget-object` 形状。

## 栈故障与修复

第一轮全系统门禁没有被掩盖：AndroidApp worker 在打开 Catalog 时触发低端用户栈
守卫。符号化 ELR 指向 `execute_activity_constructor` 入口；根因是外层打开函数
同时保留验证用和实际使用的两个固定容量 `ActivitySession`，不是构造器算法错误。

修复把验证会话隔离到独立函数，并在创建 retained session 前销毁它。没有增加栈
页、取消 guard、扩大权限或降低验证。修复后的 ABI61 门禁通过，ABI60 直接父门也
以相同源码重新通过。

## 自动验证

```text
bndr-androidbox=109/109
bndr-abi=59/59
bndr-ui=263/263
bndroid-kernel=377/377
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

ABI61 三启动、全程 `-nic none`、720×1600 QEMU 门禁：

```text
terminal=ANDROIDBOX_ACTIVITY_STATE11_QEMU_OK
evidence=target/activity-state11/coexist.DD8YfM
abi=61
capacity=2
rpc_protocol=BNDAPC07
rpc_protocol_version=7
app_defined_method_transcripts=2
app_defined_calls_per_transcript=1/1
app_defined_instance_calls_per_transcript=1/1
activity_field_reads_per_transcript=2/2
activity_int_state_values_per_transcript=1/2
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

安装启动和无 source 恢复启动均实际点击 Catalog 的两个 Button。四次栅格变化
只位于 status TextView，系统 chrome、应用栏、APK 图标和 Button 均不变：

```text
live_approve_changed_pixels=3740
live_reject_changed_pixels=4041
recovery_approve_changed_pixels=3740
recovery_reject_changed_pixels=4041
```

ABI60 直接父版本回归：

```text
terminal=ANDROIDBOX_ACTIVITY_FIELDS10_QEMU_OK
evidence=target/activity-fields10/coexist.dSivl4
abi=60
rpc_protocol=BNDAPC06
rpc_protocol_version=6
activity_field_reads_per_transcript=1/1
activity_int_state_values_per_transcript=0/0
source_free_recovery=1
source_free_writes=0
qemu_network=disabled
```

## 仍未完成

ABI61 只证明一个有界、私有、持久的 Activity `int` 状态和一个 `TextView` 引用
可以执行当前真实 D8 形状。它仍没有通用对象模型、任意字段类型、对象分配、构造
任意对象、class initialization、GC、线程、异常、reflection、完整 method graph、
ART/Dalvik、ActivityThread、Binder、Bionic、JNI、native library、完整 Android
Framework/system services、通用 PackageManager/权限实现、任意 APK 兼容或实体手机
驱动。

因此仍不能宣称“普遍兼容 Android App”或“真实手机系统完成”。如果未来决定进入
AOSP/ART 或容器化 Android 路线，下载外部源码/组件仍需用户另行明确授权。
