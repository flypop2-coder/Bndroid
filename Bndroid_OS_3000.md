# Bndroid OS 3000 字执行版

> 本文件是历史执行摘要，不再维护独立待办。当前精炼架构、模块合并规则和未来任务
> 统一见 [`TODO.md`](TODO.md)；已验证状态见 `IMPLEMENTATION_STATUS.md`。

> ABI 47 最新覆盖（2026-07-30）：隔离的 `androidbox-process0` 已把固定
> InteractiveActivity-1 的 APK VMO、解释器和会话移入独立 `AndroidApp` EL0
> image；App 仅保留可信输入与光栅。两者通过私有 `BNDAPC01` Channel 完成
> Open→Click→Close，AndroidApp 稳态仅有一个 `READ|WRITE|WAIT` Channel，无
> graphics/input/storage/duplicate/transfer。离线双 boot 门
> `scripts/check-androidbox-process0.sh` 已以 `ANDROIDBOX_PROCESS0_QEMU_OK`
> 通过，证据在 `target/androidbox-process0/check.sFpefj/`。这只证明一个固定
> 受限 APK 子集的独立 Bndroid 进程，不是 ART、通用 Android、崩溃恢复或真机。
>
> 当前增量（2026-07-30，覆盖下方 2026-07-29 的 ABI-43 状态段）：隔离
> `androidbox-apk-install0` profile 已升级到 ABI 44。syscall 59 继续发布只读
> 640-byte `BNDAPS01` boot catalog；Launcher-only syscall 60 接受 generation-bound
> 640-byte `BNDARQ01` 并以 `ShouldWait`/逐字节 exact retry 驱动 IRQ-enabled
> `recover` + `read_blob`。本机 Mac APK 在无 source 恢复 boot 中连续点击两次，
> sequence 1/2 均为 `reads=389 writes=0 flushes=0`，重新复验 APK SHA/v2 signer/
> Manifest/Resources-1 并执行 constructor(2)→`onCreate`(4)，整盘 SHA-256 不变。
> 这仍是 EL1 pure-data Resources-1，不是独立 Android App process、ART/Dalvik、
> ActivityThread、Binder/Bionic/JNI、网络、一般 Android 兼容或真机。
> `CompatibleActivitySession-0` 现已完成 capacity-one、boot-local、identity-only
> Home/Back/Overview 语义：Home 无后台执行，Overview 无 Activity pixels/thumbnail，
> recent activation 重新 syscall 60，Back 清除；`BUC1` v8/`BUE1` v6 两阶段
> Reserve/Commit/Abort 与 `BUP1` v2 System UI revision binding 均 fail closed。
>
> 并行 UI/AndroidBox Resources-1 状态（2026-07-29）：隔离的 `mobile-ui-runtime` 输出精确
> 720x1600 guest scanout，由 360x800 设计网格精确 2 倍光栅化；这只是常见 20:9
> 形态，不是物理尺寸、DPI、刷新率、面板或真机触控证明。Home 保持四 App Dock，
> All Apps 保持五个入口；普通 preview 显示 Launcher-local `AndroidBox Demo`，
> Install-0/Update-0 profile 则只在 kernel snapshot 为 installed 时换成已安装 App
> 的标题与图标。
>
> 底部系统导航与 Overview 由 SurfaceServer 持有：24 像素激活、8 像素量化、
> 上移 240 像素进入 Overview、480 像素回 Home。Overview 在当前会话内最多保存一个
> `ShellAppId` 或 compatible-Activity boot-local 身份；不读取 App 像素，没有
> thumbnail/snapshot/screenshot/live preview、后台任务、task kill、历史列表或持久化。
> 最新 local-APK 门为 31 commits、40 inputs、15 个 Launcher system-UI revisions、
> 6 requests/6 explicit completions；`bndr-ui` 235 项 host tests 通过。
> client-control `BUC1` 为 v8、server-event `BUE1` 为 v6，两条 wire 均保持 64 bytes。
>
> 隔离的 `androidbox-apk-install0` 已完成严格受限的真实安装子集。原始 Install-0
> 实证使用仓库自有、APK v2-only、RSA-2048/SHA-256、单签名的 Resources-1 fixture
> `org.bndroid.demo`（12566 bytes，`versionCode=2`）；APK SHA-256 为
> `2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7`，
> signer certificate SHA-256 为
> `e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf`。
> 同一 profile 又在完全相同的严格四 entry Resources-1 shape 内安装并恢复了本机离线
> 构建的 `org.bndroid.macdemo`；其 Manifest 绑定
> `Lorg/bndroid/macdemo/MainActivity;`，DEX 不含
> `Lorg/bndroid/demo/Main;`、`boot()` 或 `onTap()` probe。这只证明
> Manifest-driven component admission，不是任意 APK 或一般 Android 兼容。
> fixture 私钥与证书是仓库公开的 test-only 复现材料，不是生产密钥、发布者信任或 HSM
> 托管声明。
>
> 同一隔离 profile 现已完成严格受限的 APK Update-0：同包、同 signer certificate 且
> `versionCode` 严格增加时，双 registry/双 blob 从 generation 1/slot 0 的版本 2
> 原子切换到 generation 2/slot 1 的版本 3。v3 APK 同为 12566 bytes，SHA-256 为
> `3860fcd80eed1a4b284514316bc35169dda60bb1522e85abdea5aa33b8355708`；
> 持久 readback 显示 `AndroidBox Resources-1 Demo v3` 与
> `AndroidBox updated resource view`。
>
> 本地 QEMU 门先证明篡改 APK 在首个 package write 前拒绝且磁盘不变；随后首启经
> read-only `fw_cfg` 安装 generation 1，并从完整 package-store readback 执行真实
> `onCreate(Bundle)V→setContentView(0x7f020000)`，I/O 为
> `reads=1032 writes=130 flushes=3`。同一可写磁盘再进行两次完全无 APK source 的
> 恢复启动，均从持久 APK 显示 `AndroidBox resource-backed view`，且每次
> `reads=389 writes=0 flushes=0`。整盘 diff 的变化严格限定于
> `BNDROID_PACKAGES` LBA `16384..16895`。
>
> Update-0 的六启动本地 QEMU 门继续证明 generation 1→2 更新
> `reads=905 writes=129 flushes=2`、精确 v3 source replay 与无源恢复均
> `writes=0 flushes=0`、旧 v2 rollback 和篡改 v3 均在磁盘变化前拒绝。generation 2
> 整盘 SHA-256 为
> `d3166bc25b95d28d3a18ee612fe03be96d342d2b37ac620947fe71828abba1de`；
> install/update 的全部变化仍只在 `BNDROID_PACKAGES`。
>
> ActivityLifecycle-1 只在纯数据 framework model 中解释 Manifest Activity 的 exact
> public no-argument constructor：两条指令完成受限 `Activity.<init>()V` super call 与
> `return-void`，随后才解释 `onCreate`；它不构造 ART 对象，也不是一般 Android lifecycle。
>
> package-store 当前 43 项 host 单测另行覆盖 update 的每个 write 失败点、每个
> torn-write prefix、各 flush 故障点、ack loss、幂等重试和最新 registry/blob
> 损坏回退；这些是内存块设备 fault injection，不是 43 次 QEMU、QEMU host-cut、
> 物理断电、控制器 cache 或真机存储证明。Update-0 QEMU 实证是四次正启动和两次
> pre-mutation 负启动，没有事务中途 cut。
>
> 该隔离 profile 使用 ABI 43 和只读 syscall 59：向 EL0 复制 640-byte `BNDAPS01`
> 安装快照，不暴露 APK bytes、handle 或 package-store 权限。Settings 新增 Apps 页，
> 显示版本、大小、generation、Resources-1 profile 与 digest 前缀；All Apps 的已安装
> 入口使用 generation-bound touch token，打开的只是 kernel 已验证输出。generation 2
> 的 720x1600 UI 已显示 Version 3、Generation 2 与更新后的 TextView；仍没有 install/
> update/uninstall UI 或通用 PackageManager API。
>
> 边界仍为 `art=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
> permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled
> real_phone_claim=0`。它不是任意 APK installer、一般 Android 兼容、网络 App 运行或真机
> 证明；完整细节见 `ANDROIDBOX_APK_INSTALL_0.md` 与
> `ANDROIDBOX_APK_UPDATE_0.md`。

> 当前执行状态（2026-07-28）：head 为 ABI-v41/M80
> `unified-product-maintenance-plan-runtime`。它是 M79 matched child，不新增
> syscall；ABI v41 仍开放 0—57，raw 58 unknown。M77/M78/M79 的
> `BNDRMAU1`/`BNDRMEX1`/`BNDRMST1` 双槽账本保留在 DATA 相对 sector
> 5/6、7/8、9/10；M80 在 11/12 增加独立 424-byte 双槽 `BNDRMPL1`。

> 三个固定 resident operation 各绑定 exact plan ID、operation-instance ID、
> idempotency key、effect digest 与前一 SHA-256 chain。normal 对
> rotation1/rotation2/drain 严格执行九次 `PREPARED→APPLYING→CONFIRMED`；
> 只有 apply 前 `PREPARED→COMPENSATED` 合法。APPLYING 恢复区分
> result-unknown 与 exact-effect-observed-unconfirmed，terminal plan chain 在
> aggregate completion 前重读并绑定。

> 专项 release 门通过 normal、prepared/applying/effect 三处宿主 cut 与恢复、
> cancel-prepared、corrupt-newest fallback/repair 和五份终态磁盘收敛。专项日志
> `target/m80/unified-product-maintenance-plan-runtime.log` 为 3,609 行、
> 752,631 bytes，SHA-256
> `f711ad753cc21cf05abec301d664026058a2f9be4b1f786ac032eca98be2f729`；
> 三张终态截图 SHA-256 均为
> `1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994`。

> 完整离线 `scripts/test.sh` 同样 exit 0；`target/full-test-m80-rerun.log`
> 为 7,945 行、971,891 bytes，SHA-256
> `a7ef8f08ea5fac5c90ec104d9970228b8c853854edae573b1646bc1ff68081dd`。
> 最终聚合为 58 次 QEMU 自退出（50 次 PSCI），M80 5 次正启动、4 次宿主中断，
> 并通过 11 类存储负向、持久化恢复和 storage-IRQ race。

> M80 还修复 shutdown proof 跨轮采样 race：先捕获 shutdown snapshot，再采集
> subsystem evidence，最终 proof 只使用同一 snapshot generation；未增加 retry 或放宽
> fail-closed。系统仍是 bounded single-core QEMU research prototype；宿主 cut 不是
> physical power-cut，固定幂等 effect 不提供外部 exactly-once，QEMU disk 不是 trusted
> monotonic backend。没有 HSM/RPMB/eFuse、BSP/PMIC、真实控制器、刷写或真机证据，
> 仍不可日用；硬件或真机工作必须先由用户指定目标并另行明确授权。

> 历史执行状态（2026-07-28）：head 当时为 ABI-v40/M79
> `unified-product-maintenance-step-runtime`。它是 M78 matched child，
> 不新增 syscall；ABI v40 仍开放 0—57，raw 58 unknown。M77 `BNDRMAU1`
> 授权审计保留在 DATA 相对 sector 5/6，M78 `BNDRMEX1` 聚合完成账本保留在
> 7/8；M79 在 9/10 增加独立 376-byte 双槽 `BNDRMST1` step journal。

> journal 固定记录两次 clean StorageServer rotation 和 resident drain。每步精确绑定
> authorization、manifest/key policy/root/device、确定性 effect digest、前一 chain、
> sequence/base/entry-count/ordinal；pre-EL0 在 audit mutation 前预读三套双槽。空 journal
> 只允许从已完成 M78 sequence-1 迁移，已持久的较低/相同步骤只能只读 reconciliation，
> terminal drain 必须在 aggregate completion 前重读并把 chain 绑定进 runtime digest。

> 专项门通过 normal、步骤 1/2/3 durable marker 后宿主 cut 与三次恢复、
> corrupt-newest fallback/repair，以及坏签名、错误 binding、completed replay 三次
> pre-EL0 零槽改写拒绝；共 5 次 PSCI 正启动、3 次中断、7 次幂等 step replay。
> 完整离线套件 exit 0：`target/m79-full-suite.log` 为 7,124 行、885,912 bytes，
> SHA-256 `d4ca8422024a492b19c02aa8ef7f096430c376da99adb95b78806335ec032b27`；
> 最终聚合为 `qemu_psci_self_exits=40 qemu_self_exits=48`。

> 这仍是 bounded single-core QEMU research prototype。宿主 cut 不是物理 power-cut；
> 只读 reconciliation 只适用于固定幂等 resident 程序，不提供外部副作用 exactly-once
> 或 arbitrary resume。host-controlled disk 不是可信单调源；没有生产 HSM/RPMB/eFuse、
> BSP、PMIC、真实控制器、刷写或真机证据，仍不可日用。下一本地 P0 是带
> operation-instance ID、idempotency key、prepare/apply/confirm/compensate 与
> result-unknown 恢复的通用持久维护计划；硬件或真机工作仍须用户指定目标并另行明确授权。

> 历史执行状态（2026-07-28）：head 当时为 ABI-v39/M78
> `unified-product-maintenance-execution-runtime`。它是 M77 matched child，
> 不新增 syscall；ABI v39 仍开放 0—57，raw 58 unknown。M77 的授权审计保留在
> `BNDROID_DATA` 相对 sector 5/6，M78 在相对 sector 7/8 增加独立 368-byte
> 双槽 `BNDRMEX1` execution-completion ledger；两者都执行 CRC/witness、
> exact generation、SHA-256 chain、inactive-slot write、flush、完整 readback 与复验。

> pre-EL0 admission 同时检查两套 head：新授权只有在前一 sequence 已完成时才能推进；
> `audit=N/execution=N-1` 只允许精确同一 BMA1 零 audit 写恢复；两者都为 N 时重放 N
> 以 `completed-replay` 失败关闭。kernel 只在验证 4 次 use、两次 clean rotation、
> service drain 与 shutdown/runtime evidence 后提交完成记录，并且提交发生在
> device-health close/final seal 之前。

> 专项门完成 sequence 1 后，在 sequence 2 durable admission 之后、completion 之前
> 由宿主终止 QEMU，磁盘为 `audit2/execution1`；精确同授权重启恢复到
> `audit2/execution2`，audit 不变。坏签名、错误 binding 和 completed replay 三次
> 负启动均在 EL0 前零槽改写拒绝。完整离线套件 exit 0：
> `target/m78-full-suite.log` 为 6,270 行、799,409 bytes，SHA-256
> `9b5924935782d511994635a0231d76eb77f4d70e6caef7bc831c6b4dfc01f801`；
> 最终聚合为 `qemu_psci_self_exits=30 qemu_self_exits=38`。

> 这仍是 bounded single-core QEMU research prototype。宿主中断不是物理 power-cut；
> 该协议不声称 exactly-once 或 arbitrary resume，host-controlled disk 也不提供
> trusted monotonic、replay/erase/tamper resistance。没有生产 HSM/RPMB/eFuse、BSP、
> PMIC、真实控制器、刷写或真机证据，仍不可日用。下一本地 P0 是固定维护操作的持久
> step journal、幂等/补偿规则与更多 cut-point/corruption；任何硬件或真机工作仍须
> 用户指定目标并另行明确授权。

> 历史执行状态（2026-07-28）：head 当时为 ABI-v38/M77
> `unified-product-maintenance-authorization-runtime`。它是 M76 matched child，
> 新增 init-only syscall 57 `MaintenanceSessionOpen`；ABI v38 开放 0—57，
> raw 58 unknown。任何 EL0 启动前，kernel 先对 512-byte BMA1 做 signature-first
> 校验，再精确绑定 product manifest、ordered key policy、device identity、
> maintenance policy、operation/max-uses/id/sequence。

> 接受的授权必须先提交到 `BNDROID_DATA` 相对 sector 5/6 的双槽 `BNDRMAU1`：
> exact-next-sequence、SHA-256 hash chain、旧槽保留，以及 write→flush→完整
> readback→复验全部成功后才能继续启动。sequence 1/2 已分别提交为 slot generation
> 1/2。只有 init 能打开一次 boot-local session；它门禁 mutating
> `ServiceSupervisorReport`，只读 UI convergence query 不需要 maintenance authority。

> 专项 release 门通过 6 次正启动与 3 次 pre-EL0 负启动；坏签名、错误 binding 和
> replay 均零 manifest publication、零 audit-slot mutation。完整离线套件 exit 0：
> `target/m77-full-suite.log` 为 5,520 行、721,693 bytes，SHA-256
> `f9203c32fc02a0a7958c87481a98b0235a1a070559e10c334cb443761f75354d`；
> 最终聚合固定 `qemu_psci_self_exits=24 qemu_self_exits=32`。

> 这仍是 single-core QEMU research prototype。maintenance root 是 fixture，
> audit backend 是 host 可控磁盘，明确
> `trusted_monotonic_backend=0 production_key_claim=0 hsm_claim=0 rpmb_claim=0
> efuse_claim=0 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0`；
> 不是真机、不可刷机、不可日用。下一本地 P0 为威胁模型、production custody/update/
> recovery interface、更多 cut-point/corruption、重复授权/轮换与长 soak。BSP、PMIC、
> RPMB/eFuse、刷写和真机操作必须等用户指定目标并另行明确授权。

> 历史执行状态（2026-07-28）：head 当时为 ABI-v37/M76
> `unified-product-key-rotation-runtime`。它严格继承匹配的 M75 closure，不新增
> syscall；ABI v37 仍只开放 0—56，raw 57 unknown。pre-EL0 verifier 使用有序三锚点
> fixture keyring，key id/epoch 为 2/2、3/3、4/4；规范 policy SHA-256 为
> `30676f357683b6c8d2c5d67755e48beb93bdfe8864eccc67c716a1785e935e01`。

> `BNDROID_DATA` 相对 sector 3/4 的双槽现在用 96-byte `BNDRKEY1` 同时绑定 rollback
> floor、active key epoch/id、active trust-anchor SHA-256 与有序 keyring-policy
> SHA-256。签名校验先于任何 policy mutation；持久 epoch 还必须授权所选 anchor，因此
> 已退休 key 即使签名正确也会 fail-closed。

> 同一磁盘的五次正启动依次为：历史 M75 key 2/floor 3 seed、key 3 generation/floor 4
> 过渡、key 4 generation/floor 5 激活、key-4 冗余修复和 key-4 零写稳态。最终两槽
> generation 为 3/4，M76 policy 阶段合计 `reads/writes/flushes=14/3/3`；每次接受启动
> 均保留两次 clean StorageServer rotation、精确 UI 与 PSCI self-exit。损坏 key-4 签名
> 和由已退休 key 3 正确签名的 generation-6 制品均在 EL0 前拒绝，零 manifest publication、
> 零 slot mutation。

> M76 还增加离线 split-signing：仓库侧工具只生成精确 256-byte request，并从 request、
> 256-byte detached signature 与 pinned public PEM 组装并复验 512-byte BMS1；它没有
> private-key/signing 输入。仓库只含 public anchor 与 fixture signature。M33–M76 有
> 44 个 opt-in leaf，加 default 共 45 份账本；52 个 literal shell QEMU launch 全部恰有
> 一个 `-nic none`。专项静态、host 和七次 QEMU 启动门均已通过。完整离线套件也已
> 从头运行并以 exit 0 通过，共 4,496 行；封存日志 `target/m76-full-suite.log` 的
> SHA-256 为 `f4066b7cd057da32ea71af3a6df446af458e7a4130bd254b76fbb09073aa9e95`。
> 最终 `BNDROID_TEST_SUITE_OK` 聚合包含
> `unified_product_key_rotation_reboot=1 unified_product_key_rotation_boots=5
> key_rotation_fail_closed_pre_el0_boots=2 key_rotation_floor=5
> key_rotation_policy_writes=3 key_rotation_steady_read_only=1
> qemu_psci_self_exits=18 qemu_self_exits=26`。

> 这只是 writable QEMU disk 上的 fixture-key rotation proof，不是生产 key management
> 或硬件 anti-rollback：没有 HSM/key ceremony、生产授权/恢复、可信 RPMB/eFuse 单调源，
> 也不能抵抗 privileged-host replay/erase/tamper 或证明 hardware power-cut。当前仍为
> bounded single-core QEMU research prototype：
> `fixture_keys=1 production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0
> host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0
> hardware_powercut_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，
> 不是真机、不可刷机、不可日用。

> 下一本地 P0：生产 HSM-backed custody、key ceremony、授权/恢复、签名 root-of-trust/
> update metadata、可信单调后端与威胁模型，外加 maintenance 认证/审计、更广 cut-point/
> corruption、任意多次 rotation、cancel race 和长时间 soak。BSP/真机/刷写仍须用户先
> 指定目标并明确授权。

> 历史执行状态（2026-07-27）：M75 为 ABI-v36
> `unified-product-persistent-rollback-runtime`。它严格继承匹配的 M74 closure，不新增
> syscall；ABI v36 仍只开放 0—56，raw 57 unknown。任何 EL0 进程启动前，kernel 先以
> fixture trust anchor key id 2 校验 generation/index-3 BMS1，再完成 `BNDROID_DATA`
> 相对 sector 3/4 上的双槽 `BNDRRBK1` rollback transaction；只有两步都成功，后续才允许
> 发布 immutable manifest VMO。

> 空 ledger 以 bootstrap floor 2 起步：首次启动写 slot 0/generation 1、flush/readback，
> 把 floor 推进到 3；第二次保留 slot 0，并写 slot 1/generation 2 修复冗余；第三次选择最新
> 槽且保持零 write/零 flush。三次正启动均保留 UI、AppData、两轮 clean StorageServer
> rotation 与 PSCI self-exit。损坏签名在 ledger I/O 前拒绝；正确签名的旧 index-2 制品在
> 持久 floor 3 前拒绝。两条负启动均发生在 EL0/manifest publication/init-ready 前，两槽
> 逐字节不变。专项合计 `ledger_reads/writes/flushes=10/2/2
> rejected_boot_slot_mutations=0 fail_closed_pre_el0_boots=2 qemu_psci_self_exits=3`。

> 这是同一 writable QEMU disk 的持久状态，不是硬件 anti-rollback：key id 2 仅为 fixture，
> 仓库没有 private key；没有生产 key custody/rotation、RPMB/eFuse、host rollback/erase/
> tamper resistance 或硬件 power-cut 证明。M33–M75 有 43 个 opt-in leaf，加 default 共
> 44 份账本；52 个 literal shell QEMU launch 全部恰有一个 `-nic none`。当前仍是 bounded
> single-core QEMU proof：`production_key_claim=0 rpmb_claim=0 efuse_claim=0
> host_rollback_resistance=0 erase_resistance=0 tamper_resistance=0
> hardware_powercut_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是真机、
> 不可刷机、不可日用。

> M75 完整离线套件已从头 exit 0，共 4,068 行；封存日志
> `target/m75-full-suite.log` 的 SHA-256 为
> `a370baa795702627a9808fbeba5923406d9ec7128aaab9e4781360682a022332`。最终聚合固定
> `unified_product_persistent_rollback_reboot=1
> unified_product_persistent_rollback_boots=3
> persistent_rollback_fail_closed_pre_el0_boots=2 persistent_rollback_floor=3
> persistent_rollback_slot_writes=2 persistent_rollback_steady_read_only=1
> qemu_psci_self_exits=13 qemu_self_exits=21`。

> M75 当时的下一本地 P0：生产级 key custody/rotation 与离线签名制度、带明确威胁模型的可信单调后端、
> maintenance control 认证/审计，以及更广 cut-point/意外损坏、任意多次 rotation、
> cancel race 和长时间 soak。任何 BSP/真机/刷写继续要求用户先指定目标并明确授权。

> 历史执行状态（2026-07-27）：M74 为 ABI-v35
> `unified-product-verified-manifest-runtime`。它严格继承匹配的 M73 closure，不新增
> syscall；ABI v35 仍只开放 0—56，raw 57 unknown。kernel 从外部 512-byte BMS1
> 制品读取 32-byte 严格 envelope、224-byte BMF1 payload 和 256-byte RSA signature，
> 在发布 immutable manifest VMO 前，以内置 key id 1 执行 RSA-2048 PKCS#1 v1.5
> SHA-256 校验，并要求 rollback index 2 不低于静态 floor 2。

> 正向 generation/index-2 制品通过；有效签名的 generation/index-1 制品被 rollback
> floor 拒绝，损坏签名制品被 signature gate 拒绝。两个负向 QEMU 启动都先收敛真实 UI，
> 然后在 manifest 发布和 init-ready 前以 `0x7201` 停止；两次正向 release 启动完整保留
> M73 的事件监督、每次两次正常 StorageServer rotation、AppData 持久化、精确 UI screenshot
> 与 PSCI self-exit。`artifact_verifications=2 signature_successes=2
> negative_signature_boots=1 negative_rollback_boots=1 fail_closed_boots=2
> manifests_published_after_rejection=0`。

> 这是实际 RSA 验证，不是 production secure boot。仓库没有 private key；测试 key 没有生产
> 托管/轮换制度，rollback floor 是 kernel 静态常量，不是 RPMB/eFuse。M33–M74 有 42 个
> opt-in leaf，加 default 共 43 份账本；52 个 literal shell QEMU launch 全部恰有一个
> `-nic none`；累计 PSCI/QEMU self-exit 为 10/18。当前仍是 single-core QEMU proof：
> `production_key_claim=0 hardware_rollback_claim=0 emulator_only=1 general_runtime=0
> real_phone_claim=0`，不是真机、不可刷机、不可日用。
> 完整离线套件 3,937 行已通过，`target/m74-full-suite.log` 的 SHA-256 为
> `c100415c2f9187047d17aa67fd08f939aac50a46d9a22572a8e3841609653565`。

> M74 当时的下一本地 P0：生产级 key custody/rotation 与离线签名制度、可原子推进/恢复的持久单调 rollback
> state，以及 maintenance control 的认证与审计；随后覆盖任意多次 rotation、意外故障、
> cancel race 和长时间 soak。任何 BSP/真机/刷写继续要求用户先指定目标并明确授权。

> 历史执行状态（2026-07-27）：M73 为 ABI-v34
> `unified-product-event-supervision-runtime`。它在匹配的 M72 closure 上新增 init-only
> syscall 56 `ServiceSupervisorReport`：operation 0 只读查询 kernel-owned UI convergence
> seal，`ShouldWait`/success 独立计数，不返回 handle 或 authority；其余 operation 由 kernel
> 依据 generation-qualified process ledger 和唯一 live StorageServer PID 校验转换顺序。
> UI 收敛后 init 进入多 Channel event loop 并持续到认证 power event。proof profile 的两次
> 外部 F5 都走 StorageServer 正常 shutdown→`Exited`→App block→30 ms backoff→同槽下一代
> spawn/remount→两轮稳定 Healthy→budget rearm，不注入 health miss，不调用
> `ProcessTerminate`，也不模拟 kill。power 前保留 4 个已发 probe，随后 stop 并全部 drain，
> 再进入 AppData/resident/PSCI shutdown。

