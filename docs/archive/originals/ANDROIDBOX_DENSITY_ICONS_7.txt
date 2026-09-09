# AndroidBox Density Icons-7（ABI 57）

## 结论

opt-in profile `androidbox-density-icons7` 是 ABI 56
`androidbox-icon-resources5` 的 child。它把 Android SDK 常见的多密度图标资源接入
现有安装、持久恢复、Launcher、Settings 和 Activity 路径：

- 从同一已验证 APK 的 `resources.arsc` 中优先选择唯一、精确的 mdpi 160 dpi
  `drawable` / `mipmap` PNG；
- 没有 mdpi 时回退到 ABI56 已支持的唯一 default 16×16 PNG；
- 把精确 48×48 mdpi RGBA8 以 premultiplied-alpha-aware 3×3 box filter
  归一化为现有 16×16 canonical 系统图标；
- 超过 16 色时在 decoder 边界做确定性、有来源标记的量化，继续满足 kernel 与 EL0
  的固定容量 palette 预算；
- 通过 ABI57 `BNDAIC01` v2 明确传递源尺寸、源 density、归一化和量化 provenance。

本里程碑只使用 Mac 已安装的 Android SDK/JDK 和项目已有代码。没有联网、没有下载
AOSP、没有修改 Android SDK，也没有操作用户正在运行的 ABI48 Mac 预览虚拟机。

## 资源选择与失败边界

Manifest 的 `<application android:icon>` 仍必须是非零资源引用。ABI57 扫描目标
resource ID 的同类型配置，并执行一条封闭选择规则：

1. 如果存在唯一的精确 mdpi 配置，选择它；
2. 否则选择唯一 default 配置；
3. xhdpi 等其他密度配置可以存在，但不会被误选；
4. 目标配置重复、类型错误、复杂 entry、路径不规范或缺失都 fail closed。

精确 mdpi 配置只允许 density `160`，SDK version 只允许 AAPT2 会产生的 `0` 或
`4`，其余 qualifier 字节必须为零。选择后的路径只能是：

```text
res/drawable-mdpi[-v4]/<name>.png
res/mipmap-mdpi[-v4]/<name>.png
```

default 回退继续只允许 `res/drawable/<name>.png` 或
`res/mipmap/<name>.png`。ZIP entry 必须唯一、STORED、CRC 正确且不超过原有
16 KiB 上限。

PNG decoder 仍不分配内存，只接受 RGBA8、无交错、`IHDR + exactly-one-IDAT +
IEND`、完整 CRC 与 zlib 消费。ABI57 新增的源形状只有：

```text
default: 16x16, density=0
mdpi:    48x48, density=160
```

48×48 的每个 3×3 源块按 alpha 加权平均为一个 16×16 像素；全透明像素强制
`RGB=0`。如果归一化结果超过 16 个颜色，decoder 按颜色出现频率和固定 ARGB
tie-break 选择最多 16 色，再以 premultiplied RGBA 距离映射。这个过程确定、
无分配、保留透明色，并通过 `color_quantized` 明确报告；fixture 图标本身不需要
量化，所以门禁日志为 `color_quantized=0`。

本档位没有实现 nearest-density 搜索、只有 xhdpi 时的缩放、任意尺寸、adaptive
icon XML、VectorDrawable、WebP、JPEG、overlay、theme 或 runtime configuration
切换。遇到这些形状不会伪造支持。

## ABI 57 与权限

syscall 67 仍返回固定 1,152-byte `BNDAIC01`，因此没有扩大 syscall buffer 或栈
对象。ABI57 把 wire version 提升为 2，并使用原 ABI56 reserved 区域：

```text
1104..1106  source_width: u16
1106..1108  source_height: u16
1108..1110  source_density_dpi: u16
1110..1152  reserved = zero
```

新增 flags：

```text
bit 0  PRESENT
bit 1  DENSITY_NORMALIZED
bit 2  COLOR_QUANTIZED
```

v2 只接受 `(16,16,0)` default 或 `(48,48,160)` mdpi provenance，presence、flags、
源字段和像素必须互相一致；absence 时全部 icon payload 与 provenance 必须为零。
目录 revision、selector、package generation、APK SHA-256、resource ID 和 PNG
CRC32 的身份绑定不变。

