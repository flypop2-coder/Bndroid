# AndroidBox Activity UI-6（真实应用表面）

## 结论

Activity UI-6 不新增 syscall、wire 或兼容性声明，而是修复 ABI 46–56 已安装
Android Activity 的用户界面信息架构。前台 Activity 现在只显示：

- 受信任的系统状态栏、底部导航与返回操作；
- 应用标题和由已验证 APK 身份绑定的图标；
- APK 发布的有界 `TextView` / `Button` scene。

包名、版本、APK 大小、generation、签名、digest、Activity descriptor、兼容
profile 和能力边界仍完整保留在 `Settings → Apps`。它们不再以
`Publisher scene`、`Fresh launch proof`、`Scene revision`、`Scene-RPC-*` 或
`Compatibility mode` 开发诊断覆盖应用正文。

本轮只修改本项目 UI 与离线门禁，没有下载 AOSP、没有访问网络、没有修改 Android
SDK，也没有操作已有 Mac 预览虚拟机。

## 不变量

- Publisher View 的固定几何和 Button hit target 不变，旧 APK callback 坐标不变。
- Scene revision 仍用于严格状态排序和 replay 防护，但 revision-only 更新不产生
  用户可见像素。
- 改变 package、Activity、version、APK length 或 signer 会改变 Settings/Apps，
  但在标题、图标和 publisher scene 均不变时不会改变 Activity 像素。
- Activity 图标仍绑定目录 revision、selector、package generation 与 APK digest；
  AndroidApp worker 仍不能读取 syscall 67。
- 顶部/底部系统 chrome 继续由 SurfaceServer 独占，App 只能提交中间 content
  viewport。

## 自动验证

UI host tests：

```text
bndr-ui androidbox-icon-resources5: 263/263
fmt: passed
Clippy -D warnings:
  androidbox-apk-install0: passed
  androidbox-interactive0: passed
  androidbox-scene-rpc2: passed
  androidbox-icon-resources5: passed
AArch64 bndroid-init androidbox-icon-resources5: passed
```

ABI 56 的真实 PNG 图标、双包、三启动门禁：

```text
terminal=ANDROIDBOX_ICON_RESOURCES5_QEMU_OK
evidence=target/androidbox-icon-resources5/coexist.DfLzbv
physical_screen=720x1600
package0=org.bndroid.envelope
package1=org.bndroid.catalog
apk_launcher_icon_pixels=1
apk_activity_header_icon_pixels=1
distinct_activity_headers=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
aosp_downloaded=0
catalog_activity_sha256=563d52a68c16a190c04f22701549eec38b145f919d23863cd587de256b922b67
envelope_activity_sha256=b52588445dda1752e6059010c5ec31a6681a98c0a6658556988a3205f57f199e
```

ABI 55 的回退图标、双包、三启动父门：

```text
terminal=ANDROIDBOX_MULTIPACKAGE4_QEMU_OK
evidence=target/androidbox-multipackage4/coexist.UgqHpr
distinct_activity_headers=1
source_free_recovery=1
source_free_writes=0
qemu_starts=3
qemu_network=disabled
catalog_activity_sha256=77193de7f585151cb56dee9ab40c45e89d58e54b07aad465e06aa859ed434114
envelope_activity_sha256=06dbfe22051c268b04126fa8554b3f39d060d22425dadecf3b8f742a98b19bd6
```

ABI 50 的双 Button 与隔离 AndroidApp worker 父门：

```text
terminal=ANDROIDBOX_MULTIACTION3_QEMU_OK
evidence=target/androidbox-multiaction3/check.gHhuen
approve_status_changed_pixels=2951
reject_status_changed_pixels=3006
approve_outside_status_publisher_changed_pixels=0
reject_outside_status_publisher_changed_pixels=0
scene_revision_pixels_hidden=1
relaunch_initial_raster_identical=1
recovery_disk_unchanged=1
qemu_network=disabled
```

## 当前边界

这是 UI 真实性与信息隔离改进，不是新的 Android API 兼容档位。当前仍只执行严格
接纳的 Resources-1 / Scene-RPC 子集，不包含 ART/Dalvik、Binder、Bionic、JNI、
Android Framework/system services、通用 intent/permission、任意 APK、Play
services 或实体设备驱动。下一阶段若引入 AOSP/ART，仍需在明确版本、许可证、下载
体积、构建时间与磁盘成本后获得授权。