> 两次同盘 release gate 已通过：`clean_rotations=4 process_exit_faults=4
> process_terminate_calls=0 terminated_killed=0 restart_budget_rearms=4 cancel_windows=2
> cancelled_pending=8 drained_pending=0 qemu_psci_self_exits=2`，UI hash 为
> `1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994`。M73 为 ABI v34；
> M72 raw 56 与 M73 raw 57 均 unknown。M33–M73 41 个 opt-in leaf 加 default 共 42 份账本；
> 52 个 literal shell QEMU launch 全部恰有一个 `-nic none`；累计 PSCI/QEMU self-exit 为
> 8/16。它仍是 single-core QEMU proof：
> `emulator_only=1 general_runtime=0 real_phone_claim=0`，不是真机、不可刷机、不可日用。

> M73 当时的下一本地 P0：signed/verified/anti-rollback boot manifest 与经过认证/审计的 maintenance
> policy，随后覆盖任意多次 rotation、意外故障、accepted-session cancel race 和更长 soak。
> 任何 BSP/真机/刷写继续要求用户先指定目标并明确授权。

> 历史执行状态（2026-07-27）：M72 为 ABI-v33
> `unified-product-manifest-supervision-runtime`。它在匹配的 M71 closure 上新增 init-only
> syscall 55 `ServiceManifestOpen`；kernel 先验证内置 immutable 224-byte BMF1
> v1/generation-1 manifest，再以 read-only VMO 交给 init 做 allocation-free、事务式解析。
> production 顺序刻意为 App→ServiceManager→StorageServer→InputServer→SurfaceServer；
> 4 个 resident binding，StorageServer 的初次 spawn 与 replacement image 都来自 manifest。
> parser 支持容量 5 services/10 dependencies 内的合法子集和排列，但当前产品校验仍锁定五种已知
> service 与既定 4 边，故 `arbitrary_service_set_claim=0`。FNV fingerprint 不是签名，manifest
> 仍是 kernel 编译期常量，并非 signed boot/OTA artifact。

> M72 保留每次 21 轮/107 probes（104 Healthy、3 个注入 miss）的 M71 有界窗口；两次同盘
> release 启动均完成真实项目 UI/AppData/shutdown closure 与 FDT-discovered PSCI 1.1 QEMU
> self-exit。它仍是 single-core emulator research prototype：
> `bounded=1 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是真机、不可刷机、不可
> 日用。M72 为 ABI v33，M71 raw 55 仍 unknown，M72 raw 56 unknown；M33–M72 40 个 opt-in
> leaf 加 default 共 41 份账本。当前 85 个顶层 shell scripts；52 个 literal shell QEMU launch
> 全部恰有一个 `-nic none`；PSCI self-exit 合计 6，QEMU self-exit 合计 14。全量回归还修复了
> 历史 M47 permission counter 晚于 trace commit 的竞态；专项连续 5 次和完整 suite 均通过。

> M72 当前精确证据：

```text
UNIFIED_PRODUCT_MANIFEST_SUPERVISION_OK format=1 abi=33 authority=kernel-owned-immutable-bmf1-vmo-plus-init-transactional-binding-plus-kernel-authenticated-pid manifest=BMF1 manifest_generation=1 manifest_bytes=224 manifest_fingerprint=0xd69d11fdbc0fc7a9 manifest_open_calls=2 manifest_open_successes=1 manifest_argument_rejections=1 manifest_permission_denials=0 parser_transactional=1 bounded_service_capacity=5 bounded_dependency_capacity=10 services=5 service_order=App+ServiceManager+StorageServer+InputServer+SurfaceServer legacy_literal_order_independent=1 resident_bindings=4 manifest_spawns=1 service_set=ServiceManager+SurfaceServer+InputServer+StorageServer+App protocol=BSH1 dependency_edges=4 hard_edges=3 soft_edges=1 probes=107 healthy=104 missed=3 cadence_waits=21 cadence_ms=40 periodic_rounds=21 fully_healthy_rounds=20 healthy_soak_rounds=16 batch_rounds=18 batched_probes=90 health_timeouts=3 health_timeout_ms=100 concurrent_miss_windows=1 concurrent_miss_services=2 missed_probe_tolerance=1 transient_miss_recoveries=2 escalated_faults=1 backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 old_pid=4294967306 replacement_pid=8589934602 app_pid=4294967305 same_slot=1 generation_step=1 process_terminate_calls=1 process_terminate_successes=1 terminated_killed=1 storage_epochs=2 next_epoch=3 releases=2 fault_transitions=2 recovery_transitions=2 tolerated_miss_dependency_transitions=0 dependent_blocks=1 dependent_resumes=1 resident_health_sequences=21 app_health_sequences=21 replacement_health_sequences=20 replacement_mounted=1 replacement_healthy=1 elapsed_supervision_ms=1070 bounded=1 arbitrary_service_set_claim=0 injected_storage_hang=1 injected_transient_misses=1 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_MANIFEST_SUPERVISION_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 manifest_supervision_recoveries=2 manifests_decoded=2 manifest_open_calls=4 manifest_open_successes=2 manifest_argument_rejections=2 manifest_permission_denials=0 manifest_services=10 resident_bindings=8 manifest_spawns=2 supervised_services=10 liveness_dependency_edges=8 health_probes=214 healthy=208 missed=6 health_timeouts=6 cadence_waits=42 periodic_rounds=42 fully_healthy_rounds=40 healthy_soak_rounds=32 batch_rounds=36 batched_probes=180 concurrent_miss_windows=2 concurrent_miss_services=4 transient_miss_recoveries=4 escalated_faults=2 dependency_fault_transitions=4 dependency_recovery_transitions=4 dependent_blocks=2 dependent_resumes=2 killed_servers=2 replacements=2 replacement_healthy=2 storage_epochs=4 psci_discoveries=2 psci_version_probes=2 psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 psci_system_off_requests=2 qemu_psci_self_exits=2 semihosting_uses=0 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_MANIFEST_SUPERVISION_STATIC_OK source=1 feature_closure=1 abi=33 syscall=55 manifest=BMF1 generation=1 bytes=224 services=5 dependency_edges=4 bounded_capacity=5/10 nonlegacy_order=1 resident_bindings=4 manifest_spawns=1 parser_transactional=1 parser_serial_negative=26 parser_host_negative=3 semihosting=0 full_suite=1 bounded=1 arbitrary_service_set_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 app_data_async_recovery=1 storage_server_static=1 storage_server_recovery_static=1 storage_server_repeated_recovery_static=1 storage_server_async_recovery_static=1 storage_server_fault_policy_static=1 storage_server_owner_liveness_static=1 storage_server_terminal_quarantine_static=1 storage_server_persistent_health_static=1 storage_server_clean_shutdown_static=1 storage_server_shutdown_orchestration_static=1 resident_platform_shutdown_static=1 unified_product_static=1 unified_product_liveness_static=1 unified_product_multiservice_liveness_static=1 unified_product_psci_shutdown_static=1 unified_product_continuous_supervision_static=1 unified_product_manifest_supervision_static=1 storage_recovery_unification_static=1 storage_server_runtime_boots=3 storage_server_recovery=1 storage_server_repeated_recovery=1 storage_server_async_recovery=1 storage_server_fault_policy=1 storage_server_owner_liveness=1 storage_server_terminal_quarantine=1 storage_server_persistent_health_reboot=1 persistent_health_boots=2 storage_server_clean_shutdown_reboot=1 clean_shutdown_boots=2 storage_server_shutdown_orchestration_reboot=1 shutdown_orchestration_boots=2 resident_platform_shutdown_reboot=1 resident_platform_shutdown_boots=2 unified_product_reboot=1 unified_product_boots=2 unified_product_ui_interactions=2 unified_product_liveness_reboot=1 unified_product_liveness_boots=2 unified_product_liveness_recoveries=2 unified_product_multiservice_liveness_reboot=1 unified_product_multiservice_liveness_boots=2 unified_product_multiservice_liveness_recoveries=2 unified_product_psci_shutdown_reboot=1 unified_product_psci_shutdown_boots=2 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 unified_product_manifest_supervision_reboot=1 unified_product_manifest_supervision_boots=2 manifest_supervision_decodes=2 manifest_supervision_services=10 manifest_supervision_resident_bindings=8 manifest_supervision_spawns=2 manifest_open_calls=4 manifest_open_successes=2 manifest_argument_rejections=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6 qemu_psci_self_exits=6 qemu_self_exits=14 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1 storage_irq_cooperative_recovery=1
```

> M72 当时的下一本地 P0：非注入、事件驱动、可干净取消的长期 supervisor，覆盖背靠背独立升级故障；并把
> manifest 改为 verified boot artifact，而不是 kernel 常量。任何真机/BSP/刷写工作继续要求用户
> 明确目标并授权。

> 执行状态（2026-07-27）：历史 M71 里程碑为 ABI-v32/M71 `unified-product-continuous-supervision-runtime`。它严格扩展匹配的 M70 kernel/userspace closure，字面 kernel chain 为 M71→M70 `unified-product-psci-shutdown-runtime`→M69 `unified-product-multiservice-liveness-runtime`→M68 `unified-product-liveness-runtime`→M67 `unified-product-runtime`→M66 `resident-platform-shutdown-runtime`→M65 `storage-server-shutdown-orchestration-runtime`→M64→M63→M62→M61→M60→M58→M57→M56→M55。M71 不新增 syscall、镜像或 capability right，syscall 上限仍为 54；ABI v32 只更新证据契约。它保留历史 M70 的 strict FDT/PSCI 1.1、无 semihosting QEMU `SYSTEM_OFF`，并新增事务式五服务目录（ServiceManager、SurfaceServer、InputServer、StorageServer、App）、4 条依赖（3 hard、1 soft）、事务式批量 probe、每服务 missed-probe tolerance=1、16 轮额外健康 soak，以及 SurfaceServer/InputServer 同轮各漏一次后独立恢复；StorageServer 连续漏报仍走真实 100 ms timeout、30 ms backoff、同槽下一代 replacement 与 App block/resume。每次启动精确为 21 轮、107 probes、104 Healthy、3 missed，20 轮全健康、18 个 batch/90 个 batched probes、2 个瞬态恢复和 1 个升级故障；两次同盘启动都通过真实 UI/AppData closure 与 PSCI self-exit。它仍是 bounded、故障注入、single-core QEMU 研究原型，`arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是 PMIC、硬件 poweroff 或真机证明。feature-off/default 仍为 ABI v23，历史 M55—M64 为 ABI v25；M65/M66/M67/M68/M69/M70/M71 分别为 ABI v26/v27/v28/v29/v30/v31/v32；历史 M59 双账仍隔离。M33—M71 共有三十九个 opt-in leaf，加 default M32 为四十份独立账本，任何 profile 的计数、权限或 marker 都不能跨账拼接。

> 历史 M56 是 kernel-only fail-stop recovery：`OutcomeUnknown`/`RequiresReset` 都是 session-fatal。旧 StorageServer 退出、session/capability 清理和 volume unbind 完成后，kernel 才执行 virtio status 0 reset，复核 device identity、相同 features/capacity，以仍独占的 DMA pages 重建 queue，并用 prepare/commit 两阶段重新 arm IRQ；旧 GIC pending/active 只在最初 disable 时清一次，re-enable 后保持 pending，并在 DAIF 屏蔽下完成 ISR/queue/status tail audit 后才 commit。失败则 rollback 并继续 fail-closed。新 StorageServer 只能以 owner epoch `+1` reacquire 并 remount durable volume，旧 session 不恢复。物理 commit 从不隐式开 admission：M55/M56/M57 同步路径为 `rearm→open→broker`，历史 M59 AppData/timeout 为 `rearm→open→finalize/return`，历史 M58 为不对称的 `rearm→broker→open`。历史 M60 在 ownerless、DAIF-masked 的提交窗内改为 `rearm→open→prearm→broker→success-ledgers→restore-DAIF`；`prearm` 只在第 7 个 campaign epoch 由 kernel 授权，broker complete 后的成功账本也在恢复 DAIF 前发布。build wrapper 现闭合 M66→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55 kernel chain；M65/M66 都需要匹配的 kernel/userspace feature，M63/M64 userspace 请求继续被拒绝，并拒绝 userspace 越级及 AppData/StorageServer/timeout 混配。

> 历史 M56 directed runtime 对 read/write/flush 各一次性抑制 QueueNotify：read 得到 `RequiresReset`，write/flush 得到 `OutcomeUnknown`，三例均完成旧 owner 退出、kernel reset 与 replacement remount。其精确历史 marker 为：

```text
STORAGE_SERVER_RECOVERY_OK cases=3 read_requires_reset=1 mutation_outcome_unknown=2 control_sequences=3 injected_reads=1 injected_writes=1 injected_flushes=1 owner_exits=3 broker_releases=3 broker_abandoned=3 reset_attempts=3 reset_successes=3 reset_failures=0 driver_timeouts=3 driver_resets=3 final_epoch=4 final_generation=1 requests=7813 completions=7810 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1
BOOT_OK: M56 fail-stop StorageServer recovery verified
```

> 历史 M57 新增两轮**串行** `WRFWRF`，不宣称并发。driver 实际消费 read/write/flush suppression `2/2/2`；第 4 次 recovery attempt 注入 IRQ rearm commit abort 并 fail-closed rollback，第 5 次才是真正成功 retry，最终 attempts/commits/rollbacks=`7/6/1`。IRQ-sensitive broker access 采用 IRQ-masked 纪律；Running owner close 记录 `ServiceAbandoned` 而非 panic；physical submission gate 在 recovery latch 下拒绝新物理提交。成功实测 marker 为：

```text
STORAGE_SERVER_REPEATED_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF read_requires_reset=2 mutation_outcome_unknown=4 control_sequences=6 injected_reads=2 injected_writes=2 injected_flushes=2 owner_exits=6 broker_releases=6 broker_abandoned=6 recovery_attempts=7 recovery_commits=6 recovery_rollbacks=1 injected_rearm_aborts=1 fail_closed_retries=1 driver_timeouts=6 driver_resets=7 final_epoch=7 final_generation=1 requests=9625 completions=9619 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1
BOOT_OK: M57 repeated fail-stop StorageServer recovery and retry verified
```

> 历史 M58 StorageServer 分支前身保留 M57 的 `WRFWRF`、`7/6/1` 与 request delta 6，并让七次物理恢复 cooperative：driver borrow 跨相位释放，每个 cooperative step 对 reset status 或带 generation 的 capacity tuple 至多采样一次，每个相位只做短 DAIF 计时；outer coordinator attempt id 贯穿 physical recovery、IRQ rearm、broker commit 与 admission，gate 直到完整 commit 后才开。真实 `timer_dispatches`、双 worker 与已认证 EL0 在每个恢复窗口都取得进度，masked polling 为零。完整历史实测为：

```text
STORAGE_SERVER_ASYNC_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF read_requires_reset=2 mutation_outcome_unknown=4 control_sequences=6 injected_reads=2 injected_writes=2 injected_flushes=2 owner_exits=6 broker_releases=6 broker_abandoned=6 recovery_attempts=7 recovery_commits=6 recovery_rollbacks=1 injected_rearm_aborts=1 fail_closed_retries=1 driver_timeouts=6 driver_resets=7 async_starts=7 physical_completions=7 async_steps=28 recovery_yields=21 timer_progress_windows=7 worker_progress_windows=7 el0_progress_windows=7 acquire_waits=21 acquire_dispatch_changes=21 masked_poll_iterations=0 max_step_masked_ticks=39750 max_control_masked_ticks=31500 timer_period_ticks=625000 long_daif_masks=0 final_epoch=7 final_generation=1 requests=9625 completions=9619 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1
BOOT_OK: M58 cooperative fail-stop StorageServer recovery verified
```

> 历史 M59 的第一本隔离账是 ABI-v24 `app-data-async-recovery-runtime`：只抑制一次 read QueueNotify，在恢复前冻结原操作结果，read deadline 映射为 `RequiresReset`，kernel 不重放原操作；userspace 只允许 `FileOpenAt` 在零输出 `Unavailable` 后恰好重试一次。mutation 仍为 `OutcomeUnknown`，没有动态 mutation fault case，也没有 mutation replay/盲重试。物理恢复为 4 step/3 Pending，outer coordinator yield 5 次；完成后 timer、双 worker 与 authenticated owner 均取得进度，再在 IRQ mask 下 rearm、显式开 admission 并 commit。精确历史账本为：

```text
APPDATA_ASYNC_RECOVERY_OK abi=24 cases=1 fault_order=R read_requires_reset=1 injected_reads=1 unavailable_read_retries=1 blind_mutation_retries=0 recovery_starts=1 physical_completions=1 recovery_commits=1 recovery_failures=0 async_steps=4 physical_pending_returns=3 coordinator_yields=5 step_completion_relation=1 timer_progress_windows=1 worker_progress_windows=1 el0_progress_windows=1 authenticated_waits=2 wait_dispatch_changes=2 driver_timeouts=1 driver_resets=1 masked_poll_iterations=0 max_step_masked_ticks=53500 max_control_masked_ticks=86187 timer_period_ticks=625000 long_daif_masks=0 final_active=0 gate_open=1 invariant_errors=0
APPDATA_RUNTIME_OK abi=24 phase=created version=1 boot_generation=0 committed_generation=2 entries=2 files=1 directories=1 submissions=11 completions=11 retrievals=11 mutations=2 reads=1 lists=3 conflicts=1 expected_terminal=4 disk_reads=6682 disk_writes=580 disk_flushes=4 old_or_new=1 full_readback=1 resident=1 errors=0 async_recovery=1
BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified
```

> 历史 M59 的第二本隔离账是 ABI-v23 `storage-irq-timeout-self-test`，不是产品 runtime leaf。它的一次物理恢复为 4 step/3 yield；3/3 timer/worker progress windows 内共 9 次 timer dispatch，双 worker work 为 `539135/376383`。它只在 rearm 后显式开 admission，保留历史 M22 race marker，并明确 `el0_progress_claim=0`：

```text
STORAGE_IRQ_COOPERATIVE_RECOVERY_OK starts=1 physical=1 steps=4 yields=3 step_yield_relation=1 timer_progress_windows=3 worker_progress_windows=3 timer_dispatches=9 worker0_work=539135 worker1_work=376383 masked_poll_iterations=0 max_step_masked_ticks=59375 max_control_masked_ticks=87188 timer_period_ticks=625000 long_daif_masks=0 final_active=0 gate_open=1 el0_progress_claim=0
STORAGE_IRQ_RACE_OK timeout_requests=2 timeouts=1 resets=1 reset_tokens_invalidated=2 simulated_late_irq=1 spurious_acked=1 recovered_requests=2 recovered_completions=2 double_completions=0 dma_frames_before=2 dma_frames_after=2 digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c
BOOT_OK: M32 transferable graphics buffers, M22 storage IRQ timeout/reset self-test, and M20 multi-session services verified
```

> 历史 M60 在 M58 cooperative StorageServer 上增加 ticketed `RecoveryPolicy`：一次 `WRFWRFR` campaign 包含六个历史瞬态 W/R/F 控制和一个模拟永久 read fault；attempt cap=3，退避基数为 2 ticks、倍率 2，恢复成功先进入 Probation，达到健康窗后才回到 Healthy，失败上限后进入本次 boot 内粘滞 Offline。永久 fault 不是 EL0 授权：六个瞬态 EL0 控制仍保留，但 epoch 7 的 EL0 只选择普通 read workload；kernel 在 ownerless 提交窗中 prearm，因此 `el0_permanent_fault_arm_controls=0`、`permanent_authority=kernel-prearmed`。StorageConnect 先完成 principal/image/PID/rights 鉴权，再报告 recovery/Offline 状态，未认证调用者不能借状态分支绕过身份检查。

> 动态 campaign 精确得到 recovery attempts/commits/failures/rollbacks=`9/6/3/1`、physical successes/failures=`7/2`、policy attempt failures=`4`（含一次 Probation I/O failure）、三次完成的 backoff 共 8 ticks、Probation/Healthy=`7/5/2` 与 5 次 Healthy transition。终态路径在进入 DAIF-masked 窗前已关闭 admission；masked 窗内完成 IRQ rollback，复核 admission 仍关闭、coordinator inactive、driver status=0/in-flight=0 与 terminal DMA ownership，随后发布 broker Offline。历史 M60 动态运行走 direct terminal proof，fallback cooperative quarantine 仅有静态封印，计数为 `0/0/0`。Offline 后 attempt/reset/submission delta 均为 0，下一 owner epoch 保留为 8。精确实测为：

```text
STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6 permanent_cases=1 fault_order=WRFWRFR read_requires_reset=3 mutation_outcome_unknown=4 transient_control_sequences=6 kernel_permanent_owner_requests=1 kernel_permanent_fault_arms=1 kernel_permanent_reads=1 el0_permanent_fault_arm_controls=0 permanent_epoch=7 permanent_authority=kernel-prearmed injected_reads=3 injected_writes=2 injected_flushes=2 owner_exits=7 offline_probe_exits=1 broker_releases=7 broker_abandoned=7 recovery_attempts=9 recovery_commits=6 recovery_failures=3 recovery_rollbacks=1 fail_closed_retries=1 physical_successes=7 physical_failures=2 attempt_failures=4 probation_io_failures=1 attempt_cap=3 backoffs=3 backoffs_completed=3 backoff_base_ticks=2 backoff_multiplier=2 backoff_ticks=8 probation=7/5/2 healthy_transitions=5 device_offline=1 offline_transitions=1 offline_denials=1 early_rejections=0 early_physical_starts=0 offline_rejections=0 stale_ticket_rejections=0 invalid_transition_rejections=0 generation_exhaustions=0 persistent_fault_armed=1 persistent_fault_hits=2 driver_timeouts=7 driver_resets=10 async_starts=9 physical_completions=7 async_physical_failures=2 async_steps=35 recovery_yields=26 timer_progress_windows=9 worker_progress_windows=9 el0_progress_windows=9 backoff_timer_progress_windows=3 backoff_worker_progress_windows=3 acquire_waits=26 acquire_dispatch_changes=26 terminal_quarantine_starts=0 terminal_quarantine_completions=0 terminal_quarantine_physical_errors=0 terminal_irq_rollbacks=1 terminal_dma_verifications=1 terminal_driver_state=2 terminal_transport_status=0 terminal_in_flight=0 masked_poll_iterations=0 max_step_masked_ticks=44188 max_control_masked_ticks=34813 timer_period_ticks=625000 long_daif_masks=0 offline_quiet_ticks=16 post_offline_attempt_delta=0 post_offline_reset_delta=0 post_offline_submission_delta=0 final_epoch=0 next_epoch=8 final_generation=1 final_irq_armed=0 final_irq_failed=1 final_recovery_required=1 final_admission_open=0 final_broker_state=4 final_broker_bound=0 final_broker_pending=0 requests=9023 completions=9016 simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1
BOOT_OK: M60 bounded StorageServer fault policy and boot-local Offline state verified
```

> 历史 M61 是 M60 的 fault-latched owner-liveness child，而不是健康服务的通用 heartbeat。每个 session-fatal completion 都在同一个 DAIF-masked completion publication window 内原子启动一张不可续期的 250 ms physical-counter ticket；ticket 精确绑定 generation-qualified PID、broker epoch 与 lease generation，EL0 没有 arm/renew 控制。七张 ticket 中六个 owner 在宽限内合作退出；epoch 6 的 flush owner 收到 `OutcomeUnknown` 后尝试关闭 live `StorageVolume`，按 M61 process-lifetime contract 得到 `InvalidState`，随后阻塞在真实单目标 `ObjectWait`。历史 M60 的 live-volume `HandleClose` release-before-commit 路径只保留在 `not M61` profile，不能外推到 M61 账本。

> 250 ms deadline 到达后，kernel monitor 只对 ticket 中的精确 owner 标记 `Killed`，消费其真实 wait token；它不冒充 init，也没有 EL0 `ProcessTerminate`。随后仍由普通 process reaper 释放 broker、关闭 accepted session handles 并销毁 address space；recovery barrier 要等精确 process table entry 消失才解除，不能把 broker unbound 当作完成。之后 epoch 7 replacement 恢复并继续继承 M60 的 permanent-read/Offline campaign。当前样本 `acquire_waits/acquire_dispatch_changes=26/26`；checker 接受的调度样本集合是二者相等且各为 25 或 26。运行日志 SHA-256 为 `7fe87bdeb58b981ac38167f6976be30002ac5cd28e4f44ce1698e6d44f1499e4`，精确终态为：

```text
STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6 permanent_cases=1 fault_order=WRFWRFR read_requires_reset=3 mutation_outcome_unknown=4 transient_control_sequences=6 kernel_permanent_owner_requests=1 kernel_permanent_fault_arms=1 kernel_permanent_reads=1 el0_permanent_fault_arm_controls=0 permanent_epoch=7 permanent_authority=kernel-prearmed injected_reads=3 injected_writes=2 injected_flushes=2 owner_exits=7 offline_probe_exits=1 broker_releases=7 broker_abandoned=7 recovery_attempts=9 recovery_commits=6 recovery_failures=3 recovery_rollbacks=1 fail_closed_retries=1 physical_successes=7 physical_failures=2 attempt_failures=4 probation_io_failures=1 attempt_cap=3 backoffs=3 backoffs_completed=3 backoff_base_ticks=2 backoff_multiplier=2 backoff_ticks=8 probation=7/5/2 healthy_transitions=5 device_offline=1 offline_transitions=1 offline_denials=1 early_rejections=1 early_physical_starts=0 offline_rejections=0 stale_ticket_rejections=0 invalid_transition_rejections=0 generation_exhaustions=0 persistent_fault_armed=1 persistent_fault_hits=2 driver_timeouts=7 driver_resets=10 async_starts=9 physical_completions=7 async_physical_failures=2 async_steps=35 recovery_yields=26 timer_progress_windows=9 worker_progress_windows=9 el0_progress_windows=9 backoff_timer_progress_windows=3 backoff_worker_progress_windows=3 acquire_waits=26 acquire_dispatch_changes=26 terminal_quarantine_starts=0 terminal_quarantine_completions=0 terminal_quarantine_physical_errors=0 terminal_irq_rollbacks=1 terminal_dma_verifications=1 terminal_driver_state=2 terminal_transport_status=0 terminal_in_flight=0 masked_poll_iterations=0 max_step_masked_ticks=38500 max_control_masked_ticks=36563 timer_period_ticks=625000 long_daif_masks=0 offline_quiet_ticks=16 post_offline_attempt_delta=0 post_offline_reset_delta=0 post_offline_submission_delta=0 final_epoch=0 next_epoch=8 final_generation=1 final_irq_armed=0 final_irq_failed=1 final_recovery_required=1 final_admission_open=0 final_broker_state=4 final_broker_bound=0 final_broker_pending=0 requests=9023 completions=9016 simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0
STORAGE_SERVER_OWNER_LIVENESS_OK authority=kernel-fatal-completion ticket=pid-epoch-lease-generation deadline=physical-counter grace_ms=250 grace_counter_units=15625000 arms=7 cooperative_retirements=6 deadlines_expired=1 early_expirations=0 termination_requests=1 already_terminal=0 forced_retirements=1 forced_reaps=1 terminal_race_retirements=0 stalled_epoch=6 stalled_operation=flush stalled_completion=outcome-unknown live_volume_close=denied target_wait=single object_wait_abandoned=1 el0_process_terminate_calls=0 terminated_exited=7 terminated_killed=1 recovery_after_forced_retirement=1 replacement_epoch=7 final_phase=0 final_owner_pid=0 final_broker_epoch=0 next_lease_generation=8 el0_arm_controls=0 el0_renew_controls=0 simulated_fault=1 hardware_claim=0 smp_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250
BOOT_OK: M61 fault-latched StorageServer owner retirement and recovery convergence verified
```

