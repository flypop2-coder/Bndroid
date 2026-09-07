# AndroidBox Manifest Catalog-3（ABI 54）

## 结论

opt-in profile `androidbox-manifest-catalog3` 是 ABI 53
`androidbox-runtime-install2` 的 child。它把此前只接受一个严格 Launcher
Activity 的二进制 Manifest 解析器扩展为固定容量的组件与权限目录，同时保持 ABI 53
及更早 profile 的解析和运行语义不变。

本里程碑已经用本机现有 Android SDK/JDK 离线构建并运行一个真实、多组件、APK-v2
签名的应用：

```text
package=org.bndroid.catalog
primary_activity=Lorg/bndroid/catalog/MainActivity;
components=6
launchers=2
requested_permissions=2
intent_filters=4
canonical_version=3
canonical_apk_bytes=12569
canonical_apk_sha256=5be29f7bd173becb7a8081a672639e6efccebe1d4636e82234cf97f1d237a859
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
```

构建没有下载 AOSP，没有网络请求，也没有修改 Android SDK。fixture 位于
`fixtures/androidbox-manifest-catalog-demo/`。

## Manifest 目录

新解析器接受 Android SDK `aapt2` 生成的 binary XML，并识别以下有界元素：

- `manifest`、`uses-sdk`、`uses-permission`、`uses-feature`、`application`；
- `activity`、`activity-alias`、`service`、`receiver`、`provider`；
- `intent-filter`、`action`、`category`、`data`、`meta-data`。

目录最多保留 16 个组件和 16 个去重后的 requested permissions；权限名最多
192 bytes，string pool、属性数和 XML 深度也分别受固定上限约束。超过容量、重复或
矛盾的身份、无 target 的 alias、无 authority 的 provider、非法导出状态、坏资源
映射及非 canonical binary XML 都会在发布候选或写盘前 fail closed。

每个组件只记录：

- kind 与标准化 DEX descriptor；
- alias target；
- explicit/default exported、enabled；
- intent-filter/action/category/data 数量；
- 是否为完整 `MAIN + LAUNCHER`；
- provider authority 和 component permission 是否声明。

这只是包元数据。解析 service、receiver、provider 或 permission 不会创建进程、
解析 intent、注册系统服务或授予权限。

## Launcher 选择与执行边界

运行时按 Manifest 声明顺序选择第一个 `enabled && exported` 的完整 launcher。
如果选中 `activity-alias`，执行目标绑定到 alias 的 `targetActivity`。其余 Activity、
alias、Service、Receiver 和 Provider 保持 inert。

只有被选中的 Activity 继续进入现有的有界 DEX/resources/scene 执行器。当前 fixture
的真实 Java `MainActivity`：

- 调用 `setContentView` 加载编译后的 5-node layout；
- 含两个 `TextView` 和两个 callback `Button`；
- `onClick(View)` 按 View ID 走两个真实分支；
- 分别把状态更新为 `Components: 6 discovered` 与
  `Permissions: declared only`。

这仍不是 ART、Dalvik 或完整 Android Framework。当前 DEX 解释器只执行已审计的
有限指令与 Framework 方法组合。

## API 与工作区

只读检查 API：

```rust
let catalog =
    bndr_androidbox::inspect_apk_manifest_catalog_with_scratch(apk, &mut envelope_scratch)?;
assert_eq!(catalog.components().len(), 6);
assert_eq!(catalog.launcher_count(), 2);
assert_eq!(catalog.permissions().len(), 2);
```

执行 API：

```rust
let signer = bndr_androidbox::verify_apk_v2(apk)?;
let image = bndr_androidbox::AndroidBox::load_with_manifest_catalog_scratch(
    apk,
    &mut envelope_scratch,
    &mut catalog_scratch,
)?;
assert_ne!(signer.certificate_sha256, [0; 32]);
let session = image.launch_activity_session()?;
```

签名验证仍是显式、独立的 package-admission 步骤；loader 不会把 ZIP 解析成功误当成
APK authentic。Manifest 目录使用调用方提供的
`AndroidBoxManifestCatalogScratch`，每次调用前后完整清零，返回值不借用工作区。
kernel package manager 和隔离 AndroidApp 各自把它放在受控静态 BSS，而不是
64-KiB EL0 栈。

## 安装、更新与恢复

