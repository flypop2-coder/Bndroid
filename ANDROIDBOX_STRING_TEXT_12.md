# AndroidBox String Text-12（ABI 62）

## 结论

opt-in profile `androidbox-string-text12` 是 ABI61
`androidbox-activity-state11` 的 child。真实 Android SDK/D8 APK 现在可以在
连续 Activity callback 中执行：

```java
private TextView statusView;
private int clickCount;

public void onClick(View view) {
    int next = clickCount + 1;
    clickCount = next;
    statusView.setText(
        next == 1 ? "First review recorded" : "Review state advanced again"
    );
}
```

本机 D8 9.0.3-dev 为 Catalog 生成的真实形状是：

```text
iget          v3, v2, MainActivity.clickCount:I
const/4       v0, #int 1
add-int/2addr v3, v0
iput          v3, v2, MainActivity.clickCount:I
iget-object   v1, v2, MainActivity.statusView:TextView
if-ne         v3, v0, second-string
const-string  v3, "First review recorded"
goto          set-text
second-string:
const-string  v3, "Review state advanced again"
set-text:
invoke-virtual {v1, v3}, TextView.setText:(CharSequence)V
return-void
```

解释器实际执行 `const/4`、`add-int/2addr`、`if-ne`、`const-string` 和精确的
`TextView.setText(CharSequence)` overload。第一次与第二次点击分别显示 APK
自己的 21/27 字节字符串，并提交 int state `1/2`。这不是把字符串预先放进
系统资源表，也不是根据 revision 伪造结果。

本轮只使用 Mac 已安装的 Android SDK 36/JDK 与仓库源码，全程离线，没有下载
AOSP、没有修改 Android SDK，也没有操作持续运行的 ABI48 Mac 预览虚拟机。

## 接纳边界

- `const-string` 必须引用当前 `classes.dex` 中有效、非空、有界的 ASCII 字符串；
- DEX 寄存器只保留 32 位 string index，避免把最大字符串复制进每个寄存器；
- 仅接受精确的 `Landroid/widget/TextView;.setText:(Ljava/lang/CharSequence;)V`；
- receiver 必须是 retained Activity 私有字段指向的同一 scene `TextView`；
- `const/4` 必须精确产生 `1`，`add-int/2addr` 必须把刚读出的 retained int
  与这个 `1` 相加；
- `if-ne` 比较提交候选状态与 `1`，两个分支都必须属于完整、可达、无环 CFG；
- 每个 callback 必须精确读取两个字段、写回一个 int 字段并产生一个文本变更；
- scene、字段和 revision 仍作为一个事务提交；解析、分支、overload 或文本验证
  任一步失败都不会部分更新。

ABI61 的 `setText(int)`、实例 helper、静态 helper 与更早的直接资源 callback
仍由 child 接纳。direct-string 路径的 `text_resource_id` 精确为 0；父路径仍为
非零 Android resource ID，二者不会混淆。

## BNDAPC08

系统 ABI 提升为 62，固定大小进程协议升级为 `BNDAPC08` v8：

```text
arg0[31:0]   changed TextView ID
arg0[39:32]  total APK-defined calls
arg0[47:40]  instance APK-defined calls
arg0[54:48]  Activity field reads
arg0[55]     direct DEX string text
arg0[63:56]  committed int Activity state
```

direct-string 状态 callback 的 canonical 形状必须为：

```text
total=0 instance=0 field_reads=2 direct_string=1 int_state=1..255
```

父 ABI61 状态 callback 保持：

```text
total=1 instance=1 field_reads=2 direct_string=0 int_state=1..255
```

ABI decoder、AndroidApp worker、App client 与 kernel tracer 分别验证这组
约束。字段写次数不占 wire 位，但 worker 仍要求精确为 1。

## 真实 APK 与自动门禁

Envelope 和 Catalog fixture 新增独立 `src-string` 源目录。
`BNDROID_ANDROID_DEX_METHODS=5` 选择 ABI62 形状：

```text
package0=org.bndroid.envelope
apk0_bytes=12805
apk0_sha256=b0c133b302ada2e336060259b142c2b7fd27b2d2db587c208553057f35505d2d

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=760376cd8d4e408f6c08f3c13d5b3b91c0289928fc5240cec8dc8d66418666a9
```

```sh
./scripts/check-androidbox-string-text12.sh
```

最终源码对应的三启动、720×1600、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_STRING_TEXT12_QEMU_OK
evidence=target/string-text12/coexist.OLePzg
abi=62
rpc_protocol=BNDAPC08
rpc_protocol_version=8
app_defined_calls_per_transcript=0/0
app_defined_instance_calls_per_transcript=0/0
activity_field_reads_per_transcript=2/2
activity_int_state_values_per_transcript=1/2
direct_string_texts_per_transcript=true/true
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

四次状态更新的像素差只位于 status TextView：

```text
live_approve_changed_pixels=2990
live_reject_changed_pixels=4277
recovery_approve_changed_pixels=2990
recovery_reject_changed_pixels=4277
```

最终检查：

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
release kernel/nine-userspace build=passed (QEMU gate build)
```

ABI61 父门以当前共享实现重新通过：

```text
terminal=ANDROIDBOX_ACTIVITY_STATE11_QEMU_OK
evidence=target/activity-state11/coexist.PHkwhd
```

## 门禁发现的问题

第一轮 ABI62 QEMU 在 AndroidApp 打开路径触发低栈守卫。根因是最初设计把
256 字节字符串值复制进八个解释器寄存器。修复让寄存器只保存 DEX string index，
在 `setText` 提交点进行一次有界复制；没有增加栈、取消 guard 或扩大权限。

后两轮暴露了 kernel tracer 与主监控器中两份旧资源字符串的 24/26 字节断言。
修复后两层都精确验证 ABI62 的 21/27 字节内容与 48 字节合计，最终门禁通过。

## 仍未完成

ABI62 只是更接近常见 Android UI 源码的一步。当前仍不支持一般 Java
`String`/`CharSequence` 对象模型、字符串拼接、Unicode MUTF-8、对象分配、GC、
任意方法图、线程、异常、reflection、ART/Dalvik、ActivityThread、Binder、
Bionic、JNI、native library、完整 Android Framework/system services、通用
PackageManager/权限语义、任意 APK 兼容或实体手机驱动。

因此不能宣称“普遍兼容 Android App”或“真实手机系统完成”。若后续进入
AOSP/ART 或容器化 Android 路线，任何外部源码或大型组件下载仍需用户另行明确
授权。