> 历史 M62 不改变 ABI，而是在最终物理失败处由 kernel-only gate 恰好一次让 direct terminal proof 不可用，只允许一张不可续期 proof deferral。fallback 以三步 cooperative executor 前进、返回两次 `Pending`、记录一次模拟 physical error，再次完成 IRQ rollback 与 DMA/transport 复核后才发布 boot-local Offline；policy attempt/ticket delta 与 EL0 controls 都为 0。release log SHA-256 为 `c283849e9f7836140f5c1f40349f64cc5248ed51d9dbb2a9b9bca70faba4281d`：

```text
STORAGE_SERVER_TERMINAL_QUARANTINE_OK authority=kernel-offline-transition trigger=simulated-proof-unavailable gate=nonrenewable gate_phase=2 proof_arms=1 proof_deferrals=1 unsafe_observations=0 proof_publications=1 fallback_starts=1 fallback_steps=3 fallback_pending_returns=2 fallback_completions=0 fallback_physical_errors=1 fallback_timer_progress_windows=1 fallback_worker_progress_windows=1 persistent_fault_hits=3 terminal_irq_rollbacks=2 terminal_dma_verifications=1 final_driver_state=2 final_transport_status=0 final_in_flight=0 final_recovery_active=0 final_requests_terminal=1 policy_attempt_delta=0 policy_ticket_consumed=0 el0_controls=0 simulated_fault=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 smp_claim=0 general_runtime=0 invariant_errors=0
STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250 terminal_quarantine=1 proof_deferral=1
BOOT_OK: M62 terminal StorageServer quarantine fallback and reverified Offline boundary verified
```

> 历史 M63 在既有 M25 `BNDROID_DATA` 双槽 transaction 中持久化 80-byte `BNDRHLT1` v1 payload：boot generation、未闭合启动 hint，以及 capacity/features/MMIO/IRQ/sector/queue/read-only/FLUSH 的非秘密设备契约摘要与 witness。首启从 legacy 32-byte payload 升级并提交 generation 1/slot 1；同一镜像第二次启动读到 `prior_boot_open=1`，只设置 `reprobe_required=1`，用当前 live transport 重新验证 writable+FLUSH、negotiated features、IRQ/recovery、current read completion、status 与零 reset/timeout/config event，再写 inactive slot、flush、完整 readback 并提交 generation 2/slot 0。record 不能发布/恢复 Offline、清 recovery、开 admission 或授予 EL0 权限；default M25 transaction 会保留 M63 payload。release log 为 97 行/24819 bytes，SHA-256 `6e7d023540a90ecb6901c9817189c2ddefc0e21baffb28608dfcc066e41fd06b`：

```text
STORAGE_DEVICE_HEALTH_REBOOT_OK boots=2 legacy_upgrades=1 unclosed_hints=1 reprobe_required=1 reprobe_verified=2 contract_changes=0 final_generation=2 final_slot=0 prior_slot=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 changed_data_bytes=154 offline_persisted=0 offline_from_record=0 el0_controls=0 hardware_identity_claim=0 hotplug_claim=0 powercut_claim=0 tamper_resistance_claim=0 general_runtime=0
BOOT_OK: M63 persistent unclosed-boot hint and fresh kernel reprobe boundary verified
```

> 历史 M64 在 M63 上增加 exact-session clean close 与 no-later-storage 边界。kernel 在 fresh live probe 后把刚打开 health session 的 generation/epoch 作为仅驻内存能力保存；只允许该精确 generation 在 live contract 仍匹配、device healthy/idle、无 in-flight request 时向 inactive slot 写 `boot_open=false`，随后 flush、完整 readback 并确认旧 slot 未变。成功后消费该能力，stale generation、changed contract、replay、torn write、flush failure 与 generation exhaustion 都在写入前或严格事务边界 fail-closed。

> M64 directed runtime 在任何 EL0 进程启动前完成 close，随后屏蔽本地 IRQ、封闭 storage admission、禁用 logical/physical block IRQ、复核所有 request terminal，并证明没有后续 storage mutation 后 halt。同一镜像第二次启动从 prior closed 记录重新 open、fresh probe，再 close 到 generation 4；`boot_open=false` 只移除未闭合 hint，绝不替代 live probe。它没有 EL0 shutdown control，不持久化/恢复 Offline，也不声称完整 userspace shutdown、PSCI/硬件 poweroff、真实 power-cut、SMP 或 general runtime。当前 log 为 83 行、15855 字节，SHA-256 为 `a1d620dc22bae3853c1ac02ea40e24a65a6c3e7ac2930ab53cccaa379564c113`：

```text
STORAGE_CLEAN_SHUTDOWN_REBOOT_OK boots=2 legacy_upgrades=1 clean_closes=2 prior_closed=1 unclosed_hints=0 reprobe_required=0 reprobe_verified=2 contract_changes=0 final_generation=4 final_slot=0 prior_generation=3 prior_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_unchanged=1 appdata_unchanged=1 unused_data_unchanged=1 changed_data_bytes=153 offline_persisted=0 offline_from_record=0 el0_started=0 el0_controls=0 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M64 kernel-owned clean boot-session close and no-later-storage boundary verified
```

> 历史 M65 以 ABI v26/syscall 53 增加 init-only 两阶段 shutdown。两个固定 workload client 完成、退出并被回收，StorageServer idle、broker 与物理 I/O 全部 terminal 后，`Prepare` 才原子关闭新进程及 StorageAcquire/Connect/Accept admission；两个 client 的 shutdown 调用均被拒绝，Prepare 后的一次 `ProcessSpawn` 也被拒绝。已认证 StorageServer 的 Submit/Take 保留到终止工作结束。

> init 发送带 AppData generation 的 shutdown command；StorageServer 同时等待 control/volume，执行最终 flush 和同 generation 全量 recover/readback，ACK 后正常退出并由 init `ProcessWait` 回收。精确 `InitReady` 后才允许 `Commit`。kernel monitor 再验证 process/broker/I/O/IRQ/DMA/syscall 账本，调用 M64 authenticated durable close，封闭 storage admission、block IRQ 与 shutdown gate 后 halt。权限仅授予 init，不是任意 EL0 control。

> 同一 writable image 两次启动证明 AppData generation 5→6、health session `1→2` 与 `3→4` 均 clean close、两次 StorageServer flush/readback/normal exit，以及 DATA/APPDATA 之外字节不变。当前 log 为 91 行、18869 字节，SHA-256 为 `2c338c42a97a869375ccca50ec33cd8d84556b71cbcf7f56bffe8c44be58c645`：

```text
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_REBOOT_OK boots=2 userspace_shutdowns=2 clients_drained=4 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 offline_persisted=0 offline_from_record=0 el0_started=1 el0_controls=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M65 userspace StorageServer shutdown orchestration and durable close verified
```

> M65 仍只是 single-core storage-profile directed proof：没有完整 resident UI/service graph quiesce，没有 PSCI/硬件 poweroff、真实断电、SMP 或 general runtime 结论。

> 历史 M66 在 M65 上加入 ABI v27 与 child-only syscall 54 `ServiceShutdown`。init 同时启动 StorageServer 与八个非存储 resident 节点：ServiceManager、Provider、两个不同身份的 Client、SurfaceServer、InputServer、Launcher、App。Prepare 前 kernel 直接核验 10 个 live process、9 对 init control、10 对真实 dependency Channel、38 个唯一 endpoint、39 个 handle、精确 rights/空队列、唯一 StorageVolume，以及八个 PID/image/node 绑定；userspace 自报计数不能替代 object-table proof。

> 全图登记后 Prepare 才关闭 admission。一次提前 SurfaceServer quiesce 必须被拒；随后按 `Primary/Secondary/Launcher/App→Provider/InputServer→ServiceManager/SurfaceServer` 三个逆拓扑 wave 接受。Launcher/App 执行真实 AppData workload 并验证 Prepare 后 StorageConnect 拒绝。八个节点全部退出回收后，StorageServer 才最终 flush/readback/退出回收；随后 InitReady/Commit、M64 durable close、storage/shutdown seal、block IRQ disable 与本地 IRQ mask 完成，kernel 才创建不可伪造的 `ValidatedShutdown` token。

> M66 唯一 backend 是 AArch64 QEMU semihosting `SYS_EXIT_EXTENDED`。同一 writable image 连续两次都必须由 QEMU 自身以 status 0 退出，host kill 不算成功。最终 DATA/APPDATA 外字节不变、AppData 改变、health generation 4 closed。该历史 log 为 89 行、19200 字节，SHA-256 为 `656186fb9e4275bed2f64d95484160e97cdbe7960a74f90e209f566b7aff81de`：

```text
RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed
```

> 这只是 fixed-graph、single-core、emulator-only shutdown proof；不是完整产品 UI runtime，不是 PSCI/PMIC/硬件 poweroff，不证明真实掉电、SMP、general runtime 或真机。

> 历史 M67 先让真实 M45 UI/InputServer 路径收敛，再把 QMP `qcode=power` 作为物理键码 116 经认证 InputServer 路径交给 init；Launcher/App 执行 AppData，final StorageServer 启动后进入历史 M66 closure。M67 专用边界是 process capacity `10/9`、64 KiB worker exception stack 与 256-page/1 MiB heap；旧 profile 不回写。两次 release 启动均要求 status-0 QEMU 自退出和同一最终截图 SHA-256 `1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994`：

```text
UNIFIED_PRODUCT_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
```

> 历史 M68 在上述原路径中启动第一代 StorageServer，以 BSH1 probe 1 验证 Healthy；probe 2 被服务端故意收取但不回复。init 的真实 100 ms 有限 `ObjectWait` 返回 `HealthTimeout`，真实 30 ms backoff 完成后只消耗一个 restart budget，并通过一次 `ProcessTerminate`/`ProcessWait` 得到 `Killed`。replacement PID 与旧 PID 同 slot、generation 恰好 +1，重新 mount 后 AppData generation 不变，probe 1 再次 Healthy；随后完整 M67 UI/AppData/shutdown/self-exit 路径继续。两次同盘 release 启动都复现该有界恢复并保持 M67 的 exact 截图：

```text
UNIFIED_PRODUCT_LIVENESS_OK format=1 abi=29 authority=init-bsh1-plus-kernel-authenticated-pid service=StorageServer protocol=BSH1 probes=3 healthy=2 withheld=1 health_timeouts=1 health_timeout_ms=100 backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 old_pid=4294967306 replacement_pid=8589934602 same_slot=1 generation_step=1 process_terminate_calls=1 process_terminate_successes=1 terminated_killed=1 storage_epochs=2 next_epoch=3 releases=2 replacement_mounted=1 replacement_healthy=1 bounded=1 single_service=1 injected_hang=1 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_LIVENESS_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 storage_liveness_recoveries=2 health_probes=6 healthy=4 health_timeouts=2 backoff_waits=2 killed_servers=2 replacements=2 replacement_healthy=2 storage_epochs=4 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
```

> M68 仍是 bounded single-core QEMU 研究原型，只证明一个 StorageServer、一次故意 hang、一次重启；没有周期性长期 watchdog、多服务依赖恢复、并发故障或任意 soak。它不是真手机、不可刷机、不可日用；QMP power key 与 semihosting 均不是硬件 poweroff，`general_runtime=0 real_phone_claim=0`。

> 历史 M69 在完整 M68 路径中监督固定 StorageServer+App 与一条 hard `StorageServer -> App` 边。每次启动执行 BSH1 probe/Healthy/withheld=`8/7/1`、三次真实 40 ms cadence、一次真实 100 ms timeout 与 30 ms backoff；App 对 `HardBlocked` 和恢复各显式 ACK，一次同槽下一代 StorageServer replacement Healthy 后，最终双服务 Healthy 才允许 AppData 继续。两次同盘 release 启动均复现该有界依赖恢复：

```text
UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_OK format=1 abi=30 authority=init-bsh1-plus-kernel-authenticated-pid services=2 service_set=StorageServer+App protocol=BSH1 dependency=StorageServer->App dependency_kind=hard probes=8 healthy=7 withheld=1 cadence_waits=3 cadence_ms=40 periodic_rounds=3 health_timeouts=1 health_timeout_ms=100 backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 old_pid=4294967306 replacement_pid=8589934602 app_pid=4294967305 same_slot=1 generation_step=1 process_terminate_calls=1 process_terminate_successes=1 terminated_killed=1 storage_epochs=2 next_epoch=3 releases=2 fault_transitions=2 recovery_transitions=2 dependent_blocks=1 dependent_resumes=1 app_health_sequences=3 replacement_health_sequences=2 replacement_mounted=1 replacement_healthy=1 bounded=1 single_dependency=1 injected_hang=1 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 multiservice_liveness_recoveries=2 supervised_services=4 liveness_dependency_edges=2 health_probes=16 healthy=14 withheld=2 health_timeouts=2 cadence_waits=6 dependency_fault_transitions=4 dependency_recovery_transitions=4 dependent_blocks=2 dependent_resumes=2 killed_servers=2 replacements=2 replacement_healthy=2 storage_epochs=4 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
```

> M69 仍是 bounded single-core QEMU 研究原型：固定 2 服务、1 条 hard edge、3 轮、1 次注入 hang 与 1 次 restart；没有任意 DAG、非注入长期 watchdog、并发故障或任意 soak。它不是真手机、不可刷机、不可日用，QMP power key 与 semihosting 均不是硬件 poweroff。

> 历史 M70 在两次同盘 release 启动中都从 QEMU `virt` FDT 唯一发现 `/psci`，以 `PSCI_VERSION` 复核 1.1，并在完整 M69/UI/AppData/关机图收敛后通过 HVC `SYSTEM_OFF` 让 QEMU 自退出。M70 的 QEMU 启动参数不含 semihosting；以下精确标记不能外推为 PMIC、硬件断电或真机证明：

```text
M70_PSCI_DISCOVERY_OK format=1 abi=31 node=/psci compatible=arm,psci-1.0 method=hvc psci_version=1.1 version_raw=0x00010001 version_probe=PSCI_VERSION version_function_id=0x84000000 system_off_function_id=0x84000008 fdt_validated=1 installed_once=1 semihosting=0 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0
UNIFIED_PRODUCT_PSCI_SHUTDOWN_OK format=1 abi=31 authority=fdt-plus-psci-version-plus-kernel-seal node=/psci compatible=arm,psci-1.0 method=hvc psci_version=1.1 version_raw=0x00010001 version_probe=PSCI_VERSION version_function_id=0x84000000 system_off_function_id=0x84000008 backend=qemu-psci system_off_requested=1 host_self_exit_verified=0 semihosting=0 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0
UNIFIED_PRODUCT_PSCI_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 psci_discoveries=2 psci_version_probes=2 psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 psci_system_off_requests=2 qemu_psci_self_exits=2 semihosting_uses=0 multiservice_liveness_recoveries=2 supervised_services=4 liveness_dependency_edges=2 health_probes=16 healthy=14 withheld=2 health_timeouts=2 cadence_waits=6 dependency_fault_transitions=4 dependency_recovery_transitions=4 dependent_blocks=2 dependent_resumes=2 killed_servers=2 replacements=2 replacement_healthy=2 storage_epochs=4 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_PSCI_SHUTDOWN_SOURCE_OK abi=31 fdt_node=/psci compatible=arm,psci-1.0 method=hvc psci_version=1.1 version_raw=0x00010001 version_function_id=0x84000000 system_off_function_id=0x84000008 fdt_negative_classes=7 contract_negative_classes=4 parser_serial_negative=13 parser_host_negative=3 qemu_launches=52 nic_none=52 semihosting_m70=0 no_new_syscall=1 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0
UNIFIED_PRODUCT_PSCI_SHUTDOWN_EVIDENCE_PARSER_SELF_TEST_OK positive=4 serial_negative=13 host_negative=3
UNIFIED_PRODUCT_PSCI_SHUTDOWN_STATIC_OK source=1 feature_closure=1 abi=31 fdt_strict=1 psci_version_probe=1 system_off=1 opaque_seal=1 two_boot=1 parser_serial_negative=13 parser_host_negative=3 no_new_syscall=1 semihosting=0 full_suite=1 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 real_phone_claim=0
BOOT_OK: M70 FDT-validated QEMU PSCI SYSTEM_OFF, two-service liveness, AppData, and bounded shutdown armed
```

> 历史 M70 仍是 bounded single-core QEMU 研究原型。它没有 PMIC、BSP/真实板卡验证、硬件 poweroff、SMP/IOMMU、网络、蜂窝/电话、Wi-Fi、音频或产品级安全/更新闭环，不是真手机、不可刷机、不可日用。

> 历史 M71 在历史 M70 closure 上增加事务式五服务目录与批量监督。每次启动的目录固定容量为 5 services/4 edges（3 hard、1 soft），目录和 probe batch 都保持 all-or-none；ServiceManager、SurfaceServer、InputServer、StorageServer、App 各自维护序列和 missed-probe 计数。21 个真实 cadence 轮中有 16 个额外健康 soak 轮；SurfaceServer 与 InputServer 在同一窗口各漏一次，在 tolerance=1 内不传播依赖故障并于下一轮独立恢复。StorageServer 的连续漏报超过 tolerance 后仍触发 M69 的 timeout/replacement/App block-resume 路径。每次启动的精确 bounded 证据为：

```text
UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_OK format=1 abi=32 authority=init-bsh1-catalog-plus-kernel-authenticated-pid services=5 service_set=ServiceManager+SurfaceServer+InputServer+StorageServer+App protocol=BSH1 catalog_transactional=1 catalog_capacity=5/4 dependency_edges=4 hard_edges=3 soft_edges=1 probes=107 healthy=104 missed=3 cadence_waits=21 cadence_ms=40 periodic_rounds=21 fully_healthy_rounds=20 healthy_soak_rounds=16 batch_rounds=18 batched_probes=90 health_timeouts=3 health_timeout_ms=100 concurrent_miss_windows=1 concurrent_miss_services=2 missed_probe_tolerance=1 transient_miss_recoveries=2 escalated_faults=1 backoff_waits=1 backoff_ms=30 restart_budget=1 restarts=1 old_pid=4294967306 replacement_pid=8589934602 app_pid=4294967305 same_slot=1 generation_step=1 process_terminate_calls=1 process_terminate_successes=1 terminated_killed=1 storage_epochs=2 next_epoch=3 releases=2 fault_transitions=2 recovery_transitions=2 tolerated_miss_dependency_transitions=0 dependent_blocks=1 dependent_resumes=1 resident_health_sequences=21 app_health_sequences=21 replacement_health_sequences=20 replacement_mounted=1 replacement_healthy=1 elapsed_supervision_ms=1070 bounded=1 injected_storage_hang=1 injected_transient_misses=1 arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_REBOOT_OK boots=2 qemu_self_exits=2 authenticated_power_keys=2 ui_interactions=2 ui_screenshots=2 ui_sha256=1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994 product_shutdowns=2 continuous_supervision_recoveries=2 supervised_services=10 liveness_dependency_edges=8 health_probes=214 healthy=208 missed=6 health_timeouts=6 cadence_waits=42 periodic_rounds=42 fully_healthy_rounds=40 healthy_soak_rounds=32 batch_rounds=36 batched_probes=180 concurrent_miss_windows=2 concurrent_miss_services=4 transient_miss_recoveries=4 escalated_faults=2 dependency_fault_transitions=4 dependency_recovery_transitions=4 dependent_blocks=2 dependent_resumes=2 killed_servers=2 replacements=2 replacement_healthy=2 storage_epochs=4 psci_discoveries=2 psci_version_probes=2 psci_version_1_1=2 fdt_psci_nodes=2 hvc_conduits=2 psci_system_off_requests=2 qemu_psci_self_exits=2 semihosting_uses=0 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 hardware_poweroff_claim=0 pmic_claim=0 psci_claim=1 powercut_claim=0 smp_claim=0 general_runtime=0 real_phone_claim=0
```

```text
UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_SOURCE_OK abi=32 services=5 dependency_edges=4 healthy_soak_rounds=16 batch_rounds=18 concurrent_miss_services=2 transient_miss_recoveries=2 parser_serial_negative=18 parser_host_negative=3 qemu_launches=52 nic_none=52 no_new_syscall=1 semihosting_m71=0 bounded=1 arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_EVIDENCE_PARSER_SELF_TEST_OK positive=4 serial_negative=18 host_negative=3
UNIFIED_PRODUCT_CONTINUOUS_SUPERVISION_STATIC_OK source=1 feature_closure=1 abi=32 services=5 dependency_edges=4 healthy_soak_rounds=16 batch_rounds=18 concurrent_miss_services=2 transient_miss_recoveries=2 parser_serial_negative=18 parser_host_negative=3 no_new_syscall=1 semihosting=0 full_suite=1 bounded=1 arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0
```

> M71 只证明两次各 1070 ms、合计 2.14 秒的有界注入式监督窗口；16 轮额外健康 soak 不是任意时长 soak，也不证明非注入长期可靠性、一般运行时、真实硬件或手机。边界保持 `bounded=1 arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`。

> 完整回归还将 backoff witness 固定为“计时完成且 timer、两个无关 worker 都取得进度后才发下一 attempt”；M61/M62 只有一次不可续期 early-rejection probe，attempt cap、ticket、身份检查与 EL0 权限均未放宽。

> M56 `7813/7810`、M57/M58 `9625/9619`、M60/M61/M62 `9023/9016` 与所有 masked-tick 最大值都是构建样本；稳定请求差分别为 `+3`、`+6` 与 `+7`。历史 M71 入口为 `CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-continuous-supervision-runtime.sh` 和 `CARGO_NET_OFFLINE=true ./scripts/check-unified-product-continuous-supervision-static.sh`；M70 及更早 checker 继续作为隔离 regression gate，完整入口为 `CARGO_NET_OFFLINE=true ./scripts/test.sh`。M71 parser 自测为 positive/serial-negative/host-negative=`4/18/3`，持久日志为 `target/m71/unified-product-continuous-supervision-runtime.log`。当前静态封印为：

```bash
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-continuous-supervision-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-unified-product-continuous-supervision-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-liveness-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-unified-product-liveness-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-unified-product-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-resident-platform-shutdown-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-resident-platform-shutdown-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-fault-policy-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-fault-policy-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-owner-liveness-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-owner-liveness-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-terminal-quarantine-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-terminal-quarantine-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-persistent-health-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-persistent-health-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-clean-shutdown-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-clean-shutdown-static.sh
CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-storage-server-shutdown-orchestration-runtime.sh
CARGO_NET_OFFLINE=true ./scripts/check-storage-server-shutdown-orchestration-static.sh
CARGO_NET_OFFLINE=true ./scripts/test.sh
```

