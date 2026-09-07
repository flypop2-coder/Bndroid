# AndroidBox MultiPackage-4（ABI 55）

## 结论

opt-in profile `androidbox-multipackage4` 是 ABI 54
`androidbox-manifest-catalog3` 的 child。它把此前唯一的持久 Android 包扩展为
**固定容量 2 个包**，并已在 720×1600 QEMU 手机界面中证明两个由真实 Android SDK
构建、APK-v2 签名且 package name 不同的 APK 可以：

1. 依次安装到同一块持久磁盘；
2. 同时显示为两个 Launcher 图标；
3. 在同一次启动中先后打开各自不同的 Activity；
4. 在不再提供 `fw_cfg` APK source 的下一次启动中恢复并再次打开；
5. 恢复启动期间保持磁盘零写入、最终磁盘逐字节不变。

本里程碑没有下载 AOSP、没有发起网络请求，也没有修改 Android SDK。它不是通用
Android PackageManager、任意 APK 兼容层或真实手机系统。

## 固定双包存储

`bndr-package-store` 的 `multi-package-store1` 把两个原有 crash-safe 单包卷按稳定
volume order 组合。每个包仍有独立的：

- generation、版本、package/activity identity、APK digest 和 signer digest；
- double registry、double APK blob 与 mirrored tombstone；
- 写入、flush、完整 readback 和重新接纳流程。

目录以 package name 查找现有卷，以第一个空卷接纳不同的新包；容量已满、重复身份、
不一致目录或无效卷状态都会 fail closed。没有动态扩容、目录树、通用文件系统安装
或 managed app data。

## ABI 55 目录

syscall 66 `AndroidPackageDirectoryRead` 返回固定 1,344-byte `BNDAPD01` wire：

```text
header=64 bytes
capacity=2
entry=640-byte authority-free AndroidPackageSnapshot
ordering=stable package-store volume order
```

每个目录条目只暴露包目录和执行所需的只读元数据，不携带 APK 地址、任意路径、
source authority、storage handle、block handle 或 I/O counters。未使用条目必须全
零；count、revision、formatted 状态、重复 package 和嵌套 snapshot 都按 canonical
规则验证。

kernel syscall 实现一次只编码一个 snapshot，并使用 IRQ-masked、带重入保护的有界
静态 1,344-byte scratch，避免把完整目录和多个大 snapshot 同时放到内核栈。

## Launcher 与 Settings

Launcher 按稳定的一基 selector 为两个已安装包显示两个独立图标。选中图标后，
App 进程只请求对应目录条目的包；它不会得到包存储或安装权限。

当前 snapshot 尚不携带已解码 Android icon resource。为避免两个包继续显示同一个
开发者机器人和 `R1` profile 徽标，Launcher、Settings 和 Activity header 使用
由 immutable APK digest 选择颜色、由 title 选择 monogram 的稳定 fallback icon。
两个包因此可区分，但该 fallback 不会被表述为 APK 自带图标。

两个包可能都处于各自卷的 generation 1，因此 recent identity 不能只靠 generation
判断为同一个 App。切换到另一个包时，App 进程会先用精确旧 identity 提交经过鉴权的
`FinishCompatibleActivity`，确认 recent 清空且导航回到 Home 后，再为目标包 reserve
新的 session。kernel 只更新 boot-local 的当前选择兼容视图，不改变持久目录、revision
或磁盘。

Settings 的 Apps 页显示完整双包目录和独立选中状态。存在不同 install candidate
时，小操作按钮会按真实候选关系显示 `Install`、`Update` 或 `Reinstall`；安装仍需
两次明确点击，第一次不会修改磁盘。没有候选或进行中的事务时，两个 package selector
可以直接切换下方版本、大小、generation、digest 和 Activity 详情；这是 App-local
只读选择，不改变目录 revision、包存储或磁盘。

## 真实 APK 门禁

本机已有 Android SDK/JDK 离线构建了两个不同包：

```text
package0=org.bndroid.envelope
activity0=Lorg/bndroid/envelope/MainActivity;
apk0_bytes=12646
apk0_sha256=a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5

package1=org.bndroid.catalog
activity1=Lorg/bndroid/catalog/MainActivity;
apk1_bytes=12569
apk1_sha256=e1dfc35dcdd7c27ee0541be1a6024f8e21c046a9e327882446d1913760fed78a
```

严格离线三启动门禁 `scripts/check-androidbox-multipackage4.sh` 已通过：

