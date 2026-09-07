# AndroidBox Restart-0

## 状态

`androidbox-restart0` 是 `androidbox-process0` 的 opt-in ABI 48 child。
它已经通过独立的双启动离线 QEMU 门禁，但能力边界仍然是固定的
InteractiveActivity-1 子集，不是通用 Android 兼容层。

本 profile 不修改 ABI 47 父 profile。未启用 `androidbox-restart0` 时，既有
Process-0 行为、ABI 与门禁保持不变。

## 已实现的恢复闭环

ABI 48 只允许一次预定、可认证的 AndroidApp worker 恢复：

1. App 与旧 AndroidApp 完成 `Open(1)`，包括完整的 `Opened`、label chunks 与
   Button chunk。
2. App 发送唯一 `Crash(2)` 请求。kernel 只为精确的 App PID、旧 worker PID、
   compatible session、package generation 与 fault point 建立一次性恢复授权。
3. 旧 worker 在 EL0 主动访问其 lower stack guard page，产生精确 data abort：
   `reason=2`、`ESR=0x92000047`、`FAR=0x2001ee000`。
4. scheduler 把终止槽的真实 fault reason、ESR、FAR、ELR、SP 与该代进程实际
   RX/stack bounds 一起交给 reaper。reaper 只接受已授权旧 PID 的
   `Faulted/exit_code=0`、上述精确 reason/ESR/FAR、RX 范围内 ELR 和映射栈范围内
   SP，随后回收地址空间、句柄和进程槽，同时保留一次 package-image escrow。
5. Init 等待该精确 PID，复用同一个动态进程槽并创建 generation+1 AndroidApp。
   Init 通过保留的、已认证 bootstrap Channel 把新的 private App endpoint 交给 App。
6. App 必须先观察旧 Channel 的 `PEER_CLOSED`，关闭旧 endpoint，校验
   `Rebind(old_pid,new_pid)`，再读取新 worker 的 `Ready(0)`。
7. replacement 的第一个已认证 `Open(1)` 触发 kernel 把原 immutable APK VMO
   从旧 PID 重绑定给新 PID。kernel 校验 same slot、generation+1、session、
   package generation、APK length/digest、signer digest、当前安装记录与原 Launcher
   owner；Launcher 不能覆盖或重新发布这一恢复 grant。
8. 新 worker 通过 syscall 61 领取同一个 VMO identity，重新复验 APK digest、
   APK-v2 signer、Manifest、Resources-1 与有界 Activity，再返回新的完整 Open
   response。
9. App 回送 `Rebound`。kernel 只有在 replacement App 的五个 720×1600 layered
   display present 都真实成功后，才发布 `ANDROID_APP_RESTART_OK`；仅完成 RPC
   或 graphics write 不算恢复成功。
10. App 完成 `Click(2) → Updated` 与 `Close(3) → Closed`，再经
    Home→Overview 重新启动同一个 retained compatible identity。这个普通启动继续
    使用 replacement worker，领取新的 ordinary grant，完成
    `Open(4) → Close(5)` 与另外五个 Activity 进入帧。

旧 worker fault 后不会重放未确定请求。本例中的第一次 Open 已经完整结束，所以新
worker 从 request ID 1 建立全新会话；marker 固定
`completed_open_replayed=1 ambiguous_request_replay=0`。

## APK 权限边界

恢复路径没有把 APK bytes 暴露给 Init、Launcher 或 App：

- 原 grant 是绑定旧 AndroidApp PID generation 的一次性、immutable、`READ`-only
  VMO。
- 第一次成功 claim 后，kernel 只在 Restart-0 下保留一个 kernel-private same-VMO
  escrow。
- 只有通过完整 fault/reap/replacement 身份检查的新 AndroidApp generation 才能获得
  reissued grant。
- 新 claim 成功时 kernel 验证 VMO identity 未变化并销毁证明 clone。
- replacement 如果在 reissue 后、syscall 61 claim 前退出，reaper 会同时清除
  ordinary grant、same-VMO proof clone、owner/claim 与 restart state；后续正常
  publish 不会永久停在 `Busy`。
- 正常 grant、escrow 和 reissued grant 不能并存；恢复完成时三者都必须为空。
- duplicate、transfer、map、write、execute、wait、storage authority 继续全部为零。

该设计只覆盖一次、同槽、下一代 AndroidApp worker 替换。它不是通用进程 supervisor，
也不覆盖连续崩溃、任意服务、后台 Activity、系统级 availability 或 crash-loop 策略。

## 进度与二次生命周期约束

Restart-0 不依赖外部 shell timeout 才避免挂死。kernel monitor 为 phase 1–3
分别设置 1,000 tick deadline；进入 phase 4 后，在五个真实 App layered commit
完成并发布 `ANDROID_APP_RESTART_OK` 之前仍继续计时。任何 peer-close、reap、
replacement Ready、rebind、claim、reopen 或首帧停滞都会输出
`ANDROID_APP_RESTART_TIMEOUT` 并 fail closed。