```text
STORAGE_SERVER_RECOVERY_STATIC_OK sources=14 qemu_launches=52 nic_none=52 fault_kinds=3 owner_rotation=1 kernel_reset_authority=1
STORAGE_SERVER_REPEATED_RECOVERY_STATIC_OK sources=15 qemu_launches=52 nic_none=52 cycles=2 fault_cases=6 owner_rotations=6 fail_closed_retries=1 kernel_reset_authority=1
STORAGE_SERVER_ASYNC_RECOVERY_STATIC_SOURCE_OK sources=17 qemu_launches=52 nic_none=52 cooperative_phases=4 admission_gate=1 masked_poll_iterations=0
STORAGE_SERVER_ASYNC_RECOVERY_PARSER_OK negative_cases=18
STORAGE_SERVER_ASYNC_RECOVERY_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=18 kernel_reset_authority=1
STORAGE_SERVER_FAULT_POLICY_STATIC_SOURCE_OK sources=15 qemu_launches=52 nic_none=52 ticketed=1 probation=1 attempt_cap=3 backoff_base_ticks=2 backoff_multiplier=2 device_offline=sticky offline_probe=1 commit_order=rearm-open-prearm-broker transient_el0_controls=6 el0_permanent_fault_arm_controls=0 permanent_authority=kernel-prearmed terminal_irq_dma_proof=1
STORAGE_SERVER_FAULT_POLICY_PARSER_OK negative_cases=111
STORAGE_SERVER_FAULT_POLICY_STATIC_OK feature_mismatch_cases=5 parser_negative_cases=111 ticketed=1 probation=1 probation_io_failure=requires-reset attempt_cap=3 backoff=2x2 offline=sticky offline_probe=1 transient_el0_controls=6 el0_permanent_fault_arm_controls=0 permanent_authority=kernel-prearmed terminal_irq_dma_proof=1
STORAGE_SERVER_OWNER_LIVENESS_STATIC_SOURCE_OK sources=15 host_tests=9 feature_chain=M61-M60-M58-M57-M56-M55 pure_policy=1 nonrenewable=1 finish_arm_daif_window=1 ticket=pid-epoch-lease-generation-physical-deadline m61_reaper_release_only=1 historical_handle_close_release=1 recovery_barrier=1 monitor_only=1 live_volume_close=denied epoch6_flush_object_wait=1 init_process_terminate=0 qemu_launches=1 nic_none=1
STORAGE_SERVER_OWNER_LIVENESS_PARSER_OK negative_cases=184 base_fields=97 owner_fields=39 runtime_fields=13
STORAGE_SERVER_OWNER_LIVENESS_STATIC_OK feature_mismatch_cases=6 parser_negative_cases=184 parser_base_fields=97 parser_owner_fields=39 parser_runtime_fields=13 host_tests=9 pure_policy=1 nonrenewable=1 finish_arm_daif_window=1 monitor_only=1 m61_reaper_release_only=1 historical_handle_close_release=1 recovery_barrier=1 live_volume_close=denied epoch6_flush_object_wait=1 init_process_terminate=0 qemu_launches=1 nic_none=1
STORAGE_SERVER_TERMINAL_QUARANTINE_STATIC_SOURCE_OK sources=13 host_tests=8 feature_chain=M62-M61-M60-M58-M57-M56-M55 pure_policy=1 nonrenewable=1 kernel_offline_arm=1 el0_controls=0 fallback_policy_ticket=0 fallback_steps=3 fallback_pending_returns=2 reverified_dma=1 qemu_launches=1 nic_none=1
STORAGE_SERVER_TERMINAL_QUARANTINE_PARSER_OK negative_cases=204 base_fields=97 owner_fields=39 quarantine_fields=34 runtime_fields=15
STORAGE_SERVER_TERMINAL_QUARANTINE_STATIC_OK feature_mismatch_cases=6 parser_negative_cases=204 parser_base_fields=97 parser_owner_fields=39 parser_quarantine_fields=34 parser_runtime_fields=15 host_tests=8 pure_policy=1 nonrenewable=1 kernel_offline_arm=1 el0_controls=0 fallback_policy_ticket=0 fallback_steps=3 fallback_pending_returns=2 reverified_dma=1 qemu_launches=1 nic_none=1
STORAGE_SERVER_PERSISTENT_HEALTH_STATIC_SOURCE_OK sources=14 host_tests=7 feature_chain=M63-M62-M61-M60-M58-M57-M56-M55 kernel_only=1 legacy_upgrade=1 unclosed_hint=1 current_reprobe=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1
STORAGE_SERVER_PERSISTENT_HEALTH_PARSER_OK negative_cases=54 health_fields=36 phases=2
STORAGE_SERVER_PERSISTENT_HEALTH_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=54 parser_health_fields=36 parser_phases=2 host_tests=7 persist_tests=23 kernel_only=1 legacy_upgrade=1 unclosed_hint=1 current_reprobe=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1
STORAGE_SERVER_CLEAN_SHUTDOWN_STATIC_SOURCE_OK sources=15 host_tests=8 feature_chain=M64-M63-M62-M61-M60-M58-M57-M56-M55 kernel_only=1 pre_el0=1 clean_close=1 session_capability=1 admission_seal=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1
STORAGE_SERVER_CLEAN_SHUTDOWN_PARSER_OK negative_cases=93 health_fields=36 close_fields=37 phases=2
STORAGE_SERVER_CLEAN_SHUTDOWN_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=93 parser_health_fields=36 parser_close_fields=37 parser_phases=2 host_tests=8 persist_tests=31 kernel_only=1 pre_el0=1 clean_close=1 session_capability=1 admission_seal=1 offline_persisted=0 offline_from_record=0 el0_controls=0 qemu_launches=1 qemu_boots=2 nic_none=1
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_STATIC_SOURCE_OK sources=16 host_tests=4 feature_chain=M65-M64-M63-M62-M61-M60-M58-M57-M56-M55 abi=26 init_only=1 prepare_commit=1 generation_bound=1 process_gate=1 storage_gate=1 server_flush=1 server_readback=1 durable_close=1 admission_seal=1 el0_controls=1 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 qemu_launches=1 qemu_boots=2 nic_none=1
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_PARSER_OK negative_cases=133 health_fields=36 shutdown_fields=52 phases=2
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=133 parser_health_fields=36 parser_shutdown_fields=52 parser_phases=2 host_tests=4 abi_tests=1 abi=26 init_only=1 prepare_commit=1 generation_bound=1 process_gate=1 storage_gate=1 server_flush=1 server_readback=1 durable_close=1 admission_seal=1 el0_controls=1 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0 qemu_launches=1 qemu_boots=2 nic_none=1
RESIDENT_PLATFORM_SHUTDOWN_STATIC_SOURCE_OK sources=21 feature_chain=M66-M65-M64-M63-M62-M61-M60-M58-M57-M56-M55 abi=27 resident_nodes=8 dependency_edges=10 quiesce_waves=3 authenticated_nodes=1 kernel_topology=1 reverse_order=1 durable_close=1 platform_token=opaque fail_closed=1 qemu_backend=semihosting qemu_launches=1 qemu_boots=2 nic_none=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
RESIDENT_PLATFORM_SHUTDOWN_PARSER_OK negative_cases=144 health_fields=36 platform_fields=59 phases=2
RESIDENT_PLATFORM_SHUTDOWN_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=144 parser_health_fields=36 parser_platform_fields=59 parser_phases=2 service_tests=4 platform_tests=2 abi_tests=2 abi=27 resident_nodes=8 dependency_edges=10 quiesce_waves=3 authenticated_nodes=1 kernel_topology=1 reverse_order=1 durable_close=1 platform_token=opaque fail_closed=1 qemu_backend=semihosting qemu_launches=1 qemu_boots=2 nic_none=1 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
UNIFIED_PRODUCT_SOURCE_OK abi=28 ui=m45-real-ui power_key=116 appdata=1 resident_nodes=8 dynamic_capacity=9 heap_pages=256 exception_stack_kib=64 qemu_nic_none=1 real_phone_claim=0
UNIFIED_PRODUCT_EVIDENCE_PARSER_SELF_TEST_OK positive=2 negative=6
UNIFIED_PRODUCT_STATIC_OK source=1 feature_closure=1 abi=28 ui=real-m45 input=physical-qmp power_key=116 appdata=1 shutdown_graph=8 qemu_self_exit=1 parser_negative_cases=6 full_suite=1 emulator_only=1 real_phone_claim=0
UNIFIED_PRODUCT_LIVENESS_SOURCE_OK abi=29 service=StorageServer protocol=BSH1 probes=3 healthy=2 health_timeout_ms=100 backoff_ms=30 replacements=1 same_slot_next_generation=1 process_capacity=10 dynamic_capacity=9 heap_pages=256 exception_stack_kib=64 qemu_nic_none=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK positive=2 negative=8
UNIFIED_PRODUCT_LIVENESS_STATIC_OK source=1 feature_closure=1 abi=29 service=StorageServer protocol=BSH1 finite_timeout=1 bounded_backoff=1 same_slot_next_generation=1 parser_negative_cases=8 full_suite=1 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_SOURCE_OK abi=30 services=2 dependency_edges=1 dependency_kind=hard probes=8 healthy=7 health_timeout_ms=100 cadence_ms=40 backoff_ms=30 replacements=1 qemu_launches=52 nic_none=52 no_new_syscall=1 emulator_only=1 general_runtime=0 real_phone_claim=0
UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_EVIDENCE_PARSER_SELF_TEST_OK positive=2 negative=10
UNIFIED_PRODUCT_MULTISERVICE_LIVENESS_STATIC_OK source=1 feature_closure=1 abi=30 services=2 dependency_edges=1 dependency_kind=hard probes=8 healthy=7 withheld=1 cadence_waits=3 finite_timeout=1 bounded_backoff=1 same_slot_next_generation=1 parser_negative_cases=10 no_new_syscall=1 full_suite=1 emulator_only=1 general_runtime=0 real_phone_claim=0
STORAGE_RECOVERY_UNIFICATION_STATIC_SOURCE_OK sources=10 qemu_launches=52 nic_none=52 shared_engine=1 appdata_coordinator=1 timeout_coordinator=1 m58_coordinator=1 m60_coordinator=1 read_only_retry=1 mutation_retries=0
APPDATA_ASYNC_RECOVERY_PARSER_OK negative_cases=25
STORAGE_IRQ_COOPERATIVE_RECOVERY_PARSER_OK negative_cases=29
STORAGE_RECOVERY_UNIFICATION_STATIC_OK feature_mismatch_cases=5 parser_self_tests=2 shared_engine=1 explicit_gate_commits=5 offline=1
```

> 历史 M71 阶段的 83 个顶层 shell scripts 内共有 52/52 个 QEMU launch，均恰有一个 `-nic none`。M67—M70 的隔离 source/feature/parser/双启动 gate 当时全部保留；M71 新增 continuous-supervision source/feature/parser/双启动 gate。该阶段 `scripts/test.sh` 包含 `unified_product_continuous_supervision_static=1 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6`；M70/M71 的 PSCI 双启动合计 `qemu_psci_self_exits=4`，M66—M71 六份双启动 self-exit 账本合计 `qemu_self_exits=12`。

> 2026-07-27 历史 M71 当时最终源码上的完整离线 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已 exit 0，精确终态为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 app_data_async_recovery=1 storage_server_static=1 storage_server_recovery_static=1 storage_server_repeated_recovery_static=1 storage_server_async_recovery_static=1 storage_server_fault_policy_static=1 storage_server_owner_liveness_static=1 storage_server_terminal_quarantine_static=1 storage_server_persistent_health_static=1 storage_server_clean_shutdown_static=1 storage_server_shutdown_orchestration_static=1 resident_platform_shutdown_static=1 unified_product_static=1 unified_product_liveness_static=1 unified_product_multiservice_liveness_static=1 unified_product_psci_shutdown_static=1 unified_product_continuous_supervision_static=1 storage_recovery_unification_static=1 storage_server_runtime_boots=3 storage_server_recovery=1 storage_server_repeated_recovery=1 storage_server_async_recovery=1 storage_server_fault_policy=1 storage_server_owner_liveness=1 storage_server_terminal_quarantine=1 storage_server_persistent_health_reboot=1 persistent_health_boots=2 storage_server_clean_shutdown_reboot=1 clean_shutdown_boots=2 storage_server_shutdown_orchestration_reboot=1 shutdown_orchestration_boots=2 resident_platform_shutdown_reboot=1 resident_platform_shutdown_boots=2 unified_product_reboot=1 unified_product_boots=2 unified_product_ui_interactions=2 unified_product_liveness_reboot=1 unified_product_liveness_boots=2 unified_product_liveness_recoveries=2 unified_product_multiservice_liveness_reboot=1 unified_product_multiservice_liveness_boots=2 unified_product_multiservice_liveness_recoveries=2 unified_product_psci_shutdown_reboot=1 unified_product_psci_shutdown_boots=2 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6 qemu_psci_self_exits=4 qemu_self_exits=12 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1 storage_irq_cooperative_recovery=1
```

> 同一次完整回归还允许 M51 `GraphPrepared` 后 Launcher/App 两个已认证 rebind ACK 以任一合法串行顺序到达；每角色 exact-once、PID/image/token 与 payload 校验没有放宽。

> 历史 M55 权限与边界保持封印：只有 kernel-authenticated StorageServer 可 `StorageAcquire`；volume/session rights 为 `READ|WRITE|WAIT`，均无 `DUPLICATE|TRANSFER`，client 只能请求非空 READ/WRITE 子集。kernel 只 broker raw sectors、owner/epoch/session/token/completion/cleanup，path namespace 与 AppData volume policy 在 EL0，故 `kernel_namespace_ops=0 namespace_policy=0 block_driver=kernel`。

> block wire 为 strict `SBRQ` v2、exact 4160 bytes（64 header + 4096 data），offset 24 是 `u16 sector_count`；read/write count 1—8，reserved/read payload/write tail 均 canonical zero，flush 只能为 LBA/count/data 全零，并检查 `lba + count <= 1920`。read-ahead 与连续 write 最多 batch 8；batch 不是原子多扇区 transaction，kernel driver 仍逐扇区执行。

> 调度保持 logical 100 Hz，只把 physical timer compare 临时拉近形成 one-shot immediate IRQ，并在正常 exception return 处给 newly-woken StorageServer/client 一次 preference；真实 TrapFrame/TTBR/stack switch 已覆盖。精确账式是 `completion_scans=candidate_scans+no_candidate_scans`、`candidate_scans=preferred_requests+preferred_coalesced`、`preferred_requests=preferred_dispatches+stale+preferred_pending`；ready 时 `stale=0 preferred_pending=0`，并满足 `0<immediate_interrupts<=immediate_requests<=I/O completions`。completion 可能先于 client 进入 `StorageTake` 阻塞态，故扫描时可能没有 candidate；旧的 `preferred_requests==all I/O completions` 是错误不变量。不能把 one-shot 写成提高 logical tick rate。idle StorageServer same-slot replacement/reacquire/rebind 已覆盖，但不等于 in-flight 或 device-reset recovery。

> 历史 M55 同一 AppData 镜像三启动为 `0→4→5→5`：boot 1 的 read batches/write batches/flushes/completions=`1425/289/12/1726`，boot 2=`765/45/2/812`，boot 3=`606/0/0/606`；三次 max batch 8、errors/bounds rejection 0。第三次 AppData write sectors=0，boot 2/3 AppData partition SHA-256 同为 `d105d3eecbeee5e77774c1d37f83e11406e50e020d6abd48c0b4a56f9980b089`。whole-disk hash 会因独立 `BNDROID_DATA` boot counter 每次变化，稳定证据只能是 `BNDROID_APPDATA` partition。

> M55—M70 与 M59 双账本是历史前缀，历史 M71 由隔离 checker 封口。M67 统一真实 UI/InputServer、AppData 与固定 shutdown closure；M68 增加单 StorageServer 有界恢复；M69 增加固定 live App 与一条 hard dependency；M70 把最终 QEMU exit 绑定到 strict FDT `/psci`、PSCI_VERSION 1.1 与 HVC SYSTEM_OFF；M71 再加入事务式五服务/四边目录、批量 probe、16 个额外健康轮、同窗两服务瞬态漏报恢复和一次升级 StorageServer replacement。它仍是 bounded single-core QEMU `arbitrary_soak_claim=0 general_runtime=0 real_phone_claim=0` research prototype，不证明 PMIC/hardware poweroff、运行时任意服务发现、非注入长期 watchdog、背靠背升级故障、hotplug/device replacement、SMP、真实硬件、任意时长 soak、真实 power-cut、tamper resistance 或手机；不是真手机、不可刷机、不可日用。下一硬件 P0 必须先由用户指定并授权目标设备，再做 BSP、启动链、控制器与 PMIC；未授权时不连接、刷写或操作真机。一般 POSIX、网络、电话/蜂窝、Wi-Fi、音频、完整安全/更新与产品 UX 均未完成。

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 storage_server_static=1 storage_server_runtime_boots=3 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M35 让每代 App 把 75-page/307200-byte backing 以 producer RW 映射到 `0x0000000200100000`，SurfaceServer 在不同 ASID/root 中把同一物理页映射为 consumer RO。App 用 EL0 volatile store 直接 raster；Queue 在提交前验证全部 76544 个 canonical XRGB8888 pixel 和 page padding，再以 Arm break-before-make + ASID-scoped TLBI 将 producer 75 个叶页 RW→RO。Acquire 只交付 queued generation；cancel/release 或 mapped present 后以同样纪律恢复 RO→RW。SurfaceServer 每次 Acquire 都从 consumer EL0 VA volatile 读取首/中/末像素，证明真实数据面 alias，而非只比对 metadata。

> M35 七事务/two-generation 专用运行精确完成 `map=4/4 unmap=2/2 queue=4/4 acquire=4/4 explicit_release=2/2 releases=4 mapped_presents=2`，终态 `mappings=2/150 shared_pairs=1 physical_alias=1`，legacy `copy_writes=0/0`；下列 M35 marker 保留为上一里程碑的精确证据。

> M36 让 App 写满并 Queue、SurfaceServer Acquire 后被终止。reaper 以 stopped address-space mapping metadata/pin 为权威，先清除 consumer 的 75 个 leaves 并做 ASID TLBI，再以 BBM+producer ASID TLBI 恢复 RO→RW，随后 generation-safe abandon/release 并唤醒 waiter。普通 consumer 在 frame pending 时 unmap 被拒，仍有 mapping 时 close 也被拒；App 醒来后重写全部 75 页、抽样读取、显式 unmap/close/exit。专用 QEMU 走 Acquired 分支，Queued/Acquired 均有 host test。终态 graphics mapping/page/surface/handle 全为零。

> M37 保留 mapped App 常驻：generation-1 SurfaceServer 在 Acquire 后退出，M36 cleanup 先拆 consumer alias、恢复 producer RW/release/wake；同 slot generation-2 server 原子 reacquire Surface session `1→2`。Launcher/App 更换 peer-closed UI endpoint，App 重写并转移 generation-2 buffer，replacement server 建 consumer RO alias、验证并 present session-2 frame 1。终态 process `10/2/2/8`、29 handle、26 endpoint/13 pair、两个 75-page mapping、Active resident App。

> M38 让 App 创建、映射并向 SurfaceServer 转移两个 307200-byte backing，slot 0 Queue/Acquire、slot 1 保持 Writable；Init 严格先 terminate/wait App，再 terminate/wait SurfaceServer。reaper 拆除 stopped mappings、release acquired orphan，并在最后引用关闭时完整擦除两槽。Launcher 随后复用 generation-3 两槽，逐 byte 证明每个完整 backing 全零；第三个并发 create 返回 `OutOfMemory`，最终 mapping/surface/graphics handle 全为零。

> 经完整矩阵验证的 M42/ABI-v20 封口 default workspace lib suite 为 500（ABI/compositor/ELF/SM/UI/init/kernel=`20/41/19/31/99/0/290`）；另有 M41 feature-gated 20 项与 M42 feature-gated 3 项，因此 unique host total 为 523、unique kernel 为 313。hardened `persistent-window-runtime` dedicated QEMU 连续 3/3、M41 regression、all-feature Clippy 与完整 `./scripts/test.sh` exit 0。最终 suite marker 为 `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
> 历史 M43 完整封口账本为 default workspace 522 项（ABI/compositor/ELF/SM/UI/init/kernel=`21/44/19/31/111/0/296`），另有 `11 ui_trace + 10 window_trace + 3 persistent = 24` 项 feature filter，suite unique host total 为 546；`text-input-runtime` kernel all-tests 为 321。dedicated debug QEMU 连续两次、M42 ABI-v21 regression、默认/全 feature Clippy 与完整 `./scripts/test.sh` 均已通过。精确终态 marker 为 `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。

> M42 让 SurfaceServer 以动态 2—4 source WaitArray 持久服务 surface、control、Launcher 与 App，客户端以单调 command/event tracker 延续 canonical 流量。pointer contact 完整区分 idle、active/captured、reject-until-release 与 quiescent：未 capture 的 phone 外 initial down 会整段拒绝到 release；已 capture 的 move/up 可越界并投递 signed-local 坐标；capture 丢失则同样拒绝到 release。peer-close 会清理 retained layer、协调 capture 并关闭 endpoint，但不会自动重启客户端。

> App 在 slot 1 由 generation 1 销毁后以 generation 2 重建；销毁产生 53760-pixel exposure/output，重建后 full App present，最终 raise 的 PixelLedger 为空且幂等。SurfaceServer 对每次操作核对 PixelLedger，并在 READY 前断言 Launcher=`slot0/gen1`、App=`slot1/gen2/top`、focus=`none/gen4`、无 capture、contact quiescent；八帧输出严格 `ABABABAB`，write generation 为 `1-1/2-2/3-3/4-4`。

```text
WINDOW_SESSION_OK abi=20 protocol=1 policy=userspace persistent=1 capacity=2 live=2 z=launcher-app commands=13/3/6/3/1 events=21/3/6/3/8/1 input=8 raw=14 dropped=6 capture=app-launcher-app signed=4 focus_routes=1-2-3 final_focus=none/4 generation=app1-app2 damage=118784/115456/3328 exposure=53760 hidden=1 retained=1 output=full-buffer frames=8 schedule=0-1-0-1-0-1-0-1 generations=1-1/2-2/3-3/4-4 clock=software-timer release=post-copy opportunities=9 edges=9 acquired=8 presented=8 pending=1 lifecycle=10/0/2/2 supervisor=4/0/2/2 processes=9/1/1/8 handles=31 endpoints=26 buffers=2 map=2/2 queue=8/8 acquire=8/8 releases=8 validates=612352/2449408 mappings=2/150 protects=16 producer=2/rw consumer=0 pool=2/0/0 peaks=1/1/1 per_slot=4/4/4/4/4/4 outside=reject-until-release captured_outside=signed-local waits=8/2/6 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M42 persistent userspace window session, boundary routing, and generation-safe recreation verified
PERSISTENT_WINDOW_OK phase1_raw=13970/12587->136/184 drag_raw=7396/6568->72/96 phase2_raw=13970/12587->136/184 phase3_outside_raw=2055/1369->20/20 captured_local=-84/-124 markers=1/1/1/1/1
```

> M43 把上述 M42 transcript 作为有界前缀并将 ABI 推进到 v21：syscall 0—37 全部保留，只新增 object signal bit 5 `KEY_READY` 与 register-only syscall 38 `SurfaceReadKey`。每个 Surface 的 key FIFO 固定 capacity 32；`KeyInputSample` 只携带 Linux key code、release/press/repeat 状态和单调 sequence。

> App 与文本端使用严格 canonical little-endian 64-byte `BTI1` command（Activate/StateAck/Deactivate）和 64-byte `BTE1` event（Activated/Preedit/Commit/DeleteSurrounding/Rendered/Deactivated）。固定 editor 最多保存 8 个 UTF-8 bytes、一个 Unicode scalar preedit，并只在 UTF-8 byte boundary 上移动 selection。QEMU runtime 只映射 Latin `a`、Enter、Backspace：active `A Enter Backspace A Enter` 产生 10 个 transition；焦点切回 Launcher 后的一对 `A` 作为 2 个 unfocused transition 被丢弃，最终 committed 为 `a`，五次 text-field render 截图为 `target/bndroid-m43-text-field.ppm`。

```text
TEXT_INPUT_OK abi=21 protocol=1 wires=BTI1/BTE1 wire=64 session=1 window=app2 focus=none4-app5-launcher6 messages=19/0 commands=7/1/5/1 events=12/1/2/2/1/5/1 revisions=0-1-2-3-4-5 editor=utf8-8 preedit=1 committed=a keys=12/6/6 active=10/5/5 unfocused=2 fifo=12/12/0/32 raw=24 reports=12 transitions=12 renders=5 field=8/16/96/16 pixels=1536 glyph=64 colors=111118/facc15/f4f4f5 digests=90a60d86afe0de65/6e9b51d9b30b78a5/474faa238cb15725 frames=9-10-11-12-13 output=13 schedule=0-1-0-1-0-1-0-1-0-1-0-1-0 write_generations=7/6 damage=126464/123136/3328 pointer=20/12/8 capture=app-launcher-app-app-launcher clock=14/14/13/13/1 graphics=13/13/13/13 validates=995072/3980288 mappings=2/150 protects=26 per_slot=7/6/7/6/7/6 waits=8/2/6 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M43 focus-scoped hardware keyboard and bounded UTF-8 text-editor slice verified
TEXT_INPUT_QMP_OK phase1=1 phase2=1 persistent=1 app_focus=136/184 keys=a-ret-backspace-a-ret key_pairs=6 transitions=12 active=10/5/5 renders=5 launcher_focus=72/96 unfocused=2/drop field=112/160/96/16 committed_pixels=64 background_pixels=1472 screenshot=target/bndroid-m43-text-field.ppm markers=1/1/1/1/1/5/1/1/1
```

> M43 仍是 capacity 2、solid retained layers 的有界 QEMU 证明；fixed bitmap raster 只绘制 `A`。它不是完整 IME、soft keyboard 或产品 InputServer，也没有 locale/candidate UI、任意 Unicode keyboard、grapheme segmentation、font shaping、物理手机键盘、DMA-BUF/IOMMU 或硬件 vblank/pageflip。


> **M44 历史完整封口**

> M44 保留完整 M43 transcript，ABI 仍为 v21，且不新增 syscall 或进程。新 `no_std` `bndr-input`（19 tests）的 `InputMethodEngine` 位于 SurfaceServer 内，明确 `input_method=in_surface input_server=0`；Compositor 只为它提供唯一、可信、`system/nonfocusable` overlay，不占普通两窗口 capacity，也不能夺取 App focus。

> QEMU tablet 依次触发 `A → Enter → Backspace → A → Enter` 五个 soft-key contact，经 `BTI1`/`BTE1` session 2/3 产生 preedit/commit/delete=`2/2/1` 和最终 `aa`。overlay 随 App→Launcher→App 精确 `show-hide-show`；隐藏时点击旧 `A` 坐标不产文字。扩展严格为 21 messages、8 outputs、frames 14—21，pointer 终态 47、soft keys 5/5、resident waits `8/2/6`。

```text
SOFT_KEYBOARD_OK abi=21 protocol=1 prefix=m43 input_method=in_surface input_server=0 process_delta=0 windows=2 overlay=system/nonfocusable source=tablet hardware_delta=0 fifo_delta=0 contacts=9 soft=5/5 preedit=2 commit=2 delete=1 focus=launcher6-app7-launcher8-app9 visibility=show-hide-show hidden_text=0 sessions=2-3 messages=21/0 commands=8/2/5/1 events=13/2/2/2/1/5/1 state=aa editor=utf8-8 renders=5 outputs=8 frames=14-21 overlay_bounds=0/272/208/96 scanout=56/336/208/96 pixels=19968 pointer=47/27/10/17 graphics=21/21/21/21 validates=1607424/6429696 clock=22/22/21/21/1 write_generations=11/10 per_slot=11/10/11/10/11/10 waits=8/2/6 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M44 touch soft keyboard and focus-preserving text input verified
```

> dedicated QEMU checker 已通过并验证 visible/hidden/final 三张截图：`target/bndroid-m44-soft-keyboard-visible.ppm`、`target/bndroid-m44-soft-keyboard-hidden.ppm`、`target/bndroid-m44-soft-keyboard.ppm`。host 账本为 549 default（ABI/compositor/ELF/input/SM/UI/init/kernel=`21/51/19/19/31/112/0/296`）+ 28 feature filter（`11 ui_trace + 10 window_trace + 3 persistent + 1 text_input + 3 soft_keyboard`）= 577 unique；soft-keyboard kernel 为 324。完整 `./scripts/test.sh` 已 exit 0，精确终态 marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M44 仍不是产品 InputServer 或完整 IME：没有候选栏、locale、grapheme、shaping/font stack、任意 Unicode、多点触控或真实硬件输入；网络、电话/蜂窝、Wi-Fi、音频、电源、安全启动、沙箱、安全更新、产品级驱动与真机 UX 仍缺，绝不能称为现实可用的手机系统。

> **M45 历史完整封口**

> M45 把 ABI 推进到 v22，原样保留 syscall 0—38，复用 `READABLE`/`PEER_CLOSED`，新增 register-only syscall 39 `InputAcquire` 与 40 `InputReadEvent`。只有 live InputServer 可取得唯一、不可复制/转移、rights=`READ|WAIT=0x101` 的 InputCapability。capacity-64 kernel broker 终态为 enqueued/dequeued/pending=`59/59/0`、high-water 1、coalesced 0、sequence `1—59`/next 60、physical=`pointer47+key12`；Surface pointer/key FIFO reads 均为 0。strict canonical `BIC1`/`BIE1` 均为 64 bytes；InputServer 拥有 route/focus/capture/text-context/IME，SurfaceServer 保留 compositor/window/output。

```text
INPUT_SERVER_KERNEL_OK abi=22 pid=4294967302 session=1 capacity=64 enqueued=59 dequeued=59 pending=0 high_water=1 coalesced=0 sequence_next=60 process_capacity=9 dynamic_capacity=8 scheduler_contexts=12 route=broker-only surface_fifo=0
INPUT_SERVER_OK abi=22 protocol=1 wires=BIC1/BIE1 wire=64 owner=unique pid=4294967302 session=1 capacity=64 events=59/59/0 high_water=1 coalesced=0 sequence=1-59 next=60 physical=pointer47+key12 surface_fifo=0/0 legacy_key_reads=0 routes=generation-qualified focus=server capture=server ime=server processes=10/1/1/9 process_capacity=9/8 input_server=1/3 handles=36 endpoints=30 pairs=15/2 waits=9/2/7 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M45 dedicated InputServer routing, capture, and input-method ownership verified
```

> `scripts/check-input-server.sh` 已验证 `target/bndroid-m45-input-server-visible.ppm`、`target/bndroid-m45-input-server-hidden.ppm` 与 `target/bndroid-m45-input-server.ppm`；visible/final SHA-256 分别为 `2b140efbc26f0f48cb7c0b33e0299a2e24d4720bc207fd780490c10741f5fcf3`/`1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994`。host 账本为 599 default（ABI/compositor/ELF/input/SM/UI/init/kernel=`26/51/19/64/31/112/0/296`）+24 directed（`11 ui_trace + 10 window_trace + 3 persistent`）=623 executed/unique，input-server kernel 为 334；完整 suite 终态新增 `input_server=1`。

> M45 是独立有界系统服务，仍非产品级 InputServer/IME：缺 candidate/locale、grapheme、shaping/font、任意 Unicode、多点、物理手机硬件及 restart/backoff/audit/permission integration；它的精确 marker、截图和 623/334 历史账本保留如上。

> **M46 历史完整封口**

> M46 不改变 ABI：仍为 v22，syscall 0—40 全部不变。它是从冻结 M41 checkpoint 独立启动的 leaf，不串接 M42—M45 的物理输入 transcript；确定性 M41 semantic replay 精确为 trace/command/event/output=`23/9/14/6`，该前缀不进入 kernel broker。strict `BIR1` route transcript 为 8 条；`BSR1` restart transcript 为 11 条，六种消息均为 fixed 64 bytes，方向计数 Surface→Init/Init→Surface/Surface→Client/Client→Surface=`4/1/3/3`。Surface pointer/key FIFO=`0/0`，legacy key reads=`0`。

> InputServer 在 route epoch 1 保存 App capture 与 sequence floor 2；旧 Surface session 1 在 App contact established/ack=`1/1` 后于 recovery frame 7 冻结并退出，同一 process slot 的 replacement Surface 以 PID generation+1、session 2 重建。gap 中唯一 release 被排队；route epoch `1→2` 后按 stale release 忽略且不路由，App contact cancel=`1`、终态 `none`。broker enqueued/dequeued/pending/high-water=`3/3/0/1`，gap release=`1`。终态 process created/exited/reaped/live=`11/2/2/9`，handle/endpoint/pair=`36/30/15`，exact waits object/many/array=`9/2/7`。

> 图形恢复严格区分旧 Surface session 1 frame 7 与新 session 2 frame 1，累计 output frame=`8`；pool epoch=`2`、per-slot=`[1,0]`，replacement 重新取得冻结 scanout 并保留可见 released cursor。精确运行 marker 为：

```text
INPUT_SERVER_CAPTURE_CANCEL_OK old_epoch=1 new_epoch=2 target=app cancel=1 stale_release=ignored routed=0 text_delta=0 client_contact=1/1/1 final=none
BOOT_OK: M46 InputServer SurfaceServer restart rebind, route-epoch gap recovery, and capture cancellation verified
```

> `scripts/check-input-server-surface-restart.sh` 已验证 pre/frozen/post 三相恢复；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`。dedicated M46、M45 `scripts/check-input-server.sh` regression 与完整 suite 都从头 exit 0；host 账本为 626 default + 24 directed = 650 executed/unique，M46 kernel 为 335，suite 终态固定 `input_server_surface_restart=1`。