```text
terminal=ANDROIDBOX_MULTIPACKAGE4_QEMU_OK
evidence=target/androidbox-multipackage4/coexist.UgqHpr
abi=55
capacity=2
two_distinct_packages=1
two_launcher_entries=1
settings_two_package_selector=1
settings_selection_storage_mutation=0
two_activity_launches_same_boot=1
distinct_activity_headers=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
aosp_downloaded=0
initial_disk_sha256=c82bffe86b4fea83e5483723dc659b43f11bb9f320c6b2c7442d00e5026cdadc
one_package_disk_sha256=46fe50eba587626ae24277c7c1ff8967199b9060300629ec8025e1004aeb5f1d
two_package_disk_sha256=6bde48a012dff177bcf701e61457ffd271b47914e7a04fb427642bde2d7d807b
final_disk_sha256=6bde48a012dff177bcf701e61457ffd271b47914e7a04fb427642bde2d7d807b
```

`two-app-drawer.ppm`、两个 Settings package selector、两个不同 Activity 和
source-free recovery 的对应 raster 均为 canonical 720×1600 P6 证据。两个
Settings 选择结果和两个 Activity 都不是同一张图；恢复帧与安装启动的对应抽屉和
Activity 帧一致。

ABI 55 还通过了原单包完整安装/更新门禁，证明双包改造没有破坏原有路径：

```text
terminal=ANDROIDBOX_MULTIPACKAGE4_SINGLE_REGRESSION_QEMU_OK
evidence=target/androidbox-multipackage4/check.Qq8u5l
confirmation=two-step
first_tap_mutation=0
install_generation=1
update_generation=2
source_free_recovery=1
source_free_writes=0
qemu_starts=4
qemu_network=disabled
```

Host tests 已通过：

```text
bndr-package-store multi-package-store1: 47/47
bndr-abi androidbox-multipackage4: 56/56
bndr-ui androidbox-multipackage4: 260/260
fmt: passed
AArch64 release kernel/userspace check: passed
```

UI tests 覆盖两个唯一包的事务性目录接纳、重复/空洞/非 canonical count 拒绝、稳定
Launcher selector、Settings 零基详情 selector、fallback icon 身份以及安装操作文案。

## 本轮发现并修复的问题

- syscall 66 最初在栈上同时构造完整目录和多个大 snapshot，QEMU 捕获了内核栈失败；
  现改为静态有界 wire scratch 和逐 entry 编码。
- 未格式化空目录最初错误要求非零 revision；现要求 `count=0 && revision=0`，已格式化
  目录才要求非零 revision。
- 一个已安装包加一个不同候选时，Settings 小按钮曾硬编码为 `Update`；现从候选 identity
  推导 `Install / Update / Reinstall`。
- 两个不同卷都可能是 generation 1，旧 recent 曾被误当作目标包复用；现先精确关闭旧
  recent，再创建目标包的新 session。
- 两个已安装包曾在 Launcher 中显示相同机器人图标和内部 `R1` 徽标，Settings 也只能
  显示一个当前包；现改为 package-derived fallback icon 和可交互的双包详情 selector。

## AOSP 源码决策

ABI 55 不需要 AOSP 源码，因为它实现和验证的是 Bndroid 自己的有界包目录、存储、
Launcher/Settings 流程以及 Resources-1/DEX 子集执行器。

若目标升级为广泛运行现有 Android App，则通常需要选定 Android API/AOSP 基线并引入
或适配至少一部分 ART、Bionic、Binder、Framework、Resources、SELinux policy 和 HAL
接口。源码与构建产物体积、许可证义务、构建时间、工具链和磁盘成本都显著增加。
因此任何 AOSP 下载或大规模第三方代码引入都必须作为独立阶段，在明确范围、成本与
来源后取得授权，不能由当前里程碑自动推导。

## 明确不支持

ABI 55 当前仍不提供：

- ART/Dalvik、Bionic、Binder、ServiceManager、JNI 或 native `.so`；
- 完整 Android Framework API、system services、动态 intent resolution；
- Service、Receiver、Provider 执行或 runtime permission grant；
- 通用安装来源、网络下载、Play services、Android CTS/VTS；
- managed app data、多用户、Android SELinux policy；
- 实体设备 GPU、音频、相机、电话、蜂窝、传感器、休眠与电源管理驱动；
- 超过两个已安装包。

准确表述是“两个真实 SDK APK 的持久共存、Launcher 选择和有界 Activity 执行”，
不能表述为“已兼容 Android App”或“已成为真实可用手机系统”。