权限也没有扩大：只有内建 Launcher 与 App/Settings 能读取图标；隔离 AndroidApp
worker 仍被拒绝。返回值不包含 APK bytes、路径、VMO、storage handle、block
handle 或任何写权限。

## 真实 Android SDK APK

两个 fixture 的 ABI57 形状都由本机 AAPT2 编译出同一个 drawable resource ID 的
mdpi 与 xhdpi 配置：

```text
res/drawable-mdpi-v4/app_icon.png
res/drawable-xhdpi-v4/app_icon.png
```

xhdpi 条目有意存在，用来证明选择器实际按 160 dpi 选中 mdpi，而不是依赖 ZIP
顺序或“第一个资源”。真实门禁输入为：

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12805
apk0_sha256=ad7260288e06f12fd2216bfa203c719114eff086b0645ff3cb738cd1d1f05c65
mdpi_png_crc32=fc4d4dc6

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12728
apk1_sha256=a85f3c6fb6741a9223fb82b72ada120a434dd881a6c4098d3c066bc63421d621
mdpi_png_crc32=22b3a9c3
```

## 自动验证

Host 与静态检查：

```text
bndr-abi=59/59
bndr-androidbox=107/107
bndr-ui=263/263
cargo fmt --all -- --check=passed
shell syntax=passed
AArch64 Clippy -D warnings:
  bndr-abi=passed
  bndr-androidbox=passed
  bndr-ui=passed
  bndroid-kernel=passed
  bndroid-init=passed
AArch64 release QEMU kernel/userspace build=passed
```

ABI57 三启动、全程 `-nic none`、720×1600 QEMU 门禁：

```text
terminal=ANDROIDBOX_DENSITY_ICONS7_QEMU_OK
evidence=target/androidbox-density-icons7/coexist.Zk3rxj
abi=57
density_icon_reads=24
selected_source_density_dpi=160
selected_source_dimensions=48x48
apk_launcher_icon_pixels=1
apk_activity_header_icon_pixels=1
icon_read_authority=launcher+settings-only
two_distinct_packages=1
settings_two_package_selector=1
two_activity_launches_same_boot=1
distinct_activity_headers=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
aosp_downloaded=0
storage_handle_granted=0
block_handle_granted=0
```

恢复后的 drawer 和两个 Activity 与安装后的对应 raster 逐字节相同：

```text
two-app-drawer_sha256=77e521f5714860a524a59b05423dc4b6eaec412572e4726b7e89ce5aa66b3aa1
catalog-activity_sha256=563d52a68c16a190c04f22701549eec38b145f919d23863cd587de256b922b67
envelope-activity_sha256=b52588445dda1752e6059010c5ec31a6681a98c0a6658556988a3205f57f199e
```

ABI56 父版本也以当前源码重新完成三启动门禁：

```text
terminal=ANDROIDBOX_ICON_RESOURCES5_QEMU_OK
evidence=target/androidbox-icon-resources5/coexist.DfLzbv
abi=56
density_icon_reads=0
selected_source_density_dpi=0
selected_source_dimensions=16x16
source_free_recovery=1
source_free_writes=0
qemu_network=disabled
```

两个 `BNDROID_ANDROID_ICON_RESOURCES=0` 历史 fixture 也重新离线构建并与 checked-in
APK 逐字节相同：

```text
envelope_sha256=a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5
catalog_sha256=5be29f7bd173becb7a8081a672639e6efccebe1d4636e82234cf97f1d237a859
```

## 仍未完成

这一步改善了真实 APK 的资源兼容面，但没有把 Bndroid 变成通用 Android runtime。
当前仍不含 ART/Dalvik、Binder、Bionic、JNI、完整 Android Framework 与 system
services、通用 PackageManager、权限实现、native library、任意 APK 兼容或实体
手机驱动。进入 AOSP/ART 级路线前，仍需明确 Android 基线、体积、许可、构建成本
和集成方式，并取得用户授权后才能下载外部源码或预编译组件。