> M46 只证明固定一次会话、单 App capture、一次 SurfaceServer 重启的 bounded boot witness，不是通用恢复或现实可用手机系统；其 ABI-v22 marker、截图 hash 与 650/335 历史账本保留如上。

> **M47 历史完整封口**

> M47 把 ABI 推进到 v23，syscall 0—40 原样保留，新增 register-only syscall 41 `InputSessionInfo(handle, 0, 0)`，返回 InputCapability 的 session 与 acquisition floor。strict canonical 64-byte `BIR1`/`BIP1`/`BIC1`/`BIE1` 用于 replacement bind、Surface recovery phase、route snapshot/ack 与 routed event；InputServer session/route epoch 从 `1/1` 严格过渡到 `2/2`。

> 运行时只执行一次 InputServer 重启：Surface 在 broker unbound gap 中保持存活，Init 以 fixed 30 ms timeout 且 budget `1/1` 后在同 process slot/generation+1 重启，replacement 先 reacquire，再以 snapshot 恢复两条 route、Launcher focus、`capture=none`、`text=none` 和 floor 1，随后 sequence `2—3` 的 App down/up 恢复 capture 并清空。Surface 越权调用 `InputAcquire` 必定得到 `permission_denied`，audit=`1`，handle/session delta 均为 0。broker enqueued/dequeued/pending/high-water=`3/3/0/1`、release=`1`、unbound drop=`0`；终态 process created/exited/reaped/live=`11/2/2/9`、handle/endpoint/pair=`36/30/15`、exact waits object/many/array=`9/2/7`。

```text
INPUT_SERVER_RESTART_PERMISSION_OK caller=surface syscall=input_acquire status=permission_denied audits=1 handles_delta=0 sessions_delta=0
INPUT_SERVER_RESTART_BACKOFF_OK attempt=1 requested_ns=30000000 timeout=1 early_spawn=0 budget=1/1
INPUT_SERVER_RESYNC_OK snapshot=1 routes=2 focus=launcher capture=none text=none floor=1 bic=6 bie=7
INPUT_SERVER_RESTART_ROUTE_OK sequence=2-3 target=app focus=launcher/app capture=down/up final_capture=none text_delta=0
INPUT_SERVER_RESTART_OK abi=23 protocol=1 wires=BIR1/BIP1/BIC1/BIE1 sessions=1/2 surface_session=1 epochs=1/2 restart=1 backoff=fixed-30ms budget=1/1 quarantine=host-verified broker=3/3/0/1 releases=1 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/2/2/9 handles=36 endpoints=30 pairs=15 waits=9/2/7 errors=0
BOOT_OK: M47 bounded InputServer restart, backoff, Surface resync, and permission denial verified
```

> `scripts/check-input-server-restart.sh` 已验证 pre/gap/post 三相；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`。dedicated M47、M46/M45 regression 与完整 suite 均已从头 exit 0；host 账本为 650 default + 24 directed = 674 executed/unique，M47 kernel 为 338，suite 终态固定 `input_server_restart=1`。

> M47 仍只是一次 InputServer restart 的 bounded runtime witness；`quarantine=host-verified` 只表示策略宿主测试通过，M47 QEMU 没有执行重复故障、runtime quarantine 或 degraded UI。上述 650+24=674/338、三张截图与完整总套件结论仍是不可回写的 M47 历史封口。

> **M48 历史完整封口**

> M48 保持 ABI v23 与 syscall 0—41 不变。无分配、定容 `ServiceSupervisor` 与 strict canonical little-endian fixed-64-byte `BSH1` 绑定 service kind、process generation 和 generation-qualified PID；opcode/flags/fault class、inbound/outbound sequence、restart attempt/budget、interval 及全部 reserved/padding 均先严格验证再推进状态。当前 QEMU 运行时只监督 InputServer，类型实例为 `ServiceSupervisor::<1>`。

> 独立 leaf 先以 `process-exit` 将第一次 attempt 用于重启 InputServer，再完成 session/epoch `1→2`、两条 route 与 floor 1 的 Surface resync。replacement 对首个 30 ms `Probe` 回复 `Healthy`；第二个 probe 被精确取出但故意不回复，watchdog 将第二个故障分类为 `health-timeout`。attempt 2 超过 budget 1，Init 不再 spawn，而是终止/reap replacement、发送 `Quarantine`、保持 broker unbound，并收到 `DegradedAck`；SurfaceServer 存活并提交 frame 9/write generation 6 的 32×24 红色 degraded badge。终态 process=`11/3/3/8`、handle/endpoint/pair=`31/26/13`、wait=`8/2/6`。

```text
SERVICE_SUPERVISOR_FIRST_FAULT_OK class=process-exit attempt=1 budget=1 gap=1 broker=unbound
SERVICE_SUPERVISOR_RESYNC_READY sessions=1/2 epochs=1/2 routes=2 floor=1 bic=6 bie=7
SERVICE_SUPERVISOR_WATCHDOG_OK probes=2 healthy=1 requested_ns=200000000 timeout=1 class=health-timeout
SERVICE_SUPERVISOR_QUARANTINE_OK attempt=2 budget=1 reason=restart-budget-exhausted degraded=1 frame=9 generation=6
SERVICE_SUPERVISOR_OK abi=23 protocol=1 wire=BSH1 faults=process-exit/health-timeout probes=2/1 watchdog=fixed-200ms restart=1 budget=1/1 quarantine=runtime degraded=1 broker=unbound releases=2 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/3/3/8 handles=31 endpoints=26 pairs=13 waits=8/2/6 errors=0
BOOT_OK: M48 generic ServiceSupervisor watchdog, runtime quarantine, and degraded UI verified
```

> `scripts/check-service-supervisor.sh` 严格验证 BSH1 trace 唯一性/顺序/字段、同槽 PID generation+1、broker 隔离、精确 resident topology/waits 与 pre/gap/recovered/degraded 四个不同视觉阶段。`target/m48/service-supervisor-{pre,gap,recovered,degraded}.ppm` 的 SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992` / `c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324` / `5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65` / `32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`。default host 分项为 ABI/compositor/ELF/input/SM/UI/init/kernel=`26/51/19/106/51/120/0/297`，即 670，加 24 directed 为 694；`bndr-sm` 由 31 增至 51 tests，M48 kernel 为 338。dedicated QEMU、静态矩阵与最终全量 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 均已 exit 0。精确终态 suite marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M48 仍只是单一 InputServer 的有界运行时见证，没有完成 SurfaceServer+InputServer 联合监督、服务依赖传播或多服务恢复；其 ABI-v23、694 host/338 kernel、四截图与 `service_supervisor=1` 现作为历史封口保留。

> **M49 历史完整封口**

> M49 保持 ABI v23 与 syscall 0—41 完全不变。`service-dependency-runtime` 实例化 `ServiceSupervisor::<2>`，注册 SurfaceServer 与 InputServer，并建立唯一的 `InputServer -> SurfaceServer` soft dependency。Surface 自身故障只 hard-block Surface，Input 继续存活；Input 故障 hard-block Input，同时把仍存活的 Surface 标为 soft-degraded。固定容量 DAG 对 self/duplicate/cycle、过期 identity 与错误恢复顺序 fail closed。

> QEMU 先让 Surface generation 1 在 pointer-down checkpoint 后退出；Input generation 1 跨 gap 存活，同槽 Surface generation 2 以 session 2、route epoch 2、floor 3 恢复并提交 teal frame 1。真实 2 s finite wait 固定该阶段，Surface 之后在 100 ms deadline 内返回 `Healthy`。Input generation 1 随后真实 dequeue Probe 但不回 `Healthy`，100 ms watchdog 分类 `health-timeout`；Surface 保持存活并提交 frame 2/write generation 2 的红色 32×24 degraded badge。再经真实 2 s degraded hold 与 fixed 30 ms restart backoff，Input generation 2 同槽启动，以 session 2、route epoch 3/floor 3 完成 BIR/BIP、六 BIC/七 BIE snapshot；Surface 提交 distinct green frame 3，Input2 返回 `Healthy`，两服务 impact 收敛为 `unaffected`，无 restart storm。终态 process=`12/3/3/9`、handle/endpoint/pair=`36/30/15`、wait=`9/2/7`。

```text
SERVICE_DEPENDENCY_SURFACE_RECOVERED surface_generation=2 surface_session=2 input_generation=1 input_session=1 route_epoch=2 floor=3 frame=1
SERVICE_DEPENDENCY_WATCHDOG service=input generation=1 probes=1 reads=1 healthy=0 timeout_ns=100000000 impact=input-hard/surface-soft
SERVICE_DEPENDENCY_DEGRADED surface_alive=1 input_alive=0 phase=route-lost frame=2 write_generation=2 floor=3
SERVICE_DEPENDENCY_INPUT_REBOUND input_generation=2 input_session=2 route_epoch=3 floor=3 bic=6 bie=7
SERVICE_DEPENDENCY_INPUT_HEALTHY probes=1 reads=1 healthy=1 input_generation=2
SERVICE_DEPENDENCY_OK abi=23 protocol=1 services=2 dependency=input-soft-surface health_messages=5 probes=3 probe_reads=3 healthy=2 watchdog=1/1 bir=8+3 bip=4 bic=6 bie=7 surface=1/2 input=1/2 sessions=1/2 epochs=1/2/3 floor=0/2/3 frames=1/2/3 outputs=3 restarts=1/1 impacts=unaffected/unaffected created=12 exited=3 reaped=3 live=9 reasons=exited1/killed2 handles=36 endpoints=30 pairs=15 waits=9/2/7 topology=resident final_state=ready
BOOT_OK: M49 dependency-aware SurfaceServer and InputServer supervision verified
```

> `scripts/check-service-dependency.sh` 默认离线并以 `-nic none` 禁用 QEMU 网络。pre/gap/surface-recovered/degraded/recovered 五阶段 SHA-256 已冻结为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`、`32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`、`a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d`。M49 历史 suite 精确整行为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M49 仍只是单核 QEMU 上固定两服务、单 soft edge、固定窗口与脚本故障的研究/模拟器系统，不是生产手机系统，也未达到真机可用。任意 App/window、产品级 IME/font/Unicode/multitouch、网络/蜂窝/Wi-Fi、音频、电源、安全启动、沙箱、安全更新、产品驱动与真实硬件闭环仍缺。

> **M50 历史完整封口**

> M50 保持 ABI v23、syscall 0—41、八镜像 catalog、process capacity 9 与单核 12-context 不变。`post-recovery-interaction-runtime` 从 M49 的 Surface/Input generation 2、session 2、route epoch 3/floor 3、output frame 3 恢复态继续运行，不新增 syscall、signal bit、镜像或 capability。

> QEMU 先注入 screen `(16,32)` 的 down/up，形成 physical sequence `4/5`；该点位于 phone 外，InputServer 判定 `target=none`，不产生 BWE、App Present 或 output commit，armed/rejected 两张截图逐字节相同。随后 App contact `(136,184)` 形成 physical `6/7`，产生两条严格 BWE InputRoute；App 提交 Present command/sequence 5、app frame 3，Surface 提交 scene 10、output frame 4、write generation 4。SurfaceServer 与 InputServer 再分别完成 health sequence 2，终态 health=`2/2` 且两服务保持 resident。M50 不把 M49 standalone checker 的两秒 stable 阶段移植为自己的证据。

> 本次稳定性封口还移除了共享 M41 前缀对偶然跨进程调度顺序的依赖：Launcher 先消费 hidden `Presented` 再发后续命令，App 先消费首阶段三条 route 再 continuation，phase-two route 边界再由 sender-authenticated private 8-byte acknowledgement 闭合；kernel trace 独立要求 App command 3 晚于 route 2、command 4 晚于 route 3。ABI、公开 window wire、命令/事件计数、视觉结果与截图 hash 均不变；完整离线 suite 已从头通过，全部 QEMU 启动均显式 `-nic none`。

```text
POST_RECOVERY_ARMED abi=23 surface_session=2 input_session=2 route_epoch=3 floor=3 frame=3
POST_RECOVERY_REJECTED physical=4/5 target=none routed=0 frame=3
POST_RECOVERY_ROUTED physical=6/7 target=app events=2 capture=1
POST_RECOVERY_PRESENTED command=5 app_frame=3 scene=10 output_frame=4 write_generation=4
POST_RECOVERY_HEALTHY surface=2/2 input=2/2
POST_RECOVERY_INTERACTION_OK abi=23 surface_session=2 input_session=2 route_epoch=3 rejected=4..5 physical=6..7 target=app events=2 capture=1 app_present=1 output_frame=4 health=2/2 resident=1 errors=0
BOOT_OK: M50 post-recovery resident input-to-frame interaction verified
```

> `scripts/check-post-recovery-interaction.sh` 强制 `CARGO_NET_OFFLINE=true`、仅使用本地 Unix QMP，并以 `-nic none` 显式关闭 QEMU 网络。armed/rejected 的 SHA-256 均为 `a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d`；presented 为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`。M50 历史 suite 精确整行为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M50 仍只是单核 QEMU、固定两服务、固定窗口、一个非法 contact 加一个合法 App contact 的有界研究实现；它不是现实硬件上的电话系统，也不是量产或真实可用手机系统。蜂窝/电话、Wi-Fi/网络、音频、电源、产品驱动、安全启动、应用沙箱、安全更新、完整 IME/Unicode/multitouch、任意 App/window 与真机闭环仍未完成。

> **M51 历史完整封口**

> M51 保持 ABI v23、syscall 0—41、八个 AArch64 ELF、process capacity 9 与单核 12-context 不变。`post-recovery-focus-runtime` 不是另一条重放路径：它严格继承 M49 的 SurfaceServer+InputServer dependency recovery 与 M50 的拒绝/合法 App contact、output frame 4、health `2/2` resident 终态，不新增 syscall、signal bit、镜像、capability 或公开 wire。

> M50 终态为 focus `App/3` 后，QEMU 在 screen `(80,96)` 注入 down/up。physical 8 先路由到 Launcher 并建立 capture，唯一 BIC `SetFocus` 以 sequence 3 将 focus 推进为 `Launcher/4`；physical 9 release 清除 capture。认证 trace 精确收敛为 Channel write/read=`8/8`，Launcher 再提交 Present command 6/frame 4，SurfaceServer 提交 scene 11、output frame 5、write generation 5，health 仍为 `2/2` 且两服务 resident。M50 已完成的 M41 因果稳定修正继续保留：hidden `Presented`、App continuation、private acknowledgement 与 route-before-command 约束仍是该继承前缀的一部分。

```text
POST_RECOVERY_FOCUS_ARMED abi=23 surface_session=2 input_session=2 route_epoch=3 physical_floor=7 focus=app/3 output_frame=4
POST_RECOVERY_LAUNCHER_CAPTURED physical=8 target=launcher events=1 focus=launcher/4 capture=1
POST_RECOVERY_LAUNCHER_PRESENTED physical=8/9 command=6 launcher_frame=4 scene=11 output_frame=5 write_generation=5 focus=launcher/4 capture=0
POST_RECOVERY_FOCUS_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=8..9 target=launcher events=2 capture=0 launcher_present=1 output_frame=5 focus=launcher/4 health=2/2 resident=1 errors=0
BOOT_OK: M51 post-recovery App-to-Launcher focus and frame verified
```

> `scripts/check-post-recovery-focus.sh` 强制 `CARGO_NET_OFFLINE=true`，只使用本地 Unix QMP，并以 `-nic none` 启动 QEMU。全局 damage 为 `64/80/40/32`，armed→presented 精确改变 `1280` pixels；两张截图 SHA-256 分别为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974` 与 `cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729`。专项 checker 与当时的全量 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 均 exit 0；M51 历史 suite 精确整行为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> 与 M51 leaf 独立，旧 userspace UI 的 default feature-off 路径不安装 directed stale-Present 屏障；只有 opt-in `ui-stale-present-evidence` checker 才在 13-commit baseline 后持有 App frame 10/focus generation 8，经 Home focus generation 9 走既有 cancellation，再以同一 frame 10/focus generation 10 重试并最终回到 Home generation 11。该旧 UI 证据严格收敛为 36 inputs/18 commits，不改变 ABI、wire 或默认行为。

> M51 仍只是单核 QEMU、固定 Launcher/App 双窗口和脚本注入交互的有界研究系统。`-nic none` 明确表示验证时没有网络设备；项目也没有真实硬件网络、蜂窝/电话、Wi-Fi、音频、电源、产品驱动、安全启动、完整沙箱/更新、任意 App/window 或真机闭环，因此不是现实可用手机系统。

> **M52 历史完整封口**

> M52 保持 ABI v23、syscall 0—41、八个 AArch64 ELF、process capacity 9 与单核 12-context 不变。`post-recovery-focus-roundtrip-runtime` 以完整 M49 双服务恢复、M50 App input-to-frame 和 M51 App→Launcher focus/frame 作为同一条严格前缀，不新增 syscall、signal bit、镜像、capability 或公开 wire；M51 终态精确冻结在 input event 14、physical 9、route sequence 4、focus sequence 3、`Launcher/4`、dependency step 7 与 output/epoch/write generation=`5/5/5`。

> QEMU 通过本地 Unix QMP 在 App screen `(136,184)` 注入 contact。physical 10 down 产生 BIE event 15，映射为 compositor `(80,120)` 与 App local `(32,40)`，建立 capture 并把 focus 从 `Launcher/4` 推进到 `App/5`；唯一 BIC `SetFocus` sequence 4 指向 App window id/generation=`2/1`，其 wire ACK 为 BIE event 16，并把 dependency step `7→8`。physical 11 up 产生 BIE event 17 并清除 capture。App 收到 command 5 的 down/up BWE（scene 11、focus 5），再发 BWC Present command 6/frame 4，local damage=`56/72/40/32`、color=`0x00844ec7`、content generation 8；SurfaceServer 返回 Presented scene 12，并把 output frame/output epoch/write generation 一并推进到 `6/6/6`。App 消费 Presented 后另发 sender-authenticated private 8-byte completion ACK，magic=`0x4d35325f4150434b`；该确认发生在 17-bit kernel trace 封口之后，冻结 trace 的 Channel write/read=`8/8`。

```text
POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED abi=23 surface_session=2 input_session=2 route_epoch=3 physical_floor=9 focus=launcher/4 output_frame=5
POST_RECOVERY_APP_CAPTURED physical=10 target=app events=1 focus=app/5 capture=1
POST_RECOVERY_APP_PRESENTED physical=10/11 command=6 app_frame=4 scene=12 output_frame=6 write_generation=6 focus=app/5 capture=0
POST_RECOVERY_FOCUS_ROUNDTRIP_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=10..11 target=app events=2 capture=0 app_present=1 output_frame=6 focus=app/5 health=2/2 resident=1 errors=0
BOOT_OK: M52 post-recovery Launcher-to-App focus roundtrip and frame verified
```

> `scripts/check-post-recovery-focus-roundtrip.sh` 强制 `CARGO_NET_OFFLINE=true`、只使用本地 Unix QMP，并以 `-nic none` 启动 QEMU；它要求 M49/M50/M51 marker 全部唯一且顺序完整，且只有 M52 可以发布终态 `BOOT_OK`。M50→M51 的 framebuffer PPM damage=`64/80/40/32`，M51→M52 的 compositor damage=`104/152/40/32` 因 Surface scanout origin `(56,64)` 对应 framebuffer PPM damage=`160/216/40/32`，两段都精确改变 `1280` pixels。M50/M51/M52 SHA-256 分别为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`、`cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729` 与 `97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`。精确 checker seal 为：

```text
POST_RECOVERY_FOCUS_ROUNDTRIP_QMP_OK pointer=136/184/down-captured-up target=app focus=launcher4-app5 output=5-6 diffs=1280/1280 damage=64/80/40/32+160/216/40/32 m50_sha256=798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974 m51_sha256=cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729 m52_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb markers=m49-prefix/m50-prefix/m51-prefix/roundtrip-armed/app-captured/app-presented/roundtrip/boot
```

> 专项 checker 与完整 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已从头 exit 0；历史 M52 阶段全量 suite 的精确终态 marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M50 保留的 M41 因果稳定约束仍完整成立，旧 userspace UI 的 opt-in `ui-stale-present-evidence` checker 也继续确定性收敛为 36 inputs/18 commits；二者都不改变 default feature-off 路径。M52 仍只是单核 QEMU、固定 Launcher/App 双窗口、固定两服务和脚本注入的 focus roundtrip；lifecycle focus 账本仍停留在 2。它不是生产手机或真实硬件系统，验证明确没有网络设备，也未实现真实硬件网络、蜂窝/电话、Wi-Fi、音频、电源、产品驱动、安全启动、完整沙箱/更新、任意 App/window 或真机闭环。

> **M53 历史完整封口**

> M53 保持 ABI v23、syscall 0—41、八个 AArch64 ELF、process capacity 9 与单核 12-context 不变。新 opt-in `post-recovery-lifecycle-focus-runtime` 以 `post-recovery-focus-roundtrip-runtime` 为直接 feature parent，完整继承 M52；它不新增 syscall、signal bit、镜像、capability、公开 wire、QMP 输入或图形提交。replacement SurfaceServer 与原 Launcher/App 在 session 2 重绑后，先同步两条 `Ready` 和两条 `FocusChanged(App, Phone, generation 1)`，双客户端各回 RFCK；随后既有 physical 8 切到 `Launcher/2` 并各回 LFCK，既有 physical 10 切回 `App/3` 并各回 AFCK。每个阶段的两个客户端读与同阶段 ACK 顺序可独立交错，但 owner、`UserImageId`、sender、Bytes kind、session、payload、generation 和阶段均由 kernel trace 严格认证。

> M53 lifecycle 账本按阶段累计为 Channel `6/6 → 10/10 → 14/14`；终态 `14/14` 精确拆为 BUE write/read=`8/8` 与 RFCK/LFCK/AFCK write/read=`6/6`。M52 已存在的 App private APCK `0x4d35325f4150434b` 只在 M52 trace 完成且 M53 lifecycle `14/14` 后，另以 boundary write/read=`1/1` 封口；它不计入 M53 lifecycle Channel `14/14`，更不属于 M52 自身先前已经冻结的 17-bit、Channel `8/8` trace。output hook 只认证既有 frame `1..6`，M53 happy path 的额外 output count 为 0；任何 frame `>6` 都会 fail closed。

```text
POST_RECOVERY_LIFECYCLE_SESSION_READY surface_session=2 clients=launcher/app ready_events=2 focus=app/1 lifecycle_events=2 ack=rfck/2 channels=6/6
POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED physical=8 clients=launcher/app focus=launcher/2 lifecycle_events=2 ack=lfck/2 channels=10/10 compositor=launcher/4 input=launcher/3
POST_RECOVERY_LIFECYCLE_APP_SYNCED physical=10 clients=launcher/app focus=app/3 lifecycle_events=2 ack=afck/2 channels=14/14 compositor=app/5 input=app/4
POST_RECOVERY_LIFECYCLE_FOCUS_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=1..11 clients=2 ready_events=2 focus_events=6 acks=rfck2/lfck2/afck2 lifecycle=app/1-launcher/2-app/3 compositor=app/5 input=app/4 channels=14/14 m52_boundary=1/1 output_frame=6 health=2/2 resident=1 errors=0
BOOT_OK: M53 post-recovery lifecycle focus convergence verified
```

