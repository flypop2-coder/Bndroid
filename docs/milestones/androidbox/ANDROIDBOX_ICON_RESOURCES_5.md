# AndroidBox Icon Resources-5（ABI 56）

## 结论

opt-in profile `androidbox-icon-resources5` 是 ABI 55
`androidbox-multipackage4` 的 child。它第一次把真实 SDK APK 的
`<application android:icon>` 引用解析、验证、持久身份绑定并绘制到 Bndroid 的
Launcher 与 Settings。两个测试包分别显示蓝色 `E` 和紫色 `C`，不再依赖
package-derived fallback monogram。

本里程碑只使用 Mac 已安装的 Android SDK/JDK，未下载 AOSP、未访问网络、未修改
Android SDK。它仍是严格有界的 PNG/Resources/Activity 子集，不是任意 Android App
兼容层或实体手机系统。

## APK 与资源边界

ABI 56 fixture 必须在 Manifest 的 application 节点携带一个非零
`android:icon` reference。资源表只允许把它解析到 default configuration 中唯一的：

- `drawable` 或 `mipmap` 类型；
- 精确 `res/drawable/<name>.png` 或 `res/mipmap/<name>.png` 路径；
- APK 中唯一、STORED、CRC 正确且不超过 16 KiB 的 ZIP entry。

PNG decoder 不分配内存，只接受：

```text
dimensions=16x16
color=RGBA8
interlace=none
chunks=IHDR + exactly-one-IDAT + IEND
filters=0/1/2/3/4
pixels=canonical 0xAARRGGBB
transparent_rgb=0
```

整文件 ZIP CRC、PNG chunk CRC、zlib 输入完整消费、解压后精确尺寸、chunk 顺序、
未知 chunk、重复 IDAT、尾随字节和非法 filter 都 fail closed。缺少 icon 时可返回
canonical absence 并由 UI 使用诚实 fallback；声明了 icon 但解析失败时不会静默
伪造成功。

历史 ABI55 fixture 保持逐字节不变。只有 ABI56 wrapper 显式导出
`BNDROID_ANDROID_ICON_RESOURCES=1`，构建脚本才选择 `AndroidManifest.icon.xml`
并加入 PNG；这避免新增 drawable 类型改变父档位的 AAPT2 resource ID。

## ABI 56 与权限

syscall 67 `AndroidPackageIconRead` 返回固定 1,152-byte `BNDAIC01`：

```text
directory revision
zero-based selector
package generation
APK SHA-256
presence + exact 16x16 dimensions
resource ID + PNG CRC32
256 canonical ARGB pixels
reserved bytes = zero
```

结果同时绑定目录 revision、selector、generation 和 APK digest，不能把一个包的
图标移植到另一个目录条目。只允许内建 Launcher 与 App/Settings 调用；隔离
AndroidApp worker 的 syscall 67 被明确拒绝，日志证明
`authority_unchanged=1`。返回值不携带 APK bytes、路径、VMO、storage handle、
block handle 或写权限。

kernel 使用原有 1,344-byte 目录 wire scratch 作为带重入保护的临时输出，避免在小
内核栈上构造 1,152-byte wire。持久包格式保持 ABI55 不变；每次安装恢复都从已签名、
digest 匹配的 durable APK 重新导出图标，不在未认证旁路缓存像素。

## 栈预算与 UI

完整 ABI wire 可以表达 256 个不同颜色，但当前内核目录 snapshot 与 EL0 UI 模型只
接纳最多 16 色。它们保留 16-entry ARGB palette 和 128-byte 四位索引，将每个图标
从 1,024-byte raw pixels 压缩到有界小对象。超过 16 色会 fail closed；不是静默
量化。

Launcher drawer、Settings package selector/card 和已安装 Activity header 复用同一
renderer。16×16 源像素按 3×3 design pixels 绘制为 48×48 design-pixel 图标，在
720×1600、20:9 物理 surface 上进行 alpha blending。无图标包继续使用由 immutable
APK digest 与 title 派生的 fallback。

后续 Activity UI-6 已把同一已验证图标移到正常 app bar，并从前台 Activity 移除
package/RPC/revision 开发诊断。版本、包名、签名、digest 与 Activity descriptor
仍保留在 Settings/Apps；内部 scene revision 不再成为发布者界面的可见像素。该
UI 收口没有改变 syscall、图标读取权限或 APK 接纳范围。

## 真实 APK 与门禁