恢复完成后，Restart-0 的 Open/Button 特判立即退出普通 RPC 路径。第二次 lifecycle
必须保持同一个 replacement PID，RPC 状态机直接验证实时 request ID 继续到 5，
并以 `completed_rounds=2 post_complete_messages=7 current_last_request_id=5`
发布 `ANDROID_APP_POST_RESTART_RELAUNCH_OK`。这防止“一次能开、第二次必死”的
伪完成。

## 图形写入修复

真实恢复探测最初在新 worker 已经成功重新 Open 后失败。精确诊断得到首个
`GraphicsBufferWrite` 返回 `Status::OutOfMemory`：mobile path 为每批 22 行、
63,360 bytes 临时申请一个连续 `Vec`，而当前旧 profile 的 kernel heap 只有
256 KiB，回收/替换后的碎片使该大块申请失败。

修复没有扩大 heap，也没有把一次 buffer write 拆成多个 generation：

- mobile-only kernel 使用固定
  `[u8; GRAPHICS_BUFFER_WRITE_MAX_BYTES]` scratch；
- `AtomicBool`/RAII guard 在单核、IRQ-masked syscall domain 内防止重入；
- usercopy 仍按全局 4,160-byte 上限分块复制到 scratch；
- 完整 usercopy 成功后才调用一次 `GraphicsBuffer::write`；
- backing 修改与 write generation 仍保持全有或全无，一次 syscall 只递增一次。

## Mac APK 与自动门禁

门禁脚本：

```sh
CARGO_NET_OFFLINE=true ./scripts/check-androidbox-restart0.sh
```

脚本只使用本机已安装的 Android SDK/JDK 与离线 Cargo cache。它把真实 Java fixture
完整构建两次并要求 APK bytes 相同，再用 `aapt2` 与 `apksigner` 校验：

```text
package=org.bndroid.interactive
activity=Lorg/bndroid/interactive/MainActivity;
apk_bytes=12566
apk_sha256=0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee
signer_cert_sha256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
apk_signature=v2-only
```

门禁精确启动两次 QEMU，每次只有一个 network-disable 参数：

- install boot：通过显式 `fw_cfg` 安装 generation 1；禁止任何 user fault、reap、
  rebind 或 restart marker。
- recovery boot：不提供 APK source，从同一 16 MiB package disk 恢复；只允许并要求
  一个旧 worker 的精确 guard-page fault。

最新完成证据保存在：

```text
target/androidbox-restart0/check.CssUbk/
```

终态：

```text
ANDROIDBOX_RESTART0_QEMU_OK
qemu_boots=2
controlled_faults=1 fault_reaps=1 endpoint_rebinds=1
old_android_app_pid=4294967305 new_android_app_pid=8589934601
same_slot=1 generation_step=1
fault_witness_bound=1 restart_timeout=armed
image_claims=3 image_escrows=1 image_reissues=1 same_vmo=1
reopened=1 post_restart_relaunch=1 ambiguous_request_replay=0
app_layered_commits=12 restart_bound_app_commits=5
rpc_request_order=0/1/2/3 post_restart_request_order=4/5
rpc_click=1 rpc_close=2 rpc_errors=0
content_changed_pixels=4138 label_changed_pixels=4020
recovery_disk_unchanged=1 network=disabled
```

首次点击前、点击后和第二次普通 relaunch 的 720×1600 PPM SHA-256 分别为：

```text
9ea7c996652114b3544a11a1c52f72ff9a9213383f7c00cf033515d862e8c0fa
25aba1af8d3400f36d22d9215797a8d54dd129fbaf99a0fd69cca47f449452cc
9ea7c996652114b3544a11a1c52f72ff9a9213383f7c00cf033515d862e8c0fa
```

top 64 与 bottom 88 system-chrome rows 逐字节相同；source-free recovery 前后
package disk SHA-256 都是
`5fbe4cc046c03cab6ed4b1135a65b763bd0b0577c3caef7f56b1c42bc2f5b813`。
首次与第二次 Activity 初始 raster 逐字节相同。

ABI 47 的独立 `scripts/check-androidbox-process0.sh` 也在这次修改后重新通过，
证据位于 `target/androidbox-process0/check.TuITxg/`；正确 host target
`aarch64-apple-darwin` 下的完整 workspace unit/doc tests 也通过。

## 明确不声称

```text
art=0 dalvik=0 activitythread=0 binder=0 bionic=0
jni=0 native_lib=0 permissions=0 package_manager_api=0
network=0 background_execution=0 arbitrary_apk=0
general_android_compatibility=0 general_process_supervisor=0
general_crash_recovery=0 physical_phone=0 real_phone=0
```

这项结果证明的是：Mac 上构建的一个真实、签名 APK 能在 Bndroid 的严格
Resources-1/InteractiveActivity-1 子集内安装、持久恢复、运行、响应按钮，并在其独立
AndroidApp worker 的一次受控崩溃后重新绑定。它不等价于运行普通 Play Store APK，
更不等价于已经拥有 ART、Binder、完整 Android Framework 或可刷入手机的系统。