> `scripts/check-post-recovery-lifecycle-focus.sh` 全程离线，固定 `CARGO_NET_OFFLINE=true`，只使用本地 Unix QMP，并以 `-nic none` 明确关闭网络设备。它重放 M49—M52 的既有 physical `1..11`，M53 自身 QMP input count=`0`；最终 input event 仍为 17、Input focus=`App/4`、compositor focus=`App/5`、output frame/write generation=`6/6`，没有新 input、damage 或 frame。M50/M51/M52/M53 SHA-256 依次为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`、`cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729`、`97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`、`97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`；三段 framebuffer diff=`1280/1280/0`，最后一段 damage=`none`。精确 checker seal 为：

```text
POST_RECOVERY_LIFECYCLE_FOCUS_QMP_OK physical=1..11 m53_input=0 lifecycle=app1-launcher2-app3 clients=2 ready=2 focus_events=6 acks=rfck2/lfck2/afck2 channels=14/14 m52_boundary=1/1 compositor=app5 input=app4 output=6-6 diffs=1280/1280/0 damage=64/80/40/32+160/216/40/32+none m50_sha256=798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974 m51_sha256=cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729 m52_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb m53_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb markers=m49-prefix/m50-prefix/m51-prefix/m52-prefix/session-ready/launcher-synced/app-synced/lifecycle-focus/boot
```

> M53 专项 checker 与完整 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已从头 exit 0；M53 当时全量 suite 的精确终态 marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M53 仍只是单核 QEMU 上固定 SurfaceServer/InputServer、固定 Launcher/App 双窗口和确定性脚本前缀的有界研究原型，不是真实可用手机系统。它没有真机硬件适配、蜂窝/电话、Wi-Fi、硬件网络、音频、电源管理、产品级驱动与 GPU/display pageflip，也没有安全启动、完整沙箱/权限/签名/OTA、通用服务恢复、通用 App/window、原生应用生态、Android APK 运行闭环或产品级 UX。

> **M54 历史完整封口**

> M54 新增独立 opt-in `app-data-runtime`，只在该 leaf 把 ABI 推进到 v24；feature-off default 与完整封口的 M53 继续使用 ABI v23。确定性磁盘镜像新增 GPT index 2 `BNDROID_APPDATA`，范围固定为 LBA `128-2047`、共 1920 sectors；M54 monitor stack 为 256 KiB，M53 回归仍为 128 KiB。M33—M54 共二十二个 opt-in leaf，加默认路径合计二十三套独立账本。

> AppData 是 capability-scoped、固定容量的 `/data` 原型：path 最长 64 bytes、目录深度最多 4、单文件最多 4096 bytes、volume 内目录与文件合计最多 32 entries、live payload 总量最多 128 KiB；只服务 single principal 1。root capability 只有 `READ|WRITE|DUPLICATE`，没有 `TRANSFER`；读权限衰减后的 handle 不能执行 mutation，Init、Launcher 与两个 SurfaceServer identity 均被拒绝。

> 四次持久化专项运行分别证明：fresh image 完成 created 且 generation `0→2`；boot 2 把既有数据 upgraded `2→3`；boot 3 在 generation 3 stable mount，写入数为 0；损坏 newest checkpoint 后从 generation 2 fallback，并重新升级到 generation 3。`bndr-appdata` 的 22 项 host tests 包含 292 个 mutation crash points 与 965 个 sector-atomic first-format points；首个 `FORMAT_INTENT` sector 或最终 immutable superblock 的 torn write 都 fail closed。

> M54 专项 checker 四次均在 `CARGO_NET_OFFLINE=true`、本地 QMP、每次 QEMU 显式 `-nic none` 的条件下 exit 0。该 checker 证明的是重启、稳定挂载和 newest-checkpoint corruption recovery，不模拟真实断电，因此精确边界为 `powercut_claim=0`；断点枚举只属于 host sector model。M52、M53 与 M54 截图 SHA-256 仍同为 `97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`，M52→M53、M53→M54 framebuffer diff 均为 0。

> M54 专项 checker 与完整 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 均已从头 exit 0；最终 suite marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M54 仍是单核 QEMU、single principal、固定容量且由 kernel monitor 执行 AppData 请求的研究原型，不是独立 StorageServer，也不是 POSIX 文件系统。它没有通用 writable fd、rename/link、block/page cache、存储加密、认证、anti-rollback、多 App 隔离、真实块控制器或手机硬件闭环；`general_runtime=0`，不能称为现实可用手机系统。完整 suite exit 0 只封口上述有界能力，不扩大为真实手机验证。

> 下列 M41 marker 保留为上一里程碑的 bounded userspace two-window compositor 精确证据。

> M41 让用户态 SurfaceServer 通过 strict 64-byte `BWC1`/`BWE1` 管理固定 Launcher/App 两层，非法 wire fail closed；完成 2 create、5 present、2 raise 与 5 次 input route。Launcher hidden damage 1024 pixels 不产生 output，content/visible/occluded=`100864/97536/3328`；App capture phone 外拖拽并收到 signed local 坐标，Launcher raise 后取得第二阶段 focus/capture。SurfaceServer 独占两槽 output，六帧严格 `A-B-A-B-A-B`，generation=`1-1/2-2/3-3`，内核只接收最终 full-buffer commit。

```text
WINDOW_COMPOSITOR_OK abi=20 protocol=1 policy=userspace capacity=2 live=2 z=launcher-app commands=9/2/5/2 events=14/2/5/2 input=5 capture=app-then-launcher signed=2 focus=1-2 damage=100864/97536/3328 hidden=1 retained=1 raises=35840 output=full-buffer frames=6 schedule=0-1-0-1-0-1 generations=1-1/2-2/3-3 clock=software-timer release=post-copy opportunities=7 edges=7 acquired=6 presented=6 pending=1 lifecycle=10/0/2/2 supervisor=4/0/2/2 processes=9/1/1/8 handles=31 endpoints=26 buffers=2 map=2/2 queue=6/6 acquire=6/6 releases=6 validates=459264/1837056 mappings=2/150 protects=12 producer=2/rw consumer=0 shared_pairs=0 physical_alias=0 pool=2/0/0 peaks=1/1/1 per_slot=3/3/3/3/3/3 waits=8/2/6 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M41 bounded userspace two-window z-order, occlusion, damage, and input capture verified
MULTI_WINDOW_OK phase1_raw=13970/12587->136/184 drag_raw=7396/6568->72/96 phase2_raw=13970/12587->136/184 markers=1/1/1/1
```

> 下列 M40 marker 保留为上一里程碑的 resident two-buffer software-paced swapchain 精确证据。

> M40 让两个 distinct mapped slot 同时在途，六次 acquire 严格 `A-B-A-B-A-B`，成功 commit 为 `A2 → B2 → A3`，每次 post-copy release 后最终 A3 Writable、B3 Acquired。失败 Queue/acquire/frame-acquire/present/pool-exhaustion 均事务不推进，validation failure 保留 grant。终态为 31 handles、4 mappings/300 pages、两对 identity-matched physical alias，App producer 一 RW 一 RO、SurfaceServer consumer 均 RO。

```text
GRAPHICS_SWAPCHAIN_OK abi=20 protocol=1 clock=software-timer release=post-copy tick_hz=100 frame_hz=50 divider=2 phase=ready calls=4/3/0/1 opportunities=4 edges=4 acquired=3 presented=3 suppression=observed pending=1 outstanding=0 discarded=0 cancelled=0 epochs=3/3 grant_preserved=1 ungated_rejects=4 messages=10 errors=0 transactions=2 completed=2 supervisor=4/0/2/2 created=9 exited=1 reaped=1 live=8 handles=31 endpoints=26 waits=8/2/6 process_waits=2/1/1 buffers=2 mappable=2 pool_exhaustions=1 map=4/4 unmap=0/0 queue=8/6 acquire=7/6 explicit_release=2/2 releases=5 mapped_presents=3 total_presents=3 validated_pixels=459264 validated_bytes=1837056 copy_writes=0/0 mappings=4/300 protects=11 producer=2/mixed consumer=2/ro shared_pairs=2 physical_alias=1 identities=2 owner_pairs=2 allocation_generations=1/1 write_generations=3/3 refs=4/4 pool=1/0/1 peaks=2/2/2 dual=4 selective=4 per_slot_queue=3/3 per_slot_acquire=3/3 per_slot_release=3/2 acquire_order=0-1-0-1-0-1 switches=5 schedule=0:2/1:2/0:3 next=1:3 final_buffers=writable/acquired contexts_distinct=1 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M40 resident two-buffer software-paced swapchain verified
```

> 下列 M39 marker 保留为上一里程碑的单 buffer software-frame-clock 精确证据。

```text
GRAPHICS_FRAME_CLOCK_OK abi=20 protocol=1 clock=software-timer tick_hz=100 frame_hz=50 divider=2 phase=ready calls=3/3/0/0 opportunities=4 edges=4 acquired=3 presented=3 suppression=observed pending=1 outstanding=0 discarded=0 cancelled=0 epochs=3/3 grant_preserved=1 ungated_rejects=3 messages=10 errors=0 transactions=2 completed=2 supervisor=4/0/2/2 created=9 exited=1 reaped=1 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=2/1/1 buffers=1 mappable=1 map=2/2 unmap=0/0 queue=4/4 acquire=4/4 explicit_release=1/1 releases=4 mapped_presents=3 total_presents=3 validated_pixels=306176 validated_bytes=1224704 copy_writes=0/0 mappings=2/150 protects=8 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 buffer_generation=1 write_generation=4 contexts_distinct=1 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M39 software frame clock and gated mapped presents verified
```

```text
MAPPED_GRAPHICS_OK abi=19 protocol=1 messages=35 errors=0 transactions=7 completed=7 supervisor=14/0/7/7 first_instance=1 last_instance=2 first_pid=0x0000000100000008 last_pid=0x0000000200000008 created=10 exited=2 reaped=2 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=3/2/1 buffers=2 mappable=2 map=4/4 unmap=2/2 queue=4/4 acquire=4/4 explicit_release=2/2 releases=4 mapped_presents=2 total_presents=2 validated_pixels=306176 validated_bytes=1224704 copy_writes=0/0 mappings=2/150 protects=8 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 buffer_generation=2 contexts_distinct=1 topology=exact final_state=active final_app_resident=1
BOOT_OK: M35 shared mapped BufferQueue and acquire/release fences verified
```

```text
GRAPHICS_OWNER_DEATH_OK abi=19 protocol=1 messages=5 errors=0 transactions=1 completed=1 supervisor=2/0/1/1 consumer_pid=0x0000000100000006 producer_pid=0x0000000100000008 created=9 exited=3 reaped=3 live=6 handles=19 endpoints=19 waits=4/2/2 process_waits=4/3/1 owner=1/1/0/1/1/0/1 wake_nonzero=1 buffers=1 map=2/2 unmap_syscalls=2/1 mappings_removed=2 denied_consumer_unmap=1 denied_mapped_close=1 queue=1/1 acquire=1/1 explicit_release=0/0 releases=1 mappings=0/0 protects=2 surfaces=0 graphics_handles=0 producer=0/unmapped consumer=0/unmapped shared_pairs=0 el0_rewrite=all-pages explicit_cleanup=1 surface_absent=1 app_exited=1
BOOT_OK: M36 graphics consumer owner-death release and producer recovery verified
```

```text
GRAPHICS_SURFACE_RESTART_OK abi=19 protocol=1 messages=10 errors=0 transactions=2 completed=2 supervisor=4/0/2/2 old_surface=0x0000000100000006 new_surface=0x0000000200000006 app=0x0000000100000008 sessions=1/2 created=10 exited=2 reaped=2 live=8 handles=29 endpoints=26 waits=8/2/6 process_waits=3/2/1 owner=1/1/0/1/1/0/1 wake_nonzero=1 reacquire=1 discarded_input=0 buffers=1 map=3/3 unmap_syscalls=1/0 mappings_removed=1 queue=2/2 acquire=2/2 explicit_release=0/0 releases=2 mapped_presents=1 total_presents=1 validated_pixels=153088 validated_bytes=612352 mappings=2/150 protects=4 producer=1/rw consumer=1/ro shared_pairs=1 physical_alias=1 identity=1 surface_session=2 surface_owner=replacement frame=1 topology=exact final_state=active final_app_resident=1
BOOT_OK: M37 SurfaceServer restart, client rebind, and mapped frame recovery verified
```

```text
GRAPHICS_PRODUCER_ORPHAN_OK abi=19 protocol=1 messages=5 errors=0 transactions=1 completed=1 supervisor=2/0/1/1 consumer_pid=0x0000000100000006 producer_pid=0x0000000100000008 reuser_pid=0x0000000100000007 created=9 exited=3 reaped=3 live=6 reasons=1/0/2 handles=18 endpoints=18 pairs=9 waits=4/2/2 process_waits=4/3/1 owner=1/2/0/1/0/1/1 wake_nonzero=1 buffers=4 mappable=4 exhausted=1 map=6/6 unmap_syscalls=3/2 mappings_removed=6 protects=1 queue=1/1 acquire=1/1 explicit_release=0/0 releases=1 mappings=0/0 surfaces=0 graphics_handles=0 pool=2/2 reused=2 generations=3/3 vacant=2 scrubbed=2 producer=absent consumer=absent topology=exact
BOOT_OK: M38 producer orphan reclamation and two-slot graphics-buffer reuse verified
```

> M31 在 M30 的独立 SurfaceServer/Launcher 边界上增加独立 EL0 App 与第二对 UI Channel。SurfaceServer 仍是唯一 Surface capability owner；Launcher 和 App 都无法 acquire Surface。64-byte `BUC1` v1 控制协议用 client-only `AttachAppEndpoint` 转移 App 的 server endpoint，不把 app/focus transition 混入 attach；只有 Launcher 可以用严格递增 transition id 发起 `SetFocus`。SurfaceServer 以三项 wait-array 轮换等待 Surface、Launcher 与 App，两个 client 各用单对象 wait；两对通道都固定 kernel-stamped generation-qualified sender PID。

> Present protocol 已升为 v2，wire size 仍为 64 bytes。Launcher/App 提交带当前 focus generation 的 frame；server 只提交当前焦点 client 且 generation 匹配的帧，并把两个 client 的本地连续 frame id 映射为一条全局连续 commit 序列。后台或 stale-generation present 返回 `PresentCancelled`，不推进本地/全局 frame 序列，client 因而可以用同一 frame id 在新 generation 下重试。default feature-off 保持直接分发且不注入时序；opt-in `ui-stale-present-evidence` 用 capacity-1 屏障在精确 13-commit baseline 后读入并持有 App frame 10/focus generation 8，待双方 FocusChanged generation 9 发布完成后立即走既有 cancellation，再于 App focus generation 10 用同一 frame 10 重试，最后回到 Home generation 11。该确定性单次证据收敛为 36 inputs/18 commits，不再依赖多次运行或尝试不同调度相位。pointer down 时在 Home→Launcher 或当前 App 之间选定 recipient，并捕获到对应 release；surface trace 也证明在 Settings 内按下、拖到 Home 区再抬起的 App 本地 sequence `1..3` 没有泄漏给 Launcher，也不触发 focus/frame commit。Launcher 独立绘制 Home，App 独立绘制 Phone/Messages/Settings。

> M32 增加两槽、page-aligned 的静态 XRGB8888 GraphicsBuffer pool：每槽固定 `208×368`、logical/backing `306176/307200` bytes。App 以 `READ|WRITE|DUPLICATE|TRANSFER=0x0f` 创建，用 syscall 29 进行非空、4-byte 对齐且单次最多 4096-byte 的规范化写入，再把 `READ|TRANSFER=0x09` 衰减副本随首个 64-byte `BUP1` `BufferPresent` 原子转移给 SurfaceServer。M32 历史 wire 为 v1；当前 v2 保留 Full geometry 和 client/global frame id、focus generation、buffer generation，并新增 byte 40..48 的 System UI revision；mobile frame 必须绑定当前非零 revision。最后一个引用释放时完整擦除 backing 并推进 slot generation。

> buffer decode、producer/rights、focus/sequence、generation、全部像素和 counter 在首像素前验证，失败/取消不改变 scene、序列与计数。启动期以一次 4-byte seed 和 pre-focus cancelled present 验证句柄握手，所以启动账本 `created/write_calls/write_bytes/presents=1/1/4/0`；交互路径由 App raster 76544 个真实像素并由 SurfaceServer copy-present。early per-page W^X window 为此从 RAM 前 4 MiB 扩为 6 MiB，新增 L3 覆盖但不放松权限。这是有界 object transfer+copy，不是 `mmap`、共享 EL0 mapping 或零拷贝。

> M23 把 fixture 固定为 8 MiB/16,384 sectors，SHA-256 为 `36f39e23da09401dc2f686217e2bb06d146a65cd14aa4b9b8b2f0fd4aa7b809b`，sector 0/1 FNV-1a64 为 `0xbebd264b8c14cd72`/`0x4ff56c6cb05d259b`。allocation-free block boundary 在 M22 IRQ 驱动上逐 sector 读取；GPT 路径严格验证 protective MBR、primary/backup header CRC、两份 partition-entry array CRC 和逐字节一致性，找到 LBA `2048-16350` 的 `BNDROID_SYS`。FAT16 挂载验证 512-byte sector、双 FAT 镜像、14,186 个 data cluster 和有界 cluster chain；单一只读 VFS 将它挂到 `/system`，读取根文件 `/system/HELLO.TXT`（28 bytes，digest `0xdd2f71342016eede`）及嵌套文件 `/system/SYSTEM/BUILD.TXT`（40 bytes，digest `0xe57ce4ce9f1b4ec0`），并拒绝 traversal、mount escape 与 write。parser 发出 142 次 sector read；连同最初双请求，正常路径共 144 次 IRQ-only completion，仍为两个永久 coherent DMA frame、`poll_fallbacks=0`。这些 M23 磁盘证据在 M24 原样保留。

> M24 在验证上述两个文件后把固定 28+40=68 bytes 缓存为 immutable `BootfsCatalog<2>`/VMO。init 入口 `x0` 得到 move-only `READ|TRANSFER` directory capability（不可 duplicate）；`FileOpenAt` 23 只接受最长 64-byte canonical root-relative UTF-8 path，返回 `READ|DUPLICATE|TRANSFER` VMO 和 size；`VmoRead` 24 从 `x2` 高/低 32 bit 解包 offset/requested length，每次最多 4096 bytes，按 EOF 截短。VMO 没有 `WRITE|MAP|EXECUTE|WAIT`。EL0 验证两个完整内容/FNV、partial、EOF、bounds、bad address、attenuation、stale handle 和 zero-payload transfer，且 catalog 发布后 `runtime_disk_reads=0 mapped=0 shared_memory=0`。

> M25 继续使用一个确定性 8 MiB GPT raw image，但加入 index 1、LBA `64-127` 的 private `BNDROID_DATA`，同时保持 `BNDROID_SYS` LBA `2048-16350` 与 FAT16 只读。当前 fixture SHA-256 为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`，sector 0/1 FNV-1a64 为 `0xbebd264b8c14cd72`/`0x8294de399174037c`。normal virtio device 必须可写并协商 `VERSION_1|FLUSH`；raw WRITE type 1 与 FLUSH type 4 都以 `used.len=1` 完成，但写 API 只允许 DATA 范围。DATA superblock 把 16-byte format epoch 绑定到该 GPT entry 的 unique GUID；两个 512-byte CRC record slot 若同时有效，generation 必须相邻。提交严格执行 inactive-slot WRITE completion → FLUSH completion → 双槽重读，保留旧 selected slot；fresh boot 证明 `0→1`，系统分区与 DATA 外写入都在 ledger 不变时拒绝。下文 M25—M30 marker 只作为对应阶段的历史证据；其中 ABI v14/v15、旧五/七进程与 manager-owned/单-client Surface 拓扑不能冒充 M31。

> M26 严格唯一发现 direct-root `qemu,fw-cfg-mmio`，验证 `QEMU` signature/features，并用 big-endian DMA 配置 `etc/ramfb`。kernel-owned XRGB8888 framebuffer 为 320×480、stride 1280、614400 bytes/150 pages；九色静态 splash digest 为 `0x6ef9c2b7d15fde25`。`check-framebuffer.sh` 经 headless QMP screendump 验证全部 153600 pixels、9 colors/9 samples，pixel SHA-256 为 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。modern coherent virtio-input keyboard 为 device ID 18、MMIO `0x0a003c00`、SPI 78、queue 8/one DMA frame，验证 name/key bitmap 中 A+Enter；block 保持 `0x0a003e00`/SPI 79。`check-input.sh` 的 QMP A down/up 精确得到 A-down/SYN/A-up/SYN，completion/delivery/recycle=`4/4/4`、used/avail=`4/12`，drop/invalid/config IRQ/spurious 均为 0。这些是保留的 M26 历史证据。

> M27 在 M26 scanout 上加入两个 kernel-owned 软件层：完整 320×480 opaque scene 与 12×22 alpha cursor；移动时从 immutable scene 恢复旧 rect，只在 clipped/union dirty rect 内合成新 cursor，按下/释放有不同视觉与 digest。隐藏 cursor 的启动帧保持 FNV `0x6ef9c2b7d15fde25` 和 RGB SHA-256 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。输入驱动同时永久拥有 keyboard `0x0a003c00`/SPI 78 和 tablet `0x0a003a00`/raw `0/45/1`/SPI 77；每个 size-8 eventq 独占一个 DMA frame。tablet ABS X/Y 都为 `0..32767`，要求 `BTN_LEFT`/`BTN_TOUCH`，只在 SYN boundary 提交 report。`check-compositor.sh` 精确注入 raw X/Y `1234/23456`、SYN、touch down/SYN、touch up/SYN，共 7 events/3 samples，映射 pixel `12/342` 并触发 3 次 dirty redraw；前后全帧恰有 122 个变化 pixel，bbox `12/342/21/363` 完全位于 dirty rect `12/342/12/22`，最终 RGB SHA-256 为 `f24699248bcdfe5069fa13a40ef7990ba7a647cfe0ac818c04f671047e748f82`。这些是保留的 M27 历史证据。

> 交互运行 `./scripts/run-qemu.sh` 会同时挂载 ramfb、virtio keyboard 与 virtio tablet；`check-compositor.sh` 保留 M27 cursor 验收，当前 `./scripts/check-ui.sh` 以 opt-in evidence feature 验证 M32 `BUP1`/真实 App raster：baseline 为 24 input/13 commit（4 legacy Launcher + 9 buffer App），directed run 为 36 input/18 commit；frame 10 在 focus generation 8 被持有、在 generation 9 取消、于 generation 10 同帧重试并最终到 Home generation 11。Settings buffer generation 为 `76/151/226`，最终 RGB SHA-256 为 `62b15db5e54d3bbca7bd23e74a60e7fc66d2566e2094be7718e7c8f1a0ebc5dd`；下列 M28 结果仅是历史基线。

> M28 历史阶段在 M27 scene/cursor 上增加 kernel-owned clickable shell：home view 有 phone/messages/settings 三个 app hit target 与一个 home target；press 时 capture，release 必须命中同一 target 才激活。QMP 在 pixel `160/342` 点击 Settings，使 view `home→settings`、generation `0→1`；damage/composition=`56/64/208/368`，written/restored=`76544/76544`，alpha blended=`122`，scene/scanout digest=`0xf79f5bb3582452a5`/`0xa2b4da7c5f306a09`。`check-ui.sh` 的 6 个 QMP command 精确形成 14 events/6 samples，前后截图 baseline/after SHA-256=`0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`/`70720db4523aa6eb33746b76fcac314a99039b79d3aaee85422edb839bc2ee76`，diff=53760 pixels、bbox=`56/64/263/379`、colors=13；随后以 15 个导航 QMP command/35 events 验证 Settings、Phone、Messages 与三次 Home 返回，连同截图阶段总计 49 events/21 samples，`targets=3 home_returns=3 transitions=6 final_view=home`。单轴 ABS 初始化帧会保留已知轴并等待另一轴；正常输入也不会仅因 `POINTER_EVENT_OK` 证明专用的静止位置或精确事件/样本计数尚未匹配而 fatal，畸形 axis/SYN contract 仍 fail-stop。M28 当时明确为 `owner=kernel userspace_surface=0`；以下 marker 仅作历史对照：

> 以下 M28 marker 中 `max_buffered=1..8`、`irq_entries=>0`、`queue_irqs=>0` 表示脚本接受的调度相关范围，不是 guest 的固定字面输出。