两个 ABI56 APK 均由本机 SDK 36.1.0 的 AAPT2/D8/zipalign/apksigner 离线构建并只
启用 APK Signature Scheme v2：

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12717
apk0_sha256=2c2b8fa5e318babd10a0b35114ea7843eb7bdd5cc916e8629784468c850b1a33
icon0_resource_id=0x7f010000
icon0_png_crc32=f63d0b72

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12640
apk1_sha256=2410127c7a3b6a9e041d8359b227a4e3968b66bf845bf716f955bca15b2ce21f
icon1_resource_id=0x7f010000
icon1_png_crc32=9830e3a8
```

三启动、全程 `-nic none` 的 720×1600 QEMU 门禁已经通过：

```text
terminal=ANDROIDBOX_ICON_RESOURCES5_QEMU_OK
evidence=target/androidbox-icon-resources5/coexist.DfLzbv
abi=56
capacity=2
two_distinct_packages=1
two_launcher_entries=1
apk_launcher_icon_pixels=1
apk_activity_header_icon_pixels=1
icon_read_authority=launcher+settings-only
settings_two_package_selector=1
settings_selection_storage_mutation=0
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

`two-app-drawer.ppm` 与 `recovery-two-app-drawer.ppm` 的 SHA-256 都是
`77e521f5714860a524a59b05423dc4b6eaec412572e4726b7e89ce5aa66b3aa1`，
证明无 source 冷启动后抽屉逐像素恢复。门禁还验证蓝色图标物理像素
`RGB(11,87,208)`、紫色图标物理像素 `RGB(142,36,170)`，以及两个不同 Settings
selector 和两个不同 Activity raster。Catalog/Envelope Activity raster 分别为
`563d52a68c16a190c04f22701549eec38b145f919d23863cd587de256b922b67` 与
`b52588445dda1752e6059010c5ec31a6681a98c0a6658556988a3205f57f199e`；无 source
恢复结果逐字节相同。门禁单独比较 publisher scene 以上的 Activity header，并在
物理点 `(640,84)` 验证 Catalog 紫色 `C` 与 Envelope 蓝色 `E`，因此正文不同但
标题、package 和 icon 错绑到同一个包也不能通过。

Host tests 当前通过：

```text
bndr-androidbox androidbox-icon-resources5: 105/105
bndr-abi androidbox-icon-resources5: 58/58
bndr-ui androidbox-icon-resources5: 263/263
historical envelope APK: byte-identical
historical catalog APK: byte-identical
ABI56 icon APKs: byte-identical to QEMU evidence inputs
fmt: passed
Clippy all targets -D warnings: passed
AArch64 release kernel/userspace check: passed
parent ABI55 QEMU: target/androidbox-multipackage4/coexist.UgqHpr
```

## QEMU 发现并修复的问题

- 最初在 package directory snapshot 中保存 raw 1,024-byte icon，触发内核嵌套异常
  栈余量门禁；改为 16 色 palette + 四位索引。
- raw UI icon 扩大了 App 的 `MobileModel` 栈帧并触发用户栈 guard；UI 同样改为有界
  palette 形式。
- ABI55 relaunch snapshot 不携带 ABI56 icon，最初与完整期望状态比较失败；现在先
  独立验证 syscall 67 的目录/代次/digest 绑定，再只附加已验证 icon。
- 新 drawable 类型把 layout `id/*` 从 `0x7f01....` 移到 `0x7f02....`，旧的重启
  证据硬编码因此误判。ABI56 现在严格使用新类型基址，ABI55 继续严格使用旧基址；
  没有放宽 RPC 或受控故障验证。
- 为防止上述 ID 改变渗入父档位，icon Manifest 和 drawable 构建改为 ABI56 显式
  opt-in，旧 APK 已重新证明逐字节不变。
- Launcher 与 App 进程各有独立的 `MobileModel`；两个包又都可能处于 package-local
  generation 1。最初 Catalog 的 publisher scene 已正确执行，但 App 仍用启动时
  默认选择的 Envelope 绘制 header。App 现在在兼容 Activity 获得焦点时先读取
  syscall 59 的 boot-local active package，再以完整 package/activity/version/
  length/signer/APK digest 身份精确匹配 syscall 66 目录条目，并只从该匹配条目
  附加 syscall 67 icon。ABI55/56 三启动门禁都新增独立 header raster 断言。

## 明确不支持

ABI 56 当前仍不提供：

- adaptive icon XML、vector drawable、WebP、JPEG、density/locale/night
  configuration 选择或资源 overlay；
- 超过 16 色的 retained icon、动态主题 icon、安装后 icon 更新服务；
- ART/Dalvik、Bionic、Binder、JNI、native `.so` 或完整 Android Framework API；
- Service、Receiver、Provider 执行、runtime permission grant 或通用 intent；
- Play services、Play Store、网络安装、任意 APK、CTS/VTS；
- 实体设备 GPU、音频、相机、电话、蜂窝、传感器、休眠和电源驱动。

准确表述是“两个真实 SDK APK 的签名安装、持久恢复、有界 Activity 执行与真实 PNG
launcher icon”，不能表述为“已兼容 Android App”或“已成为真实可用手机系统”。
AOSP/ART 级引入仍必须作为独立阶段，在明确版本、体积、许可证、构建和磁盘成本后
取得授权。
