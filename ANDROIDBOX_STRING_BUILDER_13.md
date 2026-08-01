# AndroidBox String Builder-13（ABI 63）

## 结论

opt-in profile `androidbox-string-builder13` 是 ABI62
`androidbox-string-text12` 的 child。真实 Android SDK/D8 APK 现在可以执行常见
的动态计数文本：

```java
private TextView statusView;
private int clickCount;

public void onClick(View view) {
    int next = clickCount + 1;
    clickCount = next;
    statusView.setText("Review count: " + next);
}
```

本机 Android 36 SDK 的 D8 9.0.3-dev 生成：

```text
iget           v4, v3, MainActivity.clickCount:I
add-int/lit8   v4, v4, #int 1
iput           v4, v3, MainActivity.clickCount:I
iget-object    v0, v3, MainActivity.statusView:TextView
new-instance   v1, Ljava/lang/StringBuilder;
const-string   v2, "Review count: "
invoke-direct  {v1, v2}, StringBuilder.<init>:(String)V
invoke-virtual {v1, v4}, StringBuilder.append:(I)StringBuilder
invoke-virtual {v1}, StringBuilder.toString:()String
move-result-object v4
invoke-virtual {v0, v4}, TextView.setText:(CharSequence)V
return-void
```

解释器执行这一真实指令链，第一次和第二次点击分别产生
`Review count: 1`、`Review count: 2`，同时提交 retained int state `1/2`。
结果不是资源字符串、预设分支或根据 revision 推断。

本轮只使用 Mac 已安装的 Android SDK/JDK 与仓库源码，全程离线；没有下载 AOSP、
没有修改 Android SDK，也没有操作持续运行的 ABI48 Mac 预览虚拟机。

## 有界对象语义

- 只接受精确的 `Ljava/lang/StringBuilder;`；
- 每个 callback 只允许一次 `new-instance`；
- 构造器必须精确为 `<init>(String)`，参数来自当前 `classes.dex` 的一个非空、
  有界 ASCII `const-string`；
- 只允许一次 `append(int)`，整数必须等于本 callback 刚从 Activity 字段读出、
  加一并写回的状态；
- 只允许一次 `toString()`，返回值必须紧邻 `move-result-object`；
- 最终对象只能进入精确的 `TextView.setText(CharSequence)`；
- DEX 寄存器保存小型符号句柄和 string index，不保存 Java 对象地址，也不把
  256 字节字符串复制到每个寄存器；
- 最终文本在提交点进行一次有界十进制拼接；溢出、错误 owner/prototype、
  重复构造、错误顺序或其他对象操作全部 fail closed；
- scene、Activity 字段和 revision 继续事务提交，失败不产生部分 UI 更新。

这是一个单 builder 的有界对象子集，不是通用 Java heap、对象分配器或 GC。
ABI62 固定 DEX 字面量路径及更早的资源、helper、字段路径仍由 child 接纳。

## BNDAPC09

系统 ABI 提升为 63，固定大小进程协议升级为 `BNDAPC09` v9：

```text
arg0[31:0]   changed TextView ID
arg0[39:32]  total APK-defined calls
arg0[47:40]  instance APK-defined calls
arg0[53:48]  Activity field reads
arg0[54]     dynamic StringBuilder text
arg0[55]     direct DEX string text
arg0[63:56]  committed int Activity state
```

动态 callback 的 canonical 形状为：

```text
total=0 instance=0 field_reads=2 dynamic=1 direct=1 int_state=1..255
```

固定 ABI62 字面量在 child 中仍为 `dynamic=0 direct=1`。ABI decoder、
AndroidApp worker、App client 与 kernel tracer 分别解析和校验 bit 6，不能把
动态结果伪装为固定字符串。

## 真实 APK 与自动门禁

Envelope 和 Catalog fixture 新增独立 `src-concat` 源目录。
`BNDROID_ANDROID_DEX_METHODS=6` 选择 ABI63 形状：

```text
package0=org.bndroid.envelope
apk0_bytes=12805
apk0_sha256=9a891a3d188e4225803519713c29d02b56eaeebacfafcbba36656193964e2b53

package1=org.bndroid.catalog
apk1_bytes=12728
apk1_sha256=41d8c0639080b4b9f55a554811558d35ce4f16745543762758dbd926ae0f9118
```

```sh
./scripts/check-androidbox-string-builder13.sh
```

最终源码对应的三启动、720×1600、全程 `-nic none` QEMU 证据：

```text
terminal=ANDROIDBOX_STRING_BUILDER13_QEMU_OK
evidence=target/string-builder13/coexist.xDu7nN
abi=63
rpc_protocol=BNDAPC09
rpc_protocol_version=9
app_defined_calls_per_transcript=0/0
app_defined_instance_calls_per_transcript=0/0
activity_field_reads_per_transcript=2/2
activity_int_state_values_per_transcript=1/2
direct_string_texts_per_transcript=true/true
dynamic_string_texts_per_transcript=true/true
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
live_approve_changed_pixels=2511
live_reject_changed_pixels=1429
recovery_approve_changed_pixels=2511
recovery_reject_changed_pixels=1429
```

最终检查：

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
release kernel/nine-userspace build=passed (QEMU gate build)
```

ABI62 父门以当前共享实现重新通过：

```text
terminal=ANDROIDBOX_STRING_TEXT12_QEMU_OK
evidence=target/string-text12/coexist.OLePzg
```

## 仍未完成

ABI63 仍不支持一般 `StringBuilder` 链、多个对象、任意构造器、字符串或对象字段、
Unicode MUTF-8、数组、异常、循环、线程、reflection、通用 heap/GC、ART/Dalvik、
ActivityThread、Binder、Bionic、JNI、native library、完整 Android
Framework/system services、通用 PackageManager/权限语义、任意 APK 兼容或实体
手机驱动。

因此不能宣称“普遍兼容 Android App”或“真实手机系统完成”。若后续需要下载
AOSP 或其他大型外部组件，仍须用户另行明确授权。