```text
UI_READY protocol=1 owner=kernel view=home targets=3 phone=72/132/176/72 messages=72/220/176/72 settings=72/308/176/72 home=128/448/64/24 gesture=tap capture=1 hit_test=1 surface=opaque-scene surface_generation=0 damage_commit=1 userspace_surface=0
UI_EVENT_OK protocol=1 gesture=tap target=settings from=home to=settings reports=6 samples=6 taps=1 transitions=1 generation=1 pixel_x=160 pixel_y=342 press=1 release=1 capture=1 hit_test=1
SURFACE_COMMIT_OK owner=kernel surface=settings format=XRGB8888 generation=1 damage=56/64/208/368 written=76544 composition=56/64/208/368 restored=76544 blended=122 scene_digest=0xf79f5bb3582452a5 scanout_digest=0xa2b4da7c5f306a09 cursor_preserved=1 dma_barrier=1 userspace=0
UI_INTERACTION_OK input=virtio-tablet events=14 samples=6 target=settings view=settings generation=1 completions=14 delivered=14 recycled=14 buffered=0 max_buffered=1..8 avail_idx=22 used_idx=14 irq_entries=>0 queue_irqs=>0 config_irqs=0 spurious=0 dropped=0 invalid=0 gesture=tap hit_test=1 surface_commit=1 damage=1 cursor_preserved=1
UI_SCREENSHOT_OK qemu=1 transport=ramfb input=virtio-tablet qmp_commands=6 events=14 samples=6 target=settings view=settings generation=1 damage=56/64/208/368 written=76544 restored=76544 blended=122 diff_pixels=53760 diff_bbox=56/64/263/379 baseline_sha256=0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac after_sha256=70720db4523aa6eb33746b76fcac314a99039b79d3aaee85422edb839bc2ee76 colors=13 scene_digest=0xf79f5bb3582452a5 scanout_digest=0xa2b4da7c5f306a09 max_buffered=1..8 irq_entries=>0 queue_irqs=>0 completions=14 delivered=14 recycled=14 avail_idx=22 used_idx=14 dropped=0 invalid=0 gesture=tap hit_test=1 cursor_preserved=1 full_frame_capture=1 userspace_surface=0 navigation_qmp_commands=15 navigation_events=35 total_events=49 total_samples=21 targets=3 home_returns=3 transitions=6 final_view=home
COMPOSITOR_READY layers=2 scene=opaque cursor=alpha scene_bytes=614400 scanout_bytes=614400 scene_address=... scanout_address=... page_aligned=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25 cursor_hidden=1 dirty_rect=1 dynamic_redraw=1 input_bound=0
VIRTIO_POINTER_OK transport=mmio version=2 device_id=18 base=0x000000000a003a00 queue=8 status_queue=present event_bytes=8 dma_frames=1 dma_coherent=1 mode=interrupt irq_enabled=1 irq=77 spec=0/45/1 trigger=edge-rising name_bytes=18 name_digest=0xc193f384ee1e1bac key_bitmap_bytes=43 abs_bitmap_bytes=1 abs_x=0/32767 abs_y=0/32767 btn_left=1 btn_touch=1 writable_buffers=8 completions=0 recycled=0 dropped=0 invalid=0
POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1
POINTER_EVENT_OK device=tablet sequence=abs_x/abs_y/syn/touch_down/syn/touch_up/syn events=7 raw_x=1234 raw_y=23456 pixel_x=12 pixel_y=342 touch_down=1 touch_up=1 syn=3 samples=3 completions=7 delivered=7 recycled=7 buffered=0 max_buffered=7 avail_idx=15 used_idx=7 irq_entries=3 queue_irqs=3 config_irqs=0 spurious=0 dropped=0 invalid=0 descriptor_reuse=1
COMPOSITOR_UPDATE_OK layers=2 updates=3 dirty=12/342/12/22 restored=264 blended=122 boot_digest=0x6ef9c2b7d15fde25 pressed_digest=0x61880733dfa7cfb1 final_digest=0x09f8448817d7a8e1 digest_changed=1 press_feedback=1 cursor_visible=1 cursor_pressed=0 alpha=1 dirty_rect=1 input=touch
COMPOSITOR_INTERACTION_OK qemu=1 transport=ramfb input=virtio-tablet qmp_commands=3 events=7 samples=3 redraws=3 dirty=12/342/12/22 restored=264 blended=122 diff_pixels=122 diff_bbox=12/342/21/363 baseline_sha256=0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac after_sha256=f24699248bcdfe5069fa13a40ef7990ba7a647cfe0ac818c04f671047e748f82 colors=11 boot_digest=0x6ef9c2b7d15fde25 pressed_digest=0x61880733dfa7cfb1 final_digest=0x09f8448817d7a8e1 max_buffered=7 irq_entries=3 queue_irqs=3 completions=7 delivered=7 recycled=7 avail_idx=15 used_idx=7 dropped=0 invalid=0 alpha=1 press_feedback=1 full_frame_capture=1
FRAMEBUFFER_OK transport=ramfb fw_cfg=mmio width=320 height=480 stride=1280 format=XRGB8888 bytes=614400 page_aligned=1 digest=0x6ef9c2b7d15fde25 configured=1
VIRTIO_INPUT_OK transport=mmio version=2 device_id=18 base=0x000000000a003c00 queue=8 event_bytes=8 dma_frames=1 dma_coherent=1 irq=78 spec=0/46/1 key_a=1 key_enter=1 writable_buffers=8 dropped=0 invalid=0
INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1
INPUT_EVENT_OK device=keyboard sequence=a_down/syn/a_up/syn events=4 key_code=30 down=1 up=1 syn=2 completions=4 delivered=4 recycled=4 buffered=0 avail_idx=12 used_idx=4 dropped=0 invalid=0 descriptor_reuse=1
FDT_VIRTIO_OK nodes=32 coherent=32 irq_specifiers=32 active_blocks=1
VIRTIO_IRQ_OK controller=gicv2 spec=0/47/1 irq=79 trigger=edge-rising target_cpu=0 enabled=1
VIRTIO_BLK_OK transport=mmio version=2 queue=8 request_slots=2 request_stride=536 dma_frames=2 dma_coherent=1 mode=interrupt irq_enabled=1 irq=79 capacity_sectors=16384 device_read_only=0 flush_supported=1 system_policy=read_only data_policy=bounded_write
BLOCK_IRQ_OK requests=2 completions=2 irq_completions=2 sectors=0/1 bytes=1024 avail_idx=2 used_idx=2 statuses=0/0 digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c batch_width=2 max_outstanding=2 distinct_heads=1 irq_observed=1 poll_fallbacks=0 out_of_range_rejected=1 timeouts=0 resets=0 stale_completions=0 queue_reused=1
BLOCK_LAYER_OK sector_size=512 device_sectors=16384 parser_reads=273 requests=282 completions=282 read_requests=280 write_requests=1 flush_requests=1 bytes_read=143360 bytes_written=512 successful_flushes=1 irq_completions=282 poll_fallbacks=0 timeouts=0 dma_frames=2
GPT_OK protective_mbr=1 primary_crc=1 backup_crc=1 entry_crc=1 entries=128 entry_bytes=128 primary_entries_lba=2 backup_entries_lba=16351 partition_index=0 partition_lba=2048-16350 partition_sectors=14303 name=BNDROID_SYS
DATA_GPT_OK partition_index=1 partition_lba=64-127 partition_sectors=64 name=BNDROID_DATA type=private system_overlap=0
FAT16_OK bytes_per_sector=512 sectors_per_cluster=1 reserved_sectors=1 fats=2 fat_sectors=56 root_entries=64 clusters=14186 first_data_lba=2165 mirror_verified=1
VFS_OK mount=/system root_file=/system/HELLO.TXT root_bytes=28 root_digest=0xdd2f71342016eede nested_file=/system/SYSTEM/BUILD.TXT nested_bytes=40 nested_digest=0xe57ce4ce9f1b4ec0 read_only=1 traversal_rejected=1 mount_escape_rejected=1
DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 slots=2 initial_generation=0 committed_generation=1 initial_slot=0 committed_slot=1 valid_slots=1 rejected_slots=1 reads=5 writes=1 flushes=1 write_completion=1 flush_completion=1 readback_verified=1 old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 crash_consistency=0 qemu_reboot_proof=0
STORAGE_LIMITS writes=1 partitions=2 filesystem=1 vfs=1 persistence=1 flush=1 readback=1 el0_storage=1 catalog_files=2 catalog_bytes=68 runtime_disk_io=0 mapped=0 shared_memory=0 filesystem_write=0 crash_consistency=0 general_runtime=0
PROCESS_IMAGE_OK abi=14 init=1 spawn_sequence=2/3/4/2/4 manager_reused=1 client_image_reused=1 distinct_catalog=1
SYSCALL_OK abi=14 calls=588 successes=455 errors=132 unknown=1 private_svc=1 should_waits=3 pan_fail=0
COPYIO_OK max=64 in_calls=185 out_calls=189 in_bytes=7415 out_bytes=16479 in_faults=14 out_faults=11 range_rejects=1 fixups=25 misses=0 ...
EL0_STORAGE_OK abi=14 root_capability=1 files=2 bytes=68 opens=2/6 vmo_reads=5/10 read_bytes=105 vmo_transfers=1/1 runtime_disk_reads=0 mapped=0 shared_memory=0 writes=0 persistence=0
BOOT_OK: M25 durable data records, M24 EL0 storage VMOs, and M20 multi-session services verified
```

> 独立 feature-gated race self-test 会故意抑制通知，让一批两个 token 超时；随后禁用 SPI、reset/re-negotiate/rebuild queue，使两个旧 token 失效，把模拟 late IRQ 只计为一次已 ACK 的 spurious notification，再用两个新请求恢复读取。它要求 DMA frame 数不变、没有 double completion，精确终态为：

```text
STORAGE_IRQ_RACE_OK timeout_requests=2 timeouts=1 resets=1 reset_tokens_invalidated=2 simulated_late_irq=1 spurious_acked=1 recovered_requests=2 recovered_completions=2 double_completions=0 dma_frames_before=2 dma_frames_after=2 digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c
BOOT_OK: M22 storage IRQ timeout/reset self-test recovered and M20 multi-session services verified
```

> M20 下层服务基线仍让 primary 与 secondary 两个 Client 常驻并分别拥有独立、generation-qualified manager session；Lookup 回复沿各自 session 路由，不使用 M18 的每请求 private reply Channel。固定 23 阶段 transcript 先让 secondary lease 1 的 txid `0x601` 停滞，再证明 primary 的 `0x602` 仍可完成；随后 manager 撤销 lease 1、provider 中止停滞请求，primary 又在 secondary detached 时完成 `0x603`；secondary 以 lease 2 重连，manager/client 两端都拒绝旧 lease 1 控制，并完成 `0x604`。最终 phase/errors=`23/0`、attach=`2/2`、revoke/stale/primary-progress bitmap 均为 `0x3`、provider accept/echo/abort=`4/3/1`、secondary echo=`1`、idle bitmap=`0x3`，且没有 Client crash/restart。

> M32 最终常驻集为 `init`、manager2、provider、primary、secondary、SurfaceServer、Launcher、App 共 8 个进程，created/exited/reaped/live=`9/1/1/8`、live images=`1/1/1/2/1/1/1`、handles by image=`4/5/3/4/4/1/2`（共 23）。M20 core Channel 图保持 16 endpoint/8 pair，另有两对 UI Channel，合计 20 endpoint/10 pair；Surface capability 唯一归 SurfaceServer，GraphicsBuffer 同一 generation-qualified object identity 只由 App producer 与 SurfaceServer consumer 以 `0x0f/0x09` rights 持有。等待项仍为 `4/2/2/2/3/1/1`，object/many/array pending=`7/2/3`。

> M20 服务 transcript 与 M29—M31 旧账本只保留为历史基线。M32 normal checker 精确固定 syscall `599/462/136`、copy calls/bytes `198/198` 与 `7835/17487`、Channel pair 48、duplicate/stale-close `190/74`、transfer `65/64`、ObjectWait/WaitArray calls `455/57`、23 handle，并到达 `BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified`。默认 workspace 377 项 host tests（`19/19/31/45/0/263`）、七镜像 build、Clippy、五种 normal QEMU、全部保留负测、11 条 storage negative、persistence/recovery、IRQ race 与 M32 UI screenshot/cancellation 均通过；完整 `./scripts/test.sh` 从头 exit 0，最终 `BNDROID_TEST_SUITE_OK` 固定 `qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。

> 上述 M32 数字是默认、feature-off normal 路径的认证基线，不应改写成 M33 数字。专用 `app-lifecycle-runtime` profile 才承载 M33：canonical 64-byte `ALC1` 生命周期消息、`UBP1` endpoint bootstrap 和 `USC1` Init↔Surface 控制均拒绝非规范 reserved bytes、sequence gap/replay、stale identity 与错误 transaction。真实运行态 QEMU 证据完成 7 个严格事务 `Launch1→Activate1→Suspend1→Resume1→Terminate1→Launch2→Activate2`，共 ALC1 `35` 条（7 Request、14 State、7 Command、7 Ack，其中 2 条 Launch Command 携带 App UI endpoint）和 USC1 `14` 条 command/ack。App1 graceful exit/reap 后，App2 复用同一 process slot 且 PID generation +1，最终常驻 Active。

> M33 profile 的精确收敛为 created/exited/reaped/live=`10/2/2/8`、29 handle、26 endpoint/13 pair，最终 8 个等待进程由 2 个 wait-many 与 6 个 wait-array token 覆盖；Surface app slot 绑定完整 `AppInstanceIdentity`，install/retire 同时处理 endpoint、buffer、focus/capture/frame/input，global frame 跨 App generation 单调。host suite 现为 406 项（`20/19/31/73/0/263`）；完整 `./scripts/test.sh` 从头 exit 0，汇总同时固定 `process_terminate=1 app_lifecycle=1`。这证明的是有界两代 lifecycle/window replacement，不是任意应用、任意故障下的产品 runtime。

> M34 专用 `app-crash-recovery-runtime` 已由 dedicated QEMU 验证 10 个严格事务：App1 完成 Launch/Activate/Suspend/Resume/graceful Terminate，App2 Launch/Activate 后意外退出，SurfaceServer 原子清除 endpoint、buffer、focus/capture/frame/input 并发送 `USC1 OwnerDied`，init 再启动并激活 App3。精确证据为 ALC1 messages/transactions/states/crashes=`46/10/19/1`，USC1 commands/acks/owner-deaths=`9/9/1`；App PID generation 为 `1→2→3`，created/exited/reaped/live=`11/3/3/8`，termination reasons=`2/0/1`，terminate accepted/completed=`1/1`。最终仍为 29 handle、26 endpoint/13 pair、8 个 exact wait token，object/many/array pending=`8/2/6`、termination-abandoned=`0/0/1`，GraphicsBuffer generation=`3`。这是 ABI-v18/M34 历史专项证据；M42—M70 各自历史封口保持不变，历史 M71 里程碑为 ABI-v32/M71 continuous supervision，字面 kernel chain 从 M71→M70→M69→M68→M67→M66→…→M55。

> M35—M40 依次证明 mapped BufferQueue、consumer abandon、server restart、producer orphan、software frame grant 与双槽 swapchain；M41/M42 补固定两窗口策略与 persistent boundary/peer-close/recreate；M43—M48 再依次补有界文本/软键盘、独立 InputServer、两类单次 restart 与单 InputServer watchdog/quarantine；M49 补固定 SurfaceServer+InputServer 的单 soft-edge 依赖恢复，M50/M51/M52 再补恢复后的 App input-to-frame、App→Launcher focus/frame 与 Launcher→App focus roundtrip，M53 补双客户端 session-2 lifecycle-focus 实时同步与独立 APCK terminal boundary。当前仍没有硬件 vblank/pageflip、DMA-BUF/IOMMU、任意规模的通用多服务监督、任意 window manager、产品级 InputServer、完整 IME/font shaping/Unicode、多点或真实硬件触屏。电话/蜂窝、Wi-Fi、音频、硬件网络、电源、产品级驱动、安全启动、完整沙箱、安全更新、通用 App 生态与可用产品 UX 均未完成，绝不能称为现实可用的手机系统。

## 1. 项目定位

Bndroid OS 是一个面向移动设备的自研操作系统。它采用 Rust 优先的工程路线，主系统使用非 Linux 内核，整体架构参考现代移动系统中更安全、更统一、更可控的设计方式。系统目标是提供类似高端移动系统的整体体验，同时通过 Android Compatibility Subsystem 支持 Android 应用运行。

这个项目的长期目标是：构建一个自研微内核或混合内核，建立统一的系统服务层，提供原生应用框架，设计严格的应用沙箱和权限系统，最终通过兼容子系统运行 Android APK。更好的路线不是简单复制某个现有系统，而是吸收优秀系统的架构思想，用 Rust 实现更安全、更清晰、更容易维护的系统基础设施。

## 2. 最佳技术方向

推荐采用 Rust-first 架构：

- Kernel：Rust + 少量 AArch64 Assembly
- System Services：Rust
- Driver Framework：Rust
- IPC/IDL：Rust 代码生成
- Package Manager：Rust
- Permission Manager：Rust
- Window Server：Rust
- Compositor：Rust + Vulkan/Skia/wgpu 路线
- Android Compatibility：AOSP 组件 + Rust bridge
- Native SDK：Rust + 声明式 UI
- Build System：cargo xtask + Ninja/GN 或 Bazel
- Emulator：QEMU ARM64

Rust 更适合这个项目的原因：

- 内存安全更好，适合系统服务和内核开发。
- 类型系统强，适合定义 IPC、权限、资源句柄。
- 并发安全更好，适合多服务架构。
- 工程组织清晰，适合长期维护。
- 能和 C ABI、AOSP、Vulkan、Skia 等组件对接。

## 3. 系统总体架构

```text
Applications
├── Native Rust Apps
├── Android Apps
└── Web Apps

Frameworks
├── Native UI Framework
├── Android Compatibility Framework
├── Media Framework
└── Web Runtime

System Services
├── AppManager
├── PackageManager
├── PermissionManager
├── WindowServer
├── Compositor
├── InputServer
├── AudioServer
├── NetworkServer
├── CameraServer
├── LocationServer
├── NotificationServer
├── PowerServer
└── UpdateServer

Runtime
├── Rust Native Runtime
├── Android Runtime ART
├── Bionic Compatibility
└── Web Runtime

Security
├── Code Signing
├── App Sandbox
├── Capability Tokens
├── Secure Storage
└── Audit Log

Kernel
├── Scheduler
├── Virtual Memory
├── IPC
├── Handle Table
├── Driver Interface
├── Resource Control
└── Linux ABI Compatibility Subset
```

## 4. 内核路线

更好的第一阶段路线是做一个小而完整的 Rust 微内核。内核只负责最关键的能力：

- 线程调度
- 地址空间
- 进程对象
- Handle 表
- IPC Channel
- Timer
- Interrupt
- Shared Memory
- Driver Host 接口
- Capability 权限

第一版内核能在 QEMU ARM64 启动即可。它需要完成：

- AArch64 启动入口
- 页表初始化
- 串口日志
- 物理页分配器
- 虚拟内存映射
- 线程切换
- 简单调度器
- 系统调用入口
- IPC Channel
- 用户态 init 进程

更好的设计是把驱动、文件系统、网络、图形尽量放到用户态服务，内核保持小核心。这样系统安全边界更清楚，服务崩溃后更容易恢复。

## 5. 系统服务路线

系统服务全部用 Rust 写，并通过统一 IPC 协议通信。每个服务都有明确权限，应用不能直接访问硬件。

优先实现服务：

- Init：启动用户态系统。
- ServiceManager：管理服务注册与发现。
- LogServer：集中日志。
- PackageManager：安装原生应用和 Android APK。
- PermissionManager：管理权限授权。
- AppManager：启动和管理应用生命周期。
- WindowServer：窗口、焦点、层级。
- Compositor：画面合成。
- InputServer：触控、键盘、手势。
- NetworkServer：网络访问。
- AudioServer：音频播放和录音。

更好的系统服务原则：

- 服务接口版本化。
- 所有敏感接口都需要 capability。
- 服务崩溃可重启。
- 系统关键服务有审计日志。
- 服务之间通过 IDL 自动生成客户端和服务端绑定。

## 6. 原生应用框架

Bndroid OS 需要自己的原生应用生态，推荐使用 Rust 声明式 UI。原生应用包格式为 `.bapp`。

```text
Example.bapp
├── manifest.toml
├── signature
├── bin/main
├── assets
├── resources
└── localization
```

manifest 描述：

```toml
id = "com.example.demo"
name = "Demo"
version = "1.0.0"
entry = "bin/main"