ABI 54 复用 ABI 53 的两步确认、最小权限 syscall 和 crash-safe 单包 store：

1. immutable `fw_cfg` source 完成 ZIP、v2 signature、Manifest catalog、
   resources、DEX 和 Activity-session 接纳；
2. Settings 第一次点击只进入确认状态，磁盘不变；
3. 第二次点击才提交绑定完整 candidate identity 的安装或更新事务；
4. 写盘、flush、完整 readback 和重新执行成功后才发布 live catalog；
5. Launcher 在同一启动重新获得焦点后读取新 generation；
6. 后续启动不提供 APK source，仍从 durable store 零写入恢复并启动。

APK bytes 只通过一次性只读 VMO 暴露给隔离 AndroidApp。Launcher、Settings renderer
和普通应用不会取得 APK bytes、包存储 handle、block handle、任意路径或 source
authority。QEMU 始终使用 `-nic none`。

## 自动验证

Host 与静态验证：

```text
bndr-androidbox ABI 54: 102/102
bndr-abi ABI 54: 53/53
bndr-ui ABI 54: 255/255
parent bndr-androidbox Envelope-4: 99/99
fmt: passed
ABI/AndroidBox/UI Clippy -D warnings: passed
AArch64 kernel/userspace cross build: passed
```

完整四启动 QEMU 门禁已通过：

```text
terminal=ANDROIDBOX_MANIFEST_CATALOG3_QEMU_OK
evidence=target/androidbox-manifest-catalog3/check.sZFi7h
abi=54
qemu_starts=4
qemu_network=disabled
confirmation=two-step
first_tap_mutation=0
install_generation=1
update_generation=2
source_free_recovery=1
source_free_writes=0
same_boot_launcher_refresh=1
isolated_android_app_launch=1
initial_disk_sha256=b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801
installed_disk_sha256=d48a54e9b55f039b2686dfb5c8fa20e6187a18a21e9d22473d2833ced50fc827
updated_disk_sha256=885c3cbed5b1890ef9cec54dd2ddd78f1c3069dd81e364a11c6bef4d6aa1980f
final_disk_sha256=885c3cbed5b1890ef9cec54dd2ddd78f1c3069dd81e364a11c6bef4d6aa1980f
```

门禁覆盖 v1 安装、v1 无 source 恢复、同 signer v2 更新、v2 无 source 恢复。
每次 Activity 首开还会触发一次受控 AndroidApp guard-page fault，证明旧 worker
被回收、新 generation 重新领取同一只读 APK、重放 Open 并恢复 5-node scene；
最终 `ANDROID_APP_RESTART_OK errors=0`。

9 张 canonical 720×1600 raster 覆盖安装、确认、完成、Launcher/Activity、恢复和
更新。`install-activity.ppm` 与 `update-activity.ppm` 的 publisher scene 相同，
系统 status/navigation chrome 始终由 SurfaceServer 独立拥有。

## 本轮发现并修复的问题

- 第一个 fixture 只有静态 TextView，被 ABI 53 的“候选必须可交互执行”规则正确拒绝；
  fixture 改为两个真实 callback Button。
- Manifest catalog 工作区曾落在 AndroidApp 的 64-KiB EL0 栈，QEMU guard page
  捕获真实栈越界；现已移到调用方静态、调用后清零的 scratch。
- 旧 RPC 回归检查写死上一只 APK 的 `Approve/Reject` 文本；ABI 54 现在验证新
  fixture 的完整受限 transcript，ABI 53 分支保持原值。

## 明确不支持

ABI 54 当前仍不提供：

- ART/Dalvik、Bionic、JNI、native `.so`；
- Binder、ServiceManager 或 Android system services；
- 动态 intent resolution、Service/Receiver/Provider 执行；
- runtime permission grant、SELinux Android policy、multi-user；
- 通用多包 PackageManager、文件选择安装、网络下载或 Play services；
- 实体设备驱动、电话、蜂窝、相机、音频、GPU 合成或 Android CTS/VTS。

因此准确表述是“真实 SDK APK 的有界 Manifest 多组件目录和一个可执行
Launcher Activity”，不是“任意 Android App 兼容”，也不是真实可用手机系统。
进入 Binder/Framework/ART 阶段前，应先固定目标 Android API/AOSP 基线；任何 AOSP
源码下载和大规模第三方代码引入都需要单独授权。