[permissions]
network = true
camera = false
microphone = false
location = "while_in_use"
notifications = true
```

原生应用生命周期：

- Launching
- Active
- Inactive
- Background
- Suspended
- Terminated

更好的设计是让系统统一控制后台能力。后台音频、定位、下载、通知都通过系统授权和调度执行。

## 7. Android 应用兼容

目标架构计划让 Android 应用通过 AndroidBox 运行。AndroidBox 将是受控兼容子系统，
不是主系统本身；当前除 DEX-0、Activity-0 与 Resources-1 预备切片外，已完成原始
`org.bndroid.demo` Install-0，以及同一严格 Resources-1 shape 内另一个本机构建、
不含固定 probe 的 `org.bndroid.macdemo` 安装与恢复；受限 Update-0 仍只覆盖原始
demo 的同包、同 signer、版本单调增加 v2→v3 对。该子集可持久安装、原子切换到
generation 2、无源恢复，并以纯数据 ActivityLifecycle-1 解释两指令 public no-argument
constructor 后的受限 `onCreate`，但仍没有 ART/Bionic/Binder/JNI、Android Framework、
网络、多包生命周期、通用更新/卸载或通用 APK 兼容。

```text
Android APK
↓
Android Framework Shim
↓
ART + Bionic
↓
Binder Compatibility
↓
Linux ABI Compatibility Subset
↓
Bndroid System Services
```

完整 AndroidBox 的未来第一阶段计划支持简单 APK：

- APK 解析
- AndroidManifest 解析
- DEX 加载
- ART 启动
- Bionic 基础适配
- Binder service manager
- Activity 生命周期
- Surface 映射
- 触控事件映射
- 网络 socket
- 应用私有存储

第二阶段支持常见工具类应用：

- NotificationManager
- AudioTrack
- AudioRecord
- WebView
- SQLite
- SharedPreferences
- JobScheduler
- Clipboard
- Basic Camera2

第三阶段支持复杂应用：

- MediaCodec
- OpenGL ES
- Vulkan
- HardwareBuffer
- Camera2 完整桥接
- Push 替代服务
- Location 替代服务
- Play Services shim 或 microG 类接口

## 8. 图形系统

图形系统建议采用自研 Compositor + Vulkan 路线。第一阶段可以使用 framebuffer，第二阶段接入 virtio-gpu，第三阶段接入真机 GPU。

图形组件：

- SurfaceManager
- WindowServer
- Compositor
- AnimationEngine
- RenderScheduler
- ColorManager
- ScreenshotService

Android 图形兼容需要支持：

- Surface
- SurfaceView
- TextureView
- BufferQueue
- EGL
- OpenGL ES
- Vulkan
- Choreographer
- VSync callback

更好的实现方式是所有应用只提交 Surface，最终由系统合成器统一合成。这样可以统一动画、截图权限、防录屏保护、刷新率和性能调度。

## 9. 安全模型

Bndroid OS 应采用强安全模型：

- 所有应用必须签名。
- 所有应用运行在独立沙箱。
- 所有敏感权限由系统服务授权。
- 所有系统服务最小权限运行。
- 所有 IPC 调用可审计。
- 系统分区只读。
- 用户数据加密。
- OTA 包签名验证。

Rust 可以让系统服务和内核对象模型更安全。更好的安全实现是 capability-based security：应用拿不到能力句柄，就无法调用对应系统服务能力。

## 10. MVP TODO

### M0：工程启动

- [x] 创建 Rust workspace。
- [x] 配置 `no_std` kernel crate。
- [x] 配置 AArch64 target。
- [x] 配置 QEMU 启动与验收脚本。
- [x] 实现串口输出。
- [x] 建立项目 Markdown 文档以及 kernel、services、sdk、androidbox 目录。

### M1：内核启动

- [x] AArch64 boot。
- [x] 页表。
- [x] 物理内存管理。
- [x] 虚拟内存管理（kernel ASID 0；每进程私有 TTBR0；init ASID 1；child 动态 ASID；通用有界 VMA/frame 集合逐页验证、失败回滚，TLBI 后回收与复用）。
- [x] 异常向量。
- [x] 系统调用（feature-off default/M53 与历史 M59 `storage-irq-timeout-self-test` 均为 ABI v23：保留有界 Channel/Event/wait、image-selecting spawn、syscall 20 `ChannelPeek`、21 原子 `ChannelReadEnvelope`、22 `ObjectWaitManyArray`、23 `FileOpenAt`、24 `VmoRead`、25/26/27 Surface API、28/29/30 GraphicsBuffer、31 `ProcessTerminate`、ABI-v19 syscall 32—36 map/unmap/queue/acquire/release、ABI-v20 signal bit 4 `FRAME_READY`/syscall 37 `SurfaceFrameAcquire` 与 ABI-v21 signal bit 5 `KEY_READY`/syscall 38 `SurfaceReadKey`；ABI-v22 复用 `READABLE`/`PEER_CLOSED`，新增 syscall 39/40，ABI-v23 新增 syscall 41。M54 `app-data-runtime` 与历史 M59 child `app-data-async-recovery-runtime` 使用 ABI v24/syscall 42—46 且不含 StorageServer；M55—M64 StorageServer profiles 使用 ABI v25/syscall 47—52，并使 42—46 返回 `Unsupported`；M65 使用 ABI v26/syscall 53；M66 使用 ABI v27/syscall 54；M67/M68/M69/M70/M71 分别使用 ABI v28/v29/v30/v31/v32 且均不新增 syscall；M72 使用 ABI v33/init-only syscall 55 `ServiceManifestOpen`；M73 使用 ABI v34/init-only syscall 56 `ServiceSupervisorReport`；M74 使用 ABI v35 且不新增 syscall。ABI-v26 raw 54、ABI-v32 raw 55、ABI-v33 raw 56、ABI-v34 与 ABI-v35 raw 57 都返回 `UnknownSyscall`）。
- [x] 线程（feature-off default 为 3 个固定 EL1 context、EL0 `init` 与七个 dynamic child context，共 11 context；M45—M54 与历史 M59 AppData leaf 为八 dynamic child 与 12 context；M55—M65 StorageServer profiles 保持有界 11-context；M66/M67 为 9 个 dynamic child、合计 13 context。M66 heap 为 512 KiB；M67 专用 heap 为 1 MiB、worker exception stack 为 64 KiB。历史 M59 timeout self-test 保持 ABI-v23 低层有界配置。每个 child 独占用户/内核栈和地址空间；Zombie/Faulted 延迟回收；容量仍非通用）。
- [x] 调度器（当前单核 round-robin；七个 EL0 ObjectWait slot 以 epoch/PID/handle generation 防止 stale wake；允许宿主超售造成的零指令 selection coalescing，并在 busy/sleep checkpoint 等待 `observed==selected`；signal/timeout 以单次 counter 采样仲裁 exact token）。
- [x] IPC Channel（容量 8 的 typed scalar/byte/transfer FIFO；move-only generic-object transfer、失败 rollback 与 dead-receiver drain 保持成立；`READABLE`/`WRITABLE`/`PEER_CLOSED` level signals 接入阻塞式 `ObjectWait`，mutation commit 后在锁外扫描唤醒；非零单调 Channel ID 形成保守所有权 DAG，拒绝 self/equal/reverse 边）。
- [x] Event（ABI v10 保留的 manual-reset `SIGNALED` 对象；signal/clear 幂等、可等待、可跨进程转移，并覆盖失败 rollback、未读销毁与直接阻塞唤醒）。
- [x] 固定两项 wait-any + 相对 timeout（ABI v10 保留 syscall 19；poll/infinite/finite、全参数验证、最低 ready index、硬 deadline 与 signal/timeout exact-token 竞争均已验收）。
- [x] 有界 1—8 项 wait-any 数组（ABI v16 保留 syscall 22；8-byte canonical LE item、最多 64-byte usercopy、copy+全项 validation-before-ready、最低 index、真实八项 block/wake、显式 completion kind 与 exact token）。
- [x] M21（历史）QEMU modern virtio-mmio v2 只读块设备：FDT 固定容量发现、`VERSION_1 | RO` feature negotiation、queue 8、两个 coherent DMA frame、顺序轮询读取 sector 0/1、越界提交前拒绝，以及 legacy/corrupt/no-device 三条负路径。
- [x] M22 virtio-blk IRQ completion 与两请求并发：FDT `interrupt-parent`/GICv2 resolve、SPI INTID 79 trigger/CPU 0 target、两个 536-byte slot（head 0/3）、真实 two-outstanding、hard-IRQ used-ring completion，以及 timeout/reset/token invalidation/late-spurious/recovery 自测。
- [x] M23 allocation-free block boundary、protective MBR/主备 GPT CRC、FAT16 双镜像与有界簇链、`/system` 单只读 VFS；142 parser/144 total IRQ reads，六类 storage negative。
- [x] M24 immutable `BootfsCatalog<2>`/VMO、move-only directory root、capability-scoped EL0 open/read，以及完整内容/FNV/partial/EOF/bounds/bad-address/attenuation/stale/zero-payload-transfer 证明。
- [x] M25 private DATA GPT entry、format-epoch-bound 双 CRC record、bounded raw WRITE/FLUSH/readback、跨 QEMU `0→1→2` 与 newest-slot corruption recovery；269 项 host tests、11 类 storage negative。
- [x] M26 strict fw_cfg DMA/ramfb 与 virtio-input keyboard：320×480 XRGB8888 九色
  static splash、queue 8/one DMA frame、QMP screenshot 与 A-down/SYN/A-up/SYN 验收；283 项 host tests。
- [x] M27 opaque scene + alpha cursor compositor、dirty redraw、独立 virtio-tablet 与 7-event/3-sample interaction test；297 项 host tests。
- [x] M28（历史）kernel-owned clickable shell、三个 app target + home、Settings tap/damage 与 14-event/6-sample screenshot；323 项 host tests。
- [x] M29 protocol+raster/session 地基：canonical 64-byte `bndr-ui` 与先全验证后 raster 的纯 `SurfaceSession`。
- [x] M29 运行时接入：ABI v14 syscall 25/26/27、EL0 唯一 capability/session、真实 `KernelFallback→UserspaceBound` 单向 ownership、ServiceManager 内嵌 SurfaceServer 与独立 input event 分类；337 项 host tests。
- [x] M30 运行时拆分：ABI v15 六镜像 catalog、独立 SurfaceServer/Launcher、单独 UI Channel pair、七进程/19-handle 常驻拓扑；349 项 host tests。
- [x] M31 independent App/focus routing：ABI v16 七镜像 catalog、独立 SurfaceServer/Launcher/App、两对 UI Channel、八进程/21-handle 常驻拓扑；`BUC1` client-only attach、focus generation、Present v2 cancellation/同帧重试与按手势 capture 已固定。
- [x] M32 transferable GraphicsBuffer：ABI v17 两槽 XRGB8888、`BUP1`、rights attenuation/transfer、generation/scrub、失败原子性与真实 App raster/copy-present；23-handle 拓扑、377 host tests 和完整矩阵已通过。
- [x] M33 app lifecycle/minimal window management：ABI v18 canonical 64-byte `ALC1`/`UBP1`/`USC1`，7 个严格事务、App1 graceful exit/reap、App2 同槽下一 generation；专用 profile 收敛为 35 ALC、14 USC、`10/2/2/8` process、29 handle、26 endpoint/13 pair 与 object/many/array pending=`8/2/6`，host suite 406（`20/19/31/73/0/263`）。
- [x] M34 App crash/owner-death recovery：专用 `app-crash-recovery-runtime` 以 10 事务、ALC1 `46/10/19/1`、USC1 `9/9/1`、PID generation `1→2→3` 和 process `11/3/3/8` 验证 App2 peer-close 清理与 App3 同槽重启；dedicated QEMU 与完整回归矩阵均已通过。
- [x] M35 mapped/shared GraphicsBuffer：App RW/SurfaceServer RO 共享 75 页，单槽 Queue/Acquire/Release、BBM+TLBI 与 EL0 volatile sample read 已由专用 QEMU 验证。
- [x] M36 graphics consumer owner-death cleanup：consumer leaves/TLBI 先于 producer BBM RO→RW 与 release；专用 QEMU 验证 Acquired death、非零 wake、75 页重写与零最终 graphics mapping/handle，Queued 分支由 host test 覆盖。
- [x] M37 SurfaceServer restart/rebind：generation-2 server reacquire session 2，Launcher 与 resident mapped App 更换 endpoint，generation-2 mapped frame 由 replacement present；专用 QEMU 与完整从头矩阵均已通过。
- [x] M38 producer-death orphan/two-slot reuse：App→SurfaceServer terminate/wait ordering、两个完整 307200-byte backing scrub/zero proof、generation-3 双槽复用、第三分配 OOM 与零终态 graphics 资源已由专用 QEMU/checker 验证；host suite 418 与完整矩阵均已通过。
- [ ] 超过 8 项/任意长度 wait-any、wait-all、显式 cancel、高精度 timer 与 SMP-safe 对象等待。

### M2：用户态系统

- [x] 七个独立、pairwise-distinct AArch64 `ET_EXEC` 与内核 all-or-none catalog；内核 process capacity 为 8（init + 7 dynamic），支持显式 startup move、独立 address space/HandleTable/用户帧/内核栈及 manager 同槽 generation +1 重启；`App=7` 是独立镜像。
- [x] 独立受监督 ServiceManager 原型（manager1→manager2、跨故障重绑；M16 按 kind/opcode 通用分派并完成一次 post-cleanup 有界复用，精确 graph/wait token 持续稳定）。
- [x] M17：完成 kernel-stamped caller identity、manager 固定 PID ACL、delegated-endpoint 对抗路径及 48-byte reducer 的排列/恢复/1,024 轮测试。
- [x] M18（历史）：完成同一 client 镜像的 primary/secondary 同时 outstanding Lookup、共享 manager ingress、每请求 private reply、provider armed barrier，以及 secondary exit/reap 后回到 1M/1P/1C；该阶段仍是 `bounded_slice=1 general_runtime=0`。
- [x] M19（历史）：在不改变 M18 服务拓扑的前提下加入有界八项 `ObjectWaitManyArray`，ABI 升至 v12；完整 `./scripts/test.sh` 以 183 项 host tests、双 Clippy、五种 normal QEMU 和全部 negative/rollback/fault checks 收口。
- [x] M20：把两个 Client 变为常驻的独立 manager session，回复沿 session 路由；完成一次 stalled-secondary 隔离、lease revoke/reattach、stale-lease 双端拒绝和 detached-primary progress，最终收敛到五进程、16 endpoint/8 pair、四个 generation-qualified wait token。完整回归矩阵已通过，但仍是固定 23 阶段 `multi_session_round=1 general_runtime=0`，不是产品级 resilient daemon。
- [x] M21：在不改动 M20 服务账本与最终拓扑的前提下，加入 QEMU modern virtio-mmio v2 只读轮询块设备和确定性双 sector 证据；199 项 host tests、双 Clippy、五种 normal QEMU 与完整负测矩阵从头通过。
- [x] M22：把 M21 块设备改为 GICv2 SPI 驱动的固定两槽并发队列，正常路径无 polling fallback；加入 timeout/reset、旧 token 失效、late IRQ 隔离与恢复证据。208 项 host tests、双 Clippy、五种 normal QEMU、三项存储负测、race 自测及其余完整矩阵从头通过。
- [x] M23：以固定 8 MiB 镜像验证双份 GPT、FAT16 与 `/system` 只读 VFS，读取根/嵌套两个文件；226 项 host tests，storage negative 扩到六类，并把 scheduler no-switch 注入显式 arm 到 storage/heap baseline 之后。仍为 `general_runtime=0`。
- [x] M24：ABI v13 以 move-only root capability 打开固定两文件为 immutable VMO；248 项 host tests 与完整矩阵从头通过，运行期磁盘读取、mapping/shared memory、write/persistence 仍全为 0。
- [x] M25：DATA-bounded raw record durability、WRITE→FLUSH→readback 与跨重启 generation/recovery 证明；269 项 host tests，仍为 `filesystem_write=0 crash_consistency=0 general_runtime=0`。
- [x] M26：fw_cfg DMA/ramfb static splash 与 virtio-input keyboard event path；
  `check-framebuffer.sh`/`check-input.sh` 及 283 项 host tests 通过，仍无产品图形/输入栈。
- [x] M27：双层软件 compositor、dirty redraw、virtio-tablet touch 与前后全帧 interaction proof；
  `check-compositor.sh` 及 297 项 host tests 通过，仍无产品图形/输入栈。
- [x] M28（历史）：kernel-owned clickable shell、Settings generation-1 damage commit 与 full-frame UI proof；
  `check-ui.sh` 及 323 项 host tests 通过，当时为 `userspace_surface=0`。
- [x] M29：EL0-owned single Surface、输入 FIFO/coalescing、degraded owner-death 与 ramfb/headless 同构路径；337 项 host tests，完整套件为 `userspace_surface=1 userspace_ui=1`。
- [x] M30：独立 EL0 SurfaceServer + Launcher，以单独 UI Channel pair 交换 frame/event；最终七进程、18 endpoint/9 pair、19 handle，349 项 host tests，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M31：独立 EL0 App 与第二对 UI Channel；显式 focus、按焦点 input、press-to-release capture、Present v2 generation/cancellation；最终八进程、20 endpoint/10 pair、21 handle，默认/全 feature check/Clippy、五种 normal QEMU、headless storage IRQ、heap rollback、全部负测、24-input/13-commit baseline 与连续两次 36-input/19-commit cancellation/retry stress 通过，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M32：transferable GraphicsBuffer/buffer-present、真实像素 App raster、24-input/13-commit buffer baseline 与 36-input/18-commit cancellation retry；完整 `./scripts/test.sh` 从头 exit 0。
- [x] M33：专用 `app-lifecycle-runtime` 完成 Launch/Activate/Suspend/Resume/Terminate/relaunch、generation-safe App endpoint replacement 与最小 focus/window ownership；默认 M32 normal 路径仍保留其独立认证账本。
- [x] M34：专用 `app-crash-recovery-runtime` 完成 App2 owner-death 通知、资源清理、Launcher focus fallback 和 App3 generation-safe restart；29 handle、26 endpoint 与 8 个 exact wait token 守恒，dedicated QEMU 与完整 `./scripts/test.sh` 均已通过。
- [x] M35：专用 `mapped-graphics-runtime` 完成共享 75-page data plane、单槽 BufferQueue 与 acquire/release fence。
- [x] M36：专用 `graphics-owner-death-runtime` 完成 stopped SurfaceServer alias teardown、producer RW 恢复、owner-death release/wake 与 App 显式清理；display 仍为 Degraded。
- [x] M37：专用 `graphics-surface-restart-runtime` 完成一次 SurfaceServer generation-2 restart、双 client rebind 与 resident App mapped frame 恢复；不是通用 watchdog/retry policy。
- [x] M38：专用 `graphics-producer-orphan-runtime` 完成一次 producer orphan reclamation 与 two-slot scrub/reuse；不是 frame-paced multi-buffer swapchain。
- [ ] LogServer。
- [ ] 文件系统服务。
- [ ] PackageManager。
- [ ] PermissionManager。
- [ ] AppManager。

### M3：图形和输入

- [x] M26--M28 历史 kernel-owned QEMU ramfb、双层 compositor 与 clickable shell；M29 完成 userspace-owned 单 Surface，M30 完成 SurfaceServer/Launcher 进程拆分，M31 增加独立 App/focus/input routing。
- [ ] WindowServer。
- [ ] 完整用户态 Compositor（已封口持久 capacity-2 userspace policy、严格边界与 generation-safe recreate，但任意窗口、自动 restart、硬件 vblank 与重复 crash policy 仍未完成）。
- [x] M45 bounded InputServer（独立服务、唯一 InputCapability、kernel broker、broker-only route/focus/capture/text-context/IME）。
- [x] M47 bounded InputServer 一次 restart/reacquire、fixed 30 ms backoff、Surface resync 与 scoped `InputAcquire` permission audit。
- [x] M48 `ServiceSupervisor::<1>` + strict `BSH1` health/watchdog；首个 process-exit 重启、第二次 health-timeout、budget 1、runtime quarantine 与 degraded UI 已由专项 QEMU 验证。
- [ ] 产品级 InputServer（M49—M53 仍只覆盖固定 SurfaceServer+InputServer、单 soft edge、固定双客户端同步与脚本交互；仍无通用多服务恢复、完整 IME/candidate/locale/font shaping、任意 Unicode、多点或物理设备栈）。
- [x] M32 transferable GraphicsBuffer/buffer-present。
- [x] M33 有界 app lifecycle 与最小窗口管理（专用 profile）。
- [x] M34 有界 App crash/owner-death recovery（专用 profile；完整矩阵已通过）。
- [x] M35 共享映射 GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；App RW/SurfaceServer RO 共享 75 页，416 host tests 与完整矩阵通过。
- [x] M36 consumer owner-death abandon/release + producer recovery；Acquired QEMU 和 Queued/Acquired host coverage 已通过。
- [x] M37 SurfaceServer 自动重启/rebind + resident mapped App 帧恢复；专用 QEMU 已通过。
- [x] M38 producer-death orphan + two-slot scrub/reuse；dedicated QEMU strict validator 与 checker 已通过。
- [x] M39 ABI-v20 software frame clock、`FRAME_READY`/syscall 37 与三次 grant-gated mapped commit。
- [x] M40 resident two-buffer swapchain ownership/scheduling、ABA commit 与 post-copy release。
- [x] M41 bounded userspace multi-window compositor：z-order、occlusion、damage、raise、focus 与 input capture。
- [x] M42 持久事件驱动窗口会话、phone 边界拒绝、peer-close 清理、generation-safe App 重建与通用事件序列。
- [x] M43 focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44 SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45 独立 bounded InputServer、唯一 InputCapability、capacity-64 broker 与 Surface FIFO 零 fallback；dedicated QEMU、三截图、623 host/334 kernel 与完整总套件均已通过。
- [x] M46 SurfaceServer restart/rebind、route epoch `1→2`、gap release、App capture/contact cancel；dedicated QEMU、M45 regression、650 host/335 kernel 与完整总套件均已通过。
- [x] M47 InputServer 自身 restart/reacquire、route resync、fixed 30 ms backoff/budget 1、scoped audit/permission denial；dedicated QEMU、674 host/338 kernel 与完整总套件均已通过，quarantine 仅 host-verified。
- [x] M48 单 InputServer `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、process-exit 重启、第二次 health-timeout、budget-1 runtime quarantine、degraded UI 与 strict trace/validator/topology；694 host/338 kernel，dedicated 四阶段 QEMU、静态矩阵与最终全量 `./scripts/test.sh` 均已通过。
- [x] M49 `ServiceSupervisor::<2>` 依赖感知 SurfaceServer+InputServer；`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。
- [x] M50 保持 ABI-v23/syscall 0—41/八镜像/capacity 不变，完成恢复后 phone 外拒绝、App physical `6/7`→两条 BWE→Present sequence 5/frame 3→output frame 4/write generation 4→health `2/2` resident 的 QEMU 封口。
- [x] M51 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，继承 M49/M50 后以 physical `8/9`、screen `(80,96)`、BIC sequence 3 完成 Launcher capture、focus `App/3→Launcher/4`、Present command 6/frame 4/scene 11/output 5/write generation 5 与 health `2/2` resident；全量 suite 已固定 `post_recovery_focus=1`。
- [x] M52 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，继承完整 M49/M50/M51 前缀后以 App physical `10/11`、BIC sequence 4 完成 focus `Launcher/4→App/5` roundtrip、Present command 6/frame 4/scene 12/output 6/write generation 6；全量 suite 已固定 `post_recovery_focus_roundtrip=1`。
- [x] M53 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，以 M52 为 feature parent，在 session 2 完成双客户端 `Ready/App1/RFCK→Launcher2/LFCK→App3/AFCK` 同步；Channel `14/14`（BUE 8 + ACK 6），M52 APCK 独立 boundary `1/1`，physical `1..11`、event 17、Input `App/4`、compositor `App/5`、output 6 与 framebuffer 均不新增；全量 suite 已固定 `post_recovery_lifecycle_focus=1`。
- [x] M54 opt-in ABI-v24 AppData 完整封口：GPT index 2/LBA `128-2047`、single principal 1、固定容量 capability API、fresh `0→2`、upgrade `2→3`、stable generation 3 零写、corrupt-newest fallback 2/re-upgrade 3；22 项 host tests、四次 offline/`-nic none` QEMU checker 与完整总 suite 均已 exit 0，最终 marker 含 `app_data_runtime=1`，QEMU 仍为 `powercut_claim=0`。
- [x] M55 独立 ABI-v25 StorageServer：standalone ELF、syscall 47—52、strict v2 batch transport、kernel raw-sector broker、idle restart/rebind 与三启动 `0→4→5→5` 已封口。
- [x] M56 历史 ABI-v25 fail-stop recovery：三类 session-fatal timeout、old-owner teardown、kernel reset/rebuild/rearm 与 epoch+1 remount 已封口。
- [x] M57 历史 ABI-v25 repeated recovery：两轮串行 `WRFWRF`、suppression `2/2/2`、IRQ-safe broker access、Running-owner `ServiceAbandoned`、physical submission gate 与一次 IRQ commit rollback 后成功 retry 已封口。
- [x] 历史 M58 ABI-v25 StorageServer 分支前身：四相位跨调度推进七次 physical recovery，driver borrow 跨相位释放，每个 cooperative step 对 reset status 或带 generation 的 capacity tuple 至多采样一次；async coordinator 持有 rearm/broker/admission policy commit，七个 timer/双 worker/已认证 EL0 progress window、零 masked poll，以及被计量的 driver/control 区间均短于 timer period已封口；这不是全内核 DAIF 时长证明。
- [x] 历史 M59 cooperative unification 双账本：ABI-v24 `app-data-async-recovery-runtime` 对一次 read fault 实测 steps/pending/yields=`4/3/5`、authenticated waits/dispatch changes=`2/2`、mask `53500/86187<625000`；ABI-v23 `storage-irq-timeout-self-test` 实测 steps/yields=`4/3`、timer/worker windows=`3/3`、dispatches `9`、workers `539135/376383`、mask `59375/87188<625000`，且 `el0_progress_claim=0`。共享引擎已统一，但两账严禁拼接。
- [x] 历史 M60 ABI-v25 fault policy：`WRFWRFR`、attempt cap 3、2×2 backoff、Probation/Healthy 与 boot-local sticky Offline 已封口。
- [x] 历史 M61 ABI-v25 fault-latched owner liveness：7 张不可续期 250 ms physical-counter ticket、6 次合作退场、1 次 epoch-6 flush forced retirement、真实 `ObjectWait` abandonment、普通 reaper、epoch-7 replacement 与后续 Offline 已封口；parser/policy/mismatch=`184/9/6`。
- [x] 历史 M62 ABI-v25 terminal quarantine：一次 kernel-only proof deferral、三步/two-Pending fallback、一次模拟 physical error、reverified IRQ/DMA Offline boundary 与零 attempt/ticket/EL0 控制已封口；parser/policy/mismatch=`204/8/6`，host=771，48/48 launch 各恰有一个 `-nic none`。
- [x] 历史 M63 ABI-v25 persistent unclosed-boot hint/reprobe：80-byte 双槽 payload、legacy upgrade、同一镜像双启动、fresh current-device probe、default compatibility、persisted/record-derived Offline 与 EL0 controls 为 0。
- [x] 历史 M64 ABI-v25 clean close/no-later-storage：exact open-session capability、两次 clean close、prior-closed 第二启动、pre-EL0 admission/IRQ seal、post-close storage mutation=0；parser/persist/mismatch=`93/31/4`，feature-kernel=368。
- [x] 历史 M65 ABI-v26 bounded shutdown orchestration：init-only Prepare/Commit、两个 client drain、post-Prepare spawn rejection、StorageServer final flush/readback/normal exit/reap、M64 durable close 与 storage/IRQ/shutdown seal；parser/mismatch/host/ABI=`133/4/4/1`，feature-kernel=372。
- [x] 历史 M66 ABI-v27 resident platform shutdown：八个认证 resident 节点、10 条依赖、三波逆拓扑 quiesce、kernel object-table topology proof、九 child exit/reap、StorageServer final flush/readback、opaque fail-closed platform token 与两次 QEMU semihosting 自退出；feature ABI/kernel=`37/419`，parser/mismatch/service/platform/ABI=`144/4/4/2/2`。
- [x] 历史 M67 ABI-v28 unified product runtime：真实 M45 UI/InputServer convergence、认证 power key 116、Launcher/App AppData、final StorageServer、M66 shutdown closure、双启动 exact screenshot/disk/self-exit，明确 `general_runtime=0 real_phone_claim=0`。
- [x] 历史 M68 ABI-v29 bounded product-service liveness：BSH1 `3/2/1` probe/Healthy/withheld、100 ms timeout、30 ms backoff、restart budget 1、同 slot generation + 1、一次 killed replacement、replacement mount/Healthy 与两次完整自退出均已封口。
- [x] 历史 M69 ABI-v30 bounded two-service dependency liveness：固定 StorageServer+App、一条 hard edge、BSH1 `8/7/1`、三次 40 ms cadence、App block/resume ACK、100 ms timeout、30 ms backoff、同槽下一代 replacement 与最终双服务 Healthy 均已封口；不新增 syscall。
- [x] 历史 M70 ABI-v31 QEMU PSCI shutdown：strict `/psci`、compatible/method、PSCI_VERSION、opaque kernel seal、HVC SYSTEM_OFF、无 semihosting与两次 QEMU self-exit 均已封口；不新增 syscall，不声称 PMIC/hardware poweroff。
- [x] 历史 M71 ABI-v32 五服务连续监督：事务式 5-service/4-edge 目录与批量 probe、21 个 cadence 轮/16 个额外健康轮、同窗两服务瞬态 miss 容忍与独立恢复、一次超限 StorageServer replacement，以及两次 PSCI QEMU self-exit 均已封口；不新增 syscall，明确 `arbitrary_soak_claim=0 real_phone_claim=0`。
- [x] M72—M75：immutable BMF1、事件监督、外部 BMS1 签名门和双槽持久 rollback floor 已依次封口。
- [x] M76/ABI-v37：有序 fixture keyring、持久 `BNDRKEY1` key policy、key 2→3→4 的同盘迁移、冗余修复/只读稳态、坏签名/已退休 key 拒绝与 public-only split-signing 已由五正两负 QEMU 门封口。
- [x] M77/ABI-v38：init-only syscall 57、signature-first BMA1 精确绑定、双槽 `BNDRMAU1` exact-sequence hash-chain audit、write/flush/readback、maintenance session 对 mutating supervisor report 的门禁和只读 UI query 已由六正三负 QEMU 门封口。
- [x] M78/ABI-v39：不新增 syscall；双槽 `BNDRMEX1` completion ledger、前序完成门禁、`audit2/execution1` 精确同授权零 audit 写恢复、`audit2/execution2` kernel-validated completion 与 completed-replay pre-EL0 拒绝，已由两次 PSCI 正启动、一次宿主中断和三次负启动封口。
- [x] M79/ABI-v40：不新增 syscall；相对 sector 9/10 的双槽 376-byte `BNDRMST1` 固定记录 rotation1/rotation2/drain，支持 M78 迁移、已持久步骤只读 reconciliation、三个 post-marker cut、损坏最新槽回退/修复，并把 terminal chain 绑定进 aggregate completion；五次 PSCI 正启动、三次宿主中断和三次负启动通过。
- [x] M80/ABI-v41：不新增 syscall；相对 sector 11/12 的双槽 424-byte `BNDRMPL1` 为三个固定 operation 记录精确 plan/operation-instance/idempotency identity 与九次 `PREPARED→APPLYING→CONFIRMED` transition，只允许 apply 前补偿；prepared/applying/effect/cancel 四次宿主中断、result-unknown/effect-observed 恢复、损坏最新槽回退/修复和五份终态磁盘收敛均通过。
- [ ] 下一本地 P0：把固定三操作计划改为签名、数据驱动的 bounded plan，覆盖重复 authorization/rotation sequence、所有合法 cancel race、意外 effect divergence、更广双槽损坏组合与更长非确定性 soak；随后推进多 App 持久存储与包生命周期。可信硬件单调后端、BSP、启动链、真实控制器/PMIC、RPMB/eFuse、hotplug、真实 power-cut、SMP/IOMMU、刷写和真机恢复，都必须等用户指定目标并另行明确授权。
- [ ] 原生 Hello World。

### M4：AndroidBox MVP

- [x] DEX-0 预备切片：在 Bndroid guest 中验证仓库自有 APK 的唯一 stored
  `classes.dex`，并执行两个固定 pure-static integer `code_item`；此项不计为
  APK installer、ART、Activity 或 AndroidBox MVP。
- [x] Activity-0/ActivityLifecycle-1 预备切片：无分配解析 binary Manifest，选择
  exported MAIN/LAUNCHER，在纯数据模型中先解释 exact public no-argument constructor
  的两条指令，再执行受限 `MainActivity.onCreate(Bundle)V` 与 Activity/TextView shim；
  此项不计为 APK installer、ART、ActivityThread、通用 Framework、任意 Activity 或
  AndroidBox MVP。
- [x] Resources-1 预备切片：严格解析资源 fixture 的 default-config
  `resources.arsc`、一个 binary `TextView` layout 与 `android:text @string`，执行固定
  `onCreate→setContentView(int)` 并通过 QEMU 门；此项不计为 ResourceManager、
  qualifier/alias、任意 View/layout 或通用资源系统。
- [x] APK Install-0 子项：原始仓库自有 v2-only 单签名 Resources-1 fixture 已完成
  signature-first admission、单包双 registry/双 blob 持久事务、首启安装、两次无源
  generation-1 恢复、篡改拒绝、ABI-43/syscall-59 只读安装快照与 Settings/All Apps
  状态展示；同一严格 shape 又通过本机 `org.bndroid.macdemo` no-probe 组件安装/恢复。
  两者都不代表任意 APK；QEMU 实证不是物理断电，fault injection 单测也不是硬件存储证明。
- [x] APK Update-0 子项：对同一仓库 fixture 完成同包、同 signer certificate、
  `versionCode 2→3` 的 generation `1→2` 双槽原子切换；精确 source replay 与无源恢复
  零写入，旧版本 rollback 与签名篡改在磁盘变化前拒绝，Settings/Activity 显示
  Version 3、Generation 2 与更新资源。六启动 QEMU 门不是事务中途 cut 或物理断电证明。
- [ ] 通用 APK installer：任意合规 APK、多包、配额、稳定 App identity、per-app
  storage、生产 signer policy 与 Installer/PackageManager UI/API 均未完成。
- [ ] 通用 APK 更新/卸载：多包版本策略、signer rotation/lineage、rollback 授权、
  数据迁移/保留、用户更新/卸载 UI/API 与故障恢复均未完成。
- [ ] ART 启动。
- [ ] Bionic 基础运行。
- [ ] Binder 兼容。
- [ ] Activity 启动。
- [ ] Android View 显示。
- [ ] 触控输入映射。
- [ ] 网络和存储。

## 11. 最短落地路线

最好的实际路线是：

1. 保留默认 M32 transferable GraphicsBuffer/真实像素 copy-present 作为 feature-off 认证基线，并保留已完整验证的 M33 专用 lifecycle/window profile。
2. 保留已通过 dedicated QEMU 与完整回归矩阵的 M34 有界 App owner-death cleanup/restart 证据；它尚不覆盖 Launcher/Surface 或任意应用的通用故障策略。
3. 保留已通过的 M36 consumer cleanup、M37 单次 SurfaceServer restart/rebind 与 M38 producer-death orphan/two-slot reuse 证据。
4. 保留 M42—M79 各自历史封口；当前 ABI-v41/M80 chain 为 M80→M79→M78→M77→M76→M75→M74→M73→M72→M71→M70→M69→M68→M67→M66→M65→…→M55。M80 在既有 audit/execution/step 三套账本之上增加相对 sector 11/12 的双槽 `BNDRMPL1`，以 plan ID、operation-instance ID、idempotency key 和 PREPARED/APPLYING/CONFIRMED/COMPENSATED phase machine 约束三项固定 resident effect；四种宿主中断、result-unknown/effect-observed reconciliation、apply 前补偿与损坏槽回退均已由磁盘证据证明，terminal plan chain 也已绑定进 aggregate completion。它仍固定五服务/四边/两次 F5 和三项 operation；fixture root 无生产 HSM custody，QEMU disk 也不是 trusted RPMB/eFuse 或 host replay/erase/tamper-resistant 单调源，宿主终止也不是 physical power-cut，且不声称外部副作用 exactly-once 或 arbitrary resume。下一本地 P0 是签名、数据驱动的 bounded plan 与更宽 race/corruption/soak；BSP/PMIC、真实 power-cut、SMP/IOMMU、刷写、真实硬件和真机恢复均未证明，且任何真机工作都必须先由用户指定目标并另行明确授权。
5. 在 M54 固定 AppData 之上再扩展动态目录、独立 StorageServer、通用文件服务、VM mapping/shared memory 与 storage-backed 加载，并把 M20 固定双会话服务泛化为通用 runtime。
6. 再扩展到 virtio-gpu、WindowServer、完整 Compositor 和输入服务。
7. 接入网络、音频、电话等移动设备基础能力。
8. 做 `.bapp`、PackageManager、PermissionManager 和原生应用。
9. 移植 ART/Bionic/Binder，跑通最小 Android APK。
10. 最后推进真实 ARM64 手机硬件适配与兼容性矩阵。

这个方向能做出真正属于自己的系统，同时又能承接 Android 应用生态。Rust 是这个项目的最佳主语言，C/C++ 只作为必要兼容层存在。
