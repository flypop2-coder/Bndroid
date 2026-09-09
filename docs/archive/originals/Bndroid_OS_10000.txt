# Bndroid OS 10000 字详细 TODO 文档

> 本文件保留历史详细规划，不再是权威待办。当前精炼架构、模块合并规则和未来任务
> 统一见 [`TODO.md`](TODO.md)；若计划冲突，以 `TODO.md` 为准。

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
> package-lifecycle profile 则只在 kernel snapshot 为 installed 时换成已安装 App 的标题与图标。
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
> 本地 QEMU 门先证明篡改 APK 在首个 package write 前拒绝且磁盘不变；随后首启经
> read-only `fw_cfg` 安装 generation 1，并从完整 package-store readback 执行真实
> `onCreate(Bundle)V→setContentView(0x7f020000)`，I/O 为
> `reads=1032 writes=130 flushes=3`。同一可写磁盘再进行两次完全无 APK source 的
> 恢复启动，均从持久 APK 显示 `AndroidBox resource-backed view`，且每次
> `reads=389 writes=0 flushes=0`。整盘 diff 的变化严格限定于
> `BNDROID_PACKAGES` LBA `16384..16895`。
>
> 同一隔离 profile 的严格受限 Update-0 也已完成：只允许已安装
> `org.bndroid.demo` 由同一 v2 signer certificate 从 `versionCode=2`、
> generation 1 更新到仓库自有 `versionCode=3` fixture，并在完整持久 readback
> 验证后原子发布 generation 2。重复提供同一 v3 source 与完全无 source 的恢复启动
> 都是 `writes=0 flushes=0`；v2 rollback source 与篡改 v3 均在 package mutation
> 前拒绝，磁盘不变。这只是同包、同证书、严格递增版本的单 fixture 规则，不是可信
> 单调硬件、host rollback resistance 或通用 Android 更新机制。
>
> 同一存储与 kernel boot path 已实现严格受限 Uninstall-0：唯一 authority 是受信离线
> host 在 EL0 启动前通过只读 `opt/bndroid/package-uninstall` `fw_cfg` 提供的 canonical
> 256-byte `BNDUNS01` 请求。请求绑定当前 package、generation、version、APK
> length/digest 与 signer digest；删除用两个逻辑完全相同、同 generation 的 tombstone
> 防止单 registry 损坏后旧包复活。APK blob 不擦除但逻辑不可达，且当前没有 managed
> package data。12-boot QEMU gate 已在
> `target/androidbox-apk-uninstall0/check.gWbx8U` 留存，证明
> generation `1→2→removed 3→reinstall 4`、双墓碑同字节、零写 replay/recovery、
> 五类负例磁盘不变和 reinstall 后 Activity 持久恢复。
>
> ActivityLifecycle-1 只在纯数据 framework model 中解释 Manifest Activity 的 exact
> public no-argument constructor：两条指令完成受限 `Activity.<init>()V` super call 与
> `return-void`，随后才解释 `onCreate`；它不构造 ART 对象，也不是一般 Android lifecycle。
>
> package-store 的 43 项 host 单测另行覆盖 install/update 的 write/torn-write/flush、
> ack loss、幂等重试和 registry/blob 损坏回退，并覆盖 Uninstall-0 的双同代
> tombstone、两个写点及四类 torn prefix、flush/readback/ack loss、单副本修复、
> 零写 replay、单墓碑损坏不复活和 reinstall identity/signer/version/digest policy；
> 这些是内存块设备 fault injection，不是 QEMU host-cut、物理断电、控制器 cache
> 或真机存储证明。Install-0、Update-0 与 Uninstall-0 各自已有独立的保留 QEMU 证据。
>
> 该隔离 profile 使用 ABI 43 和只读 syscall 59：向 EL0 复制 640-byte `BNDAPS01`
> 安装快照，不暴露 APK bytes、handle 或 package-store 权限。Settings 新增 Apps 页，
> 显示版本、大小、generation、Resources-1 profile 与 digest 前缀；All Apps 的已安装
> 入口使用 generation-bound touch token，打开的只是 kernel 已验证输出。没有运行时
> install/update/uninstall UI 或通用 PackageManager API；Uninstall-0 只有 boot-time
> host request，不增加 mutation syscall。旧 kernel 不理解 `BNDPRM01`，因此该磁盘
> 没有 downgrade-safe 声明。
>
> 边界仍为 `art=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
> permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled
> real_phone_claim=0`。它不是任意 APK installer、一般 Android 兼容、网络 App 运行或真机
> 证明；完整细节见 `ANDROIDBOX_APK_INSTALL_0.md`、
> `ANDROIDBOX_APK_UPDATE_0.md` 与 `ANDROIDBOX_APK_UNINSTALL_0.md`。

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

> 执行状态（2026-07-27）：历史 M71 里程碑为 ABI-v32/M71 `unified-product-continuous-supervision-runtime`。它严格扩展匹配的 M70 kernel/userspace closure，字面 kernel chain 为 M71→M70 `unified-product-psci-shutdown-runtime`→M69 `unified-product-multiservice-liveness-runtime`→M68 `unified-product-liveness-runtime`→M67 `unified-product-runtime`→M66 `resident-platform-shutdown-runtime`→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55。M71 不新增 syscall、镜像或 capability right，syscall 上限仍为 54；ABI v32 只更新证据契约。M71 保留历史 M70 的 strict FDT/PSCI 1.1、无 semihosting QEMU `SYSTEM_OFF`，并新增事务式五服务目录（ServiceManager、SurfaceServer、InputServer、StorageServer、App）、4 条依赖（3 hard、1 soft）、事务式批量 probe、每服务 missed-probe tolerance=1、16 轮额外健康 soak，以及 SurfaceServer/InputServer 同轮各漏一次后独立恢复；StorageServer 连续漏报仍走历史真实 100 ms timeout、30 ms backoff、同槽下一代 replacement 与 App block/resume。每次启动精确为 21 轮、107 probes、104 Healthy、3 missed，20 轮全健康、18 个 batch/90 个 batched probes、2 个瞬态恢复和 1 个升级故障；两次同盘启动都通过真实 UI/AppData closure 与 PSCI self-exit。它仍是 bounded、故障注入、single-core QEMU 研究原型，`arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是 PMIC、硬件 poweroff 或真机证明。feature-off/default 仍为 ABI v23，历史 M55—M64 为 ABI v25；M65/M66/M67/M68/M69/M70/M71 分别为 ABI v26/v27/v28/v29/v30/v31/v32；历史 M59 双账仍隔离。M33—M71 共有三十九个 opt-in leaf，加 default M32 为四十份独立账本。

> 历史 M56 是 kernel-only fail-stop recovery：`OutcomeUnknown`/`RequiresReset` 都是 session-fatal。旧 StorageServer 退出、session/capability 清理和 volume unbind 完成后，kernel 才执行 virtio status 0 reset，复核 device identity、相同 features/capacity，以仍独占的 DMA pages 重建 queue，并用 prepare/commit 两阶段重新 arm IRQ；旧 GIC pending/active 只在最初 disable 时清一次，re-enable 后保持 pending，并在 DAIF 屏蔽下完成 ISR/queue/status tail audit 后才 commit。失败则 rollback 并继续 fail-closed。新 StorageServer 只能以 owner epoch `+1` reacquire 并 remount durable volume，旧 session 不恢复。物理 commit 从不隐式开 admission：M55/M56/M57 同步路径为 `rearm→open→broker`，历史 M59 AppData/timeout 为 `rearm→open→finalize/return`，历史 M58 为不对称的 `rearm→broker→open`。M60 在 ownerless、DAIF-masked 的提交窗内改为 `rearm→open→prearm→broker→success-ledgers→restore-DAIF`；`prearm` 只在第 7 个 campaign epoch 由 kernel 授权，broker complete 后的成功账本也在恢复 DAIF 前发布。build wrapper 闭合 M66→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55 kernel chain；M65/M66 需要匹配 kernel/userspace feature，M63/M64 userspace 继续被拒绝。

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

> 历史 M65 使用 ABI v26/syscall 53 实现 init-only 两阶段 shutdown。两个固定 workload client 退出并被回收、StorageServer idle、broker/物理 I/O terminal 后，`Prepare` 才关闭新进程与 StorageAcquire/Connect/Accept admission；两个 client 的 shutdown 调用被拒绝，Prepare 后的 `ProcessSpawn` 也被拒绝。StorageServer 收到 generation-qualified command 后执行最终 flush、同 generation 全量 recover/readback、ACK、正常退出；init `ProcessWait` 回收后发布精确 `InitReady` 并 `Commit`。kernel monitor 再验证 process/broker/I/O/IRQ/DMA/syscall 账本，调用 M64 durable close，封闭 storage/IRQ/shutdown gate 后 halt。

> 同一 writable image 双启动证明 AppData generation 5→6、health session `1→2` 和 `3→4` clean close、两次 server flush/readback/exit，且 DATA/APPDATA 外字节不变。当前 log 为 91 行、18869 字节，SHA-256 `2c338c42a97a869375ccca50ec33cd8d84556b71cbcf7f56bffe8c44be58c645`：

```text
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_REBOOT_OK boots=2 userspace_shutdowns=2 clients_drained=4 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 offline_persisted=0 offline_from_record=0 el0_started=1 el0_controls=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M65 userspace StorageServer shutdown orchestration and durable close verified
```

> 这仍是 single-core storage-profile directed proof：没有完整 resident UI/service graph、PSCI/硬件 poweroff、真实断电、SMP 或 general runtime。

> 历史 M66 在 M65 上加入 ABI v27 与 child-only syscall 54 `ServiceShutdown`。init 同时启动 StorageServer、ServiceManager、Provider、两个不同身份的 Client、SurfaceServer、InputServer、Launcher 与 App。Prepare 前 kernel 核验 10 个 live process、9 对 init control、10 对 dependency Channel、38 个唯一 endpoint、39 个 handle、精确 rights/空队列、唯一 StorageVolume 及八个 PID/image/node 绑定。

> 全图登记后 Prepare 才关闭 admission；提前 quiesce SurfaceServer 必须被拒，随后 `Primary/Secondary/Launcher/App→Provider/InputServer→ServiceManager/SurfaceServer` 三波逆拓扑关闭。Launcher/App 执行真实 AppData workload 并验证 Prepare 后 StorageConnect 拒绝。八个节点以及 StorageServer 全部 exit/reap 后，kernel 才接受 InitReady/Commit、执行 M64 durable close、封闭 storage/shutdown、禁用 block IRQ，并在本地 IRQ mask 下创建 opaque `ValidatedShutdown` token。

> M66 唯一 backend 是 AArch64 QEMU semihosting `SYS_EXIT_EXTENDED`。同一 writable image 两次启动都要求 QEMU 自身 status 0 退出，host kill 不算成功。该历史 89 行/19200 字节日志 SHA-256 为 `656186fb9e4275bed2f64d95484160e97cdbe7960a74f90e209f566b7aff81de`：

```text
RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed
```

> 这只是 fixed-graph、single-core、emulator-only shutdown proof；不是完整产品 UI runtime，不是 PSCI/PMIC/硬件 poweroff，不证明真实掉电、SMP、general runtime 或真机。

> 历史 M67 先让真实 M45 UI/InputServer 路径收敛，再把 QMP `qcode=power` 作为物理键码 116 经认证 InputServer 路径交给 init；Launcher/App 执行 AppData，final StorageServer 启动后进入历史 M66 closure。M67 专用边界是 process capacity `10/9`、64 KiB worker exception stack 与 256-page/1 MiB heap。两次 release 启动均要求 status-0 QEMU 自退出和同一最终截图 SHA-256：

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

> M56 `7813/7810`、M57/M58 `9625/9619`、M60/M61/M62 `9023/9016` 与所有 masked-tick 最大值都是构建样本。历史 M71 入口为 `CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-continuous-supervision-runtime.sh` 和 `CARGO_NET_OFFLINE=true ./scripts/check-unified-product-continuous-supervision-static.sh`；M70 及更早 checker 继续作为隔离 regression gate。M71 parser 自测 positive/serial-negative/host-negative=`4/18/3`，持久日志为 `target/m71/unified-product-continuous-supervision-runtime.log`。

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

> M36 让 App 写满并 Queue、SurfaceServer Acquire 后被终止。reaper 以 stopped address-space mapping metadata/pin 为权威，先清除 consumer 的 75 个 leaves 并做 ASID TLBI，再以 BBM+producer ASID TLBI 恢复 RO→RW，随后 generation-safe abandon/release 并唤醒 waiter。普通 consumer 在 frame pending 时 unmap 被拒，仍有 mapping 时 close 也被拒；App 醒来后重写全部 75 页、抽样读取、显式 unmap/close/exit。专用 QEMU 走 Acquired 分支，Queued/Acquired 均有 host test；终态 graphics mapping/page/surface/handle 全为零。

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

> M47 把 ABI 推进到 v23，syscall 0—40 原样保留，新增 register-only syscall 41 `InputSessionInfo(handle, 0, 0)`，只对合法 InputCapability 返回 session 与 acquisition floor。strict canonical little-endian 64-byte `BIR1`/`BIP1`/`BIC1`/`BIE1` 分别封结 replacement bind、Surface recovery phase、route snapshot/ack 与 routed event；严格 sequence tracker 防止 gap/replay/direction 混用。InputServer session/route epoch 从 `1/1` 过渡到 `2/2`，Surface session 稳定为 1。

> Surface 先以越权 `InputAcquire` 触发 `permission_denied`，audit=`1`，handle/session delta=`0/0`，证明失败不污染 capability/session 状态。旧 InputServer 退出后 broker 进入 unbound gap，Surface 仍存活且 pending=0；Init 以有限 wait-array 完成精确 fixed 30 ms timeout，attempt/budget=`1/1`、`early_spawn=0`，再以同 process slot/generation+1 启动 replacement。新 InputServer 先 reacquire session 2/acquisition floor 1，然后以 `BIP1` RebindOffer 与 6 条 `BIC1`/7 条 `BIE1` 重建 snapshot、两条 route、Launcher focus、`capture=none`、`text=none`；sequence `2—3` 的 App down/up 再证明 focus `launcher→app`、capture `down→up→none`且 text delta 0。

> 最终 broker enqueued/dequeued/pending/high-water=`3/3/0/1`、release=`1`、unbound drop=`0`、Surface reacquire/fallback=`0/0`；process created/exited/reaped/live=`11/2/2/9`，handle/endpoint/pair=`36/30/15`，exact waits object/many/array=`9/2/7`。精确运行 marker 为：

```text
INPUT_SERVER_RESTART_PERMISSION_OK caller=surface syscall=input_acquire status=permission_denied audits=1 handles_delta=0 sessions_delta=0
INPUT_SERVER_RESTART_GAP_READY floor=1 broker=unbound surface=alive pending=0
INPUT_SERVER_RESTART_BACKOFF_OK attempt=1 requested_ns=30000000 timeout=1 early_spawn=0 budget=1/1
INPUT_SERVER_RESYNC_OK snapshot=1 routes=2 focus=launcher capture=none text=none floor=1 bic=6 bie=7
INPUT_SERVER_RESTART_ROUTE_OK sequence=2-3 target=app focus=launcher/app capture=down/up final_capture=none text_delta=0
INPUT_SERVER_RESTART_OK abi=23 protocol=1 wires=BIR1/BIP1/BIC1/BIE1 sessions=1/2 surface_session=1 epochs=1/2 restart=1 backoff=fixed-30ms budget=1/1 quarantine=host-verified broker=3/3/0/1 releases=1 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/2/2/9 handles=36 endpoints=30 pairs=15 waits=9/2/7 errors=0
BOOT_OK: M47 bounded InputServer restart, backoff, Surface resync, and permission denial verified
```

> `scripts/check-input-server-restart.sh` 已验证 pre/gap/post 三相；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`。dedicated M47、M46/M45 regression 与完整 suite 均已从头 exit 0；host 账本为 650 default + 24 directed = 674 executed/unique，M47 kernel 为 338，suite 终态固定 `input_server_restart=1`。

> M47 QEMU runtime 只执行一次 InputServer restart；`quarantine=host-verified` 只表示有限策略宿主测试通过，不代表 M47 QEMU 已执行 repeated-fault quarantine 或 degraded UI。上述 650+24=674/338、pre/gap/post 三张截图、marker 与完整总套件结论仍是不可回写的 M47 历史封口。

> **M48 历史完整封口**

> M48 保持 ABI v23 与 syscall 0—41 不变。新增的无分配、定容 `ServiceSupervisor` 与 strict canonical little-endian fixed-64-byte `BSH1` health protocol 把 `ServiceIdentity` 绑定为 service kind、process generation 与 generation-qualified PID。opcode/flags/fault class、inbound/outbound sequence、restart attempt/budget、interval 和所有 reserved/padding 字节必须在 supervisor 状态推进前全部严格通过；unknown/gap/replay/direction/identity 混用全部 fail closed。

> 独立 `service-supervisor-runtime` 只构造 `ServiceSupervisor::<1>` 监督 InputServer，不得写成已完成多服务监督。它先继承 M47 的授权 process-exit 恢复：首个故障分类为 `process-exit`、attempt 1/budget 1，同槽 generation+1 replacement 重新 reacquire，并以 session/epoch `1→2`、routes 2、floor 1 完成 Surface resync。Init 对 replacement 发送 `Probe` sequence 1，InputServer 在 30 ms deadline 内返回匹配的 `Healthy` sequence 1；第二个 probe 被精确 dequeue 但故意保持沉默，watchdog 因而将第二个故障分类为 `health-timeout`。

> attempt 2 超过 runtime budget `1/1`，supervisor 进入 quarantine 而不再 spawn。Init 发出 canonical `Quarantine`、终止并 reap 不健康 replacement、保持 kernel input broker unbound，并接收精确 `DegradedAck`。SurfaceServer 保持存活，提交 frame 9/write generation 6 的 32×24 红色 degraded badge。终态 process created/exited/reaped/live=`11/3/3/8`，resident handle/endpoint/pair=`31/26/13`，pending object/many/array waits=`8/2/6`，broker release=`2`、unbound drop=`0`。稳定 guest marker 为：

```text
SERVICE_SUPERVISOR_FIRST_FAULT_OK class=process-exit attempt=1 budget=1 gap=1 broker=unbound
SERVICE_SUPERVISOR_RESYNC_READY sessions=1/2 epochs=1/2 routes=2 floor=1 bic=6 bie=7
SERVICE_SUPERVISOR_WATCHDOG_OK probes=2 healthy=1 requested_ns=200000000 timeout=1 class=health-timeout
SERVICE_SUPERVISOR_QUARANTINE_OK attempt=2 budget=1 reason=restart-budget-exhausted degraded=1 frame=9 generation=6
SERVICE_SUPERVISOR_OK abi=23 protocol=1 wire=BSH1 faults=process-exit/health-timeout probes=2/1 watchdog=fixed-200ms restart=1 budget=1/1 quarantine=runtime degraded=1 broker=unbound releases=2 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/3/3/8 handles=31 endpoints=26 pairs=13 waits=8/2/6 errors=0
BOOT_OK: M48 generic ServiceSupervisor watchdog, runtime quarantine, and degraded UI verified
```

> `scripts/check-service-supervisor.sh` 以 strict trace/validator/topology 封口：七类 proof marker 必须唯一、字段集精确且顺序固定；old/replacement PID 必须同槽且 generation 只 +1；M45/M46/M47 或其他 leaf marker、Surface fallback 和 legacy input 证据均被拒绝；终态 resident topology/waits 与 broker 隔离必须精确收敛。checker 验证 `target/m48/service-supervisor-pre.ppm`、`service-supervisor-gap.ppm`、`service-supervisor-recovered.ppm` 与 `service-supervisor-degraded.ppm` 四个不同阶段；RGB SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`、`32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`。精确 checker seal 为：

```text
SERVICE_SUPERVISOR_QMP_OK armed=1 first_fault=process-exit resync=1 health=1 watchdog=health-timeout quarantine=runtime degraded=1 broker=unbound pointer=136/184/move-down-up screenshots=pre-gap-recovered-degraded pre_sha256=9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992 gap_sha256=c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324 recovered_sha256=5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65 degraded_sha256=32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff markers=armed/fault/resync/health/watchdog/quarantine/runtime/boot
```

> 历史 M48 当时的 host 账本为 670 default（ABI/compositor/ELF/input/SM/UI/init/kernel=`26/51/19/106/51/120/0/297`）+ 24 directed（`11 ui_trace + 10 window_trace + 3 persistent`）= 694 executed/unique；`bndr-sm` 测试由 31 增至 51，`service-supervisor-runtime` kernel all-tests 为 338。dedicated M48 QEMU、M45/M46/M47 隔离回归、default/M48/all-feature/mixed-feature 静态构建/Clippy 矩阵与最终全量 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 均已 exit 0。精确终态 suite marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M48 仍只是单一 InputServer 的有界运行时见证；其 ABI-v23、694 host/338 kernel、四截图、runtime quarantine 与 `service_supervisor=1` 现归历史，不能回写成 M49 的双服务结论。

> **M49 历史完整封口**

> M49 保持 ABI v23 与 syscall 0—41 不变。`service-dependency-runtime` 实例化 `ServiceSupervisor::<2>`，注册 SurfaceServer 与 InputServer，并建立唯一 `InputServer -> SurfaceServer` soft dependency。Surface 故障只 hard-block Surface，Input 仍存活；Input 故障 hard-block Input，并把仍存活的 Surface 标为 soft-degraded。固定容量 DAG 对 self/duplicate/cycle、未注册节点、过期 identity 和错误恢复顺序 fail closed。

> QEMU 首先终止 Surface generation 1；Input generation 1 跨 route gap 存活，同槽 Surface generation 2 以 session 2、route epoch 2、floor 3 恢复并提交 teal frame 1。真实 2 s finite wait 固定该阶段，Surface 随后在 100 ms deadline 内回 `Healthy`。Input generation 1 再真实 dequeue Probe 而不回 `Healthy`，100 ms watchdog 分类 `health-timeout`；Surface 保持存活，提交 frame 2/write generation 2 的红色 32×24 route-lost badge。经过真实 2 s degraded hold 与 fixed 30 ms restart backoff，Input generation 2 同槽启动，以 session 2、route epoch 3/floor 3 完成 BIR/BIP、六 BIC/七 BIE snapshot；Surface 提交 distinct green frame 3，Input2 回 `Healthy`，两个 impact 收敛为 `unaffected`，无 restart storm。终态 process=`12/3/3/9`、handle/endpoint/pair=`36/30/15`、wait=`9/2/7`。

```text
SERVICE_DEPENDENCY_SURFACE_RECOVERED surface_generation=2 surface_session=2 input_generation=1 input_session=1 route_epoch=2 floor=3 frame=1
SERVICE_DEPENDENCY_SURFACE_HEALTHY probes=1 healthy=1 surface_generation=2 timeout_ns=100000000
SERVICE_DEPENDENCY_WATCHDOG service=input generation=1 probes=1 reads=1 healthy=0 timeout_ns=100000000 impact=input-hard/surface-soft
SERVICE_DEPENDENCY_DEGRADED surface_alive=1 input_alive=0 phase=route-lost frame=2 write_generation=2 floor=3
SERVICE_DEPENDENCY_INPUT_REBOUND input_generation=2 input_session=2 route_epoch=3 floor=3 bic=6 bie=7
SERVICE_DEPENDENCY_INPUT_HEALTHY probes=1 reads=1 healthy=1 input_generation=2
SERVICE_DEPENDENCY_OK abi=23 protocol=1 services=2 dependency=input-soft-surface health_messages=5 probes=3 probe_reads=3 healthy=2 watchdog=1/1 bir=8+3 bip=4 bic=6 bie=7 surface=1/2 input=1/2 sessions=1/2 epochs=1/2/3 floor=0/2/3 frames=1/2/3 outputs=3 restarts=1/1 impacts=unaffected/unaffected created=12 exited=3 reaped=3 live=9 reasons=exited1/killed2 handles=36 endpoints=30 pairs=15 waits=9/2/7 topology=resident final_state=ready
BOOT_OK: M49 dependency-aware SurfaceServer and InputServer supervision verified
```

> `scripts/check-service-dependency.sh` 默认离线运行，并以 `-nic none` 显式禁用 QEMU 网络。pre/gap/surface-recovered/degraded/recovered 五阶段冻结 SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`、`32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`、`a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d`。M49 历史 suite 精确整行为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M49 仍只是单核 QEMU 中固定两服务、单 soft edge、固定窗口和脚本故障的研究/模拟器系统，不是生产手机，也未达到真机可用。任意 App/window、产品级 IME/font/Unicode/multitouch、网络/蜂窝/Wi-Fi、音频、电源、安全启动、沙箱、安全更新、产品驱动与真实硬件闭环仍缺。

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

> M54 新增独立 opt-in `app-data-runtime`，只在该 leaf 把 ABI 推进到 v24；feature-off default 与完整封口的 M53 继续使用 ABI v23。确定性磁盘镜像新增 GPT index 2 `BNDROID_APPDATA`，范围固定为 LBA `128-2047`、共 1920 sectors，不覆盖 index 1 `BNDROID_DATA` 或 index 0 `BNDROID_SYS`。为容纳无分配 AppData transaction/recovery frame，M54 monitor stack 为 256 KiB；M53 回归仍精确使用 128 KiB。M33—M54 共二十二个 opt-in leaf，加默认路径合计二十三套独立账本。

> AppData 是 capability-scoped、固定容量的 `/data` 原型。canonical path 最长 64 bytes、目录深度最多 4、单文件最多 4096 bytes、volume 内目录与文件合计最多 32 entries、live payload 总量最多 128 KiB；当前只有 single principal 1。root capability 只有 `READ|WRITE|DUPLICATE`，没有 `TRANSFER`；read-only attenuation 后不能再执行 create/write/unlink 等 mutation。Init、Launcher 与两个 SurfaceServer identity 都不能取得 AppData root，拒绝不会发布 handle 或扩大权限。

> 持久化格式使用双 checkpoint/双 bank snapshot。四次专项运行分别证明：fresh image 完成 created 且 generation `0→2`；boot 2 读取旧内容并 upgraded `2→3`；boot 3 在 generation 3 stable mount 且写入数为 0；复制镜像损坏 newest checkpoint 后从 generation 2 fallback，再升级回 generation 3。`bndr-appdata` 的 22 项 host tests 包含 292 个 mutation crash points 与 965 个 sector-atomic first-format points；首个 `FORMAT_INTENT` sector 或最终 immutable superblock 的 torn write 都 fail closed，不能把任意污染误认成可重放格式化意图。

> M54 专项 checker 四次均在 `CARGO_NET_OFFLINE=true`、本地 Unix QMP、每次 QEMU 显式 `-nic none` 的条件下 exit 0，并验证 fresh/upgrade/stable/corrupt-newest 四条磁盘路径。它没有模拟真实断电，因此 checker 的精确边界是 `powercut_claim=0`；292/965 crash-point 结论属于 host sector model，不能冒充 QEMU power-cut 证明。M52、M53 与 M54 截图 SHA-256 仍同为 `97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`，M52→M53、M53→M54 framebuffer diff 均为 0。

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

> M31 在 M30 的独立 SurfaceServer/Launcher 边界上增加独立 EL0 App 与第二对 UI Channel。SurfaceServer 仍是唯一 Surface capability owner；Launcher 和 App 都无法 acquire Surface。64-byte `BUC1` v1 控制协议用 client-only `AttachAppEndpoint` 转移 App 的 server endpoint，attach payload 不夹带 app/focus transition；只有 Launcher 可以用严格递增 transition id 发起 `SetFocus`。SurfaceServer 以三项 wait-array 轮换等待 Surface、Launcher 与 App，两个 client 各用单对象 wait；两对通道都固定 kernel-stamped generation-qualified sender PID。

> Present protocol 已升为 v2，wire size 仍为 64 bytes。Launcher/App 提交带当前 focus generation 的 frame；server 只提交当前焦点 client 且 generation 匹配的帧，并把两个 client 的本地连续 frame id 映射为全局连续 commit 序列。后台或 stale-generation present 返回 `PresentCancelled`，不推进本地/全局序列，client 可用同一 frame id 在新 generation 下重试。default feature-off 保持直接分发且不注入时序；opt-in `ui-stale-present-evidence` 用 capacity-1 屏障在精确 13-commit baseline 后读入并持有 App frame 10/focus generation 8，待双方 FocusChanged generation 9 发布完成后立即走既有 cancellation，再于 App focus generation 10 用同一 frame 10 重试，最后回到 Home generation 11。该确定性单次证据收敛为 36 inputs/18 commits，不再依赖多次运行或尝试不同调度相位。pointer down 在 Home→Launcher 或当前 App 之间选定 recipient 并捕获到 release；surface trace 也证明 Settings 内按下、拖到 Home 区再抬起的 App 本地 sequence `1..3` 全部只到 App，Launcher 无泄漏且不触发 focus/frame commit。Launcher 独立绘制 Home，App 独立绘制 Phone/Messages/Settings。

> M32 增加两槽、page-aligned 的静态 XRGB8888 GraphicsBuffer pool：每槽固定 `208×368`、logical/backing `306176/307200` bytes。App 以 `READ|WRITE|DUPLICATE|TRANSFER=0x0f` 创建，用 syscall 29 进行非空、4-byte 对齐且单次最多 4096-byte 的规范化写入，再把 `READ|TRANSFER=0x09` 衰减副本随首个 64-byte `BUP1` `BufferPresent` 原子转移给 SurfaceServer。M32 历史 wire 为 v1；当前 v2 保留 Full geometry 和 client/global frame id、focus generation、buffer generation，并新增 byte 40..48 的 System UI revision；mobile frame 必须绑定当前非零 revision。最后一个引用释放时完整擦除 backing 并推进 slot generation。

> buffer decode、producer/rights、focus/sequence、generation、全部像素和 counter 在首像素前验证，失败/取消不改变 scene、序列与计数。启动期以一次 4-byte seed 和 pre-focus cancelled present 验证句柄握手，所以启动账本 `created/write_calls/write_bytes/presents=1/1/4/0`；交互路径由 App raster 76544 个真实像素并由 SurfaceServer copy-present。early per-page W^X window 为此从 RAM 前 4 MiB 扩为 6 MiB，新增 L3 覆盖但不放松权限。这是有界 object transfer+copy，不是 `mmap`、共享 EL0 mapping 或零拷贝。

> M23 使用确定性 8 MiB/16,384-sector raw fixture，SHA-256 为 `36f39e23da09401dc2f686217e2bb06d146a65cd14aa4b9b8b2f0fd4aa7b809b`，sector 0/1 FNV-1a64 为 `0xbebd264b8c14cd72`/`0x4ff56c6cb05d259b`。新的 allocation-free sector reader 隔离 parser 与 virtio DMA；GPT parser 严格验证 protective MBR、revision/header size/current+backup LBA、usable bounds、primary/backup header CRC32、两份 128×128-byte entry array 的 CRC32、布局与逐字节一致性，再唯一选择 raw Microsoft Basic Data GUID 的 `BNDROID_SYS`（LBA `2048-16350`）。FAT16 mount 验证 512 BPS、power-of-two SPC、reserved/FAT/root/total bounds、两个 FAT 镜像、14,186-cluster FAT16 范围以及有界、无 cycle/bad-cluster 的 chain；8.3 lookup/read 支持 root 和嵌套目录且拒绝非 canonical traversal。单一只读 VFS 把该卷挂在 `/system`，读取 `/system/HELLO.TXT` 与 `/system/SYSTEM/BUILD.TXT`，write 固定返回 read-only。142 次 parser read 加最初两次 batch read，共形成 144 次 IRQ-only completion，仍只永久占用两个 coherent DMA frame。该 M23 fixture/解析证据在 M24 原样保留。

> M24 在 FAT16 验证后将固定 28+40=68 bytes 缓存为 immutable `BootfsCatalog<2>`/VMO。`init` 入口 `x0` 得到 move-only `READ|TRANSFER` directory capability，不可 duplicate；syscall 23 `FileOpenAt(root,path_ptr,len)` 只接受最长 64-byte canonical root-relative UTF-8 path，返回 `READ|DUPLICATE|TRANSFER` VMO/size；syscall 24 `VmoRead(vmo,dst,packed offset/len)` 在 `x2` 高/低 32 bit 放置 offset/requested length，每次最多 4096 bytes并按 EOF 截短。VMO 没有 `WRITE|MAP|EXECUTE|WAIT`。EL0 证明两个完整内容/FNV、partial、EOF、bounds、bad address、rights attenuation、stale handle、WAIT denial 和 zero-payload transfer；catalog 发布后 `runtime_disk_reads=0 mapped=0 shared_memory=0`。

> M25 继续使用一个确定性 8 MiB GPT raw image，但加入 index 1、LBA `64-127` 的 private `BNDROID_DATA`，保持 `BNDROID_SYS` LBA `2048-16350` 和 FAT16 只读。当前 SHA-256 为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`，sector 0/1 FNV-1a64 为 `0xbebd264b8c14cd72`/`0x8294de399174037c`。normal device 必须可写并协商 `VERSION_1|FLUSH`，拒绝只读设备；WRITE type 1/FLUSH type 4 均要求 `used.len=1`，且 API 只允许 DATA LBA。DATA superblock 和两个 512-byte CRC record 共享一个绑定 GPT unique GUID 的 16-byte format epoch；双槽同时有效时 generation 必须相邻。事务严格按 inactive-slot WRITE completion → FLUSH completion → 双槽重读完成，保留旧 selected slot；fresh boot 为 `0→1`，系统分区与 DATA 外拒写时 ledger 不变。下文 M25—M30 marker 只作为对应阶段的历史证据；其中 ABI v14/v15 与旧五/七进程拓扑不能冒充 M31。

> M26 严格唯一发现 direct-root `qemu,fw-cfg-mmio`，验证 fw_cfg signature/features 并用 big-endian DMA 配置 `etc/ramfb`。kernel-owned framebuffer 为 320×480 XRGB8888、stride 1280、614400 bytes/150 pages；九色 static splash 的 kernel digest 为 `0x6ef9c2b7d15fde25`。headless `check-framebuffer.sh` 经 QMP screendump 验证 153600 pixels、9 colors/9 samples，pixel SHA-256 为 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。M26 还绑定 modern coherent virtio-input keyboard（device ID 18、MMIO `0x0a003c00`、SPI 78、queue 8/one DMA frame），验证 config name/key bitmap 的 A+Enter；block 保持 `0x0a003e00`/SPI 79。`check-input.sh` 通过 QMP A down/up 精确得到 A-down/SYN/A-up/SYN，completions/delivered/recycled=`4/4/4`、used/avail=`4/12`，drop/invalid/config IRQ/spurious 均为 0。这些是保留的 M26 历史证据。

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

> M34 专用 `app-crash-recovery-runtime` 已由 dedicated QEMU 验证 10 个严格事务：App1 完成 Launch/Activate/Suspend/Resume/graceful Terminate，App2 Launch/Activate 后意外退出，SurfaceServer 原子清除 endpoint、buffer、focus/capture/frame/input 并发送 `USC1 OwnerDied`，init 再启动并激活 App3。精确证据为 ALC1 messages/transactions/states/crashes=`46/10/19/1`，USC1 commands/acks/owner-deaths=`9/9/1`；App PID generation 为 `1→2→3`，created/exited/reaped/live=`11/3/3/8`，termination reasons=`2/0/1`，terminate accepted/completed=`1/1`。最终仍为 29 handle、26 endpoint/13 pair、8 个 exact wait token，object/many/array pending=`8/2/6`、termination-abandoned=`0/0/1`，GraphicsBuffer generation=`3`。这是 ABI-v18/M34 历史专项证据；M42—M70 各自历史封口保持不变，历史 M71 里程碑为 ABI-v32/M71 continuous supervision，字面 kernel chain 为 M71→M70→M69→M68→M67→M66→…→M55。

> M35—M40 依次证明 mapped BufferQueue、consumer abandon、server restart、producer orphan、software frame grant 与双槽 swapchain；M41/M42 补固定两窗口策略与 persistent boundary/peer-close/recreate；M43—M48 再依次补有界文本/软键盘、独立 InputServer、两类单次 restart 与单 InputServer watchdog/quarantine；M49 补固定 SurfaceServer+InputServer 的单 soft-edge 依赖恢复，M50/M51/M52 再补恢复后的 App input-to-frame、App→Launcher focus/frame 与 Launcher→App focus roundtrip，M53 补双客户端 session-2 lifecycle-focus 实时同步与独立 APCK terminal boundary。当前仍没有硬件 vblank/pageflip、DMA-BUF/IOMMU、任意规模的通用多服务监督、任意 window manager、产品级 InputServer、完整 IME/font shaping/Unicode、多点或真实硬件触屏。电话/蜂窝、Wi-Fi、音频、硬件网络、电源、产品级驱动、安全启动、完整沙箱、安全更新、通用 App 生态与可用产品 UX 均未完成，绝不能称为现实可用的手机系统。

## 1. 总体目标

Bndroid OS 是一个 Rust 优先的自研移动操作系统项目。它的主系统采用非 Linux 内核，系统架构参考现代高安全移动操作系统的优点，重点建设统一安全模型、统一应用生命周期、统一系统服务、统一图形栈和统一应用分发机制，并以通过完整 AndroidBox 兼容子系统运行 Android 应用为长期目标；当前已有不计入一般兼容性的 DEX-0、Activity-0 回归、纯数据 constructor→`onCreate` 的 ActivityLifecycle-1 与 Resources-1 precursor，完成了严格受限 Install-0/Update-0 及 12-boot boot-time Uninstall-0/reinstall；通用包管理和一般 Android 兼容仍未完成。

项目要完成的事情可以概括为：

- 用 Rust 实现一个适合移动设备的微内核或混合内核。
- 用 Rust 实现用户态系统服务。
- 用 Rust 实现原生应用框架和 SDK。
- 建立类似高安全移动系统的应用沙箱、权限和签名机制。
- 建立 Android 兼容层，让 APK 能在受控环境中运行。
- 建立开发工具链、模拟器、调试器和兼容性测试体系。
- 最终适配 ARM64 开发板和真实移动硬件。

更好的项目名称应避免和现有系统混淆，可以使用 Bndroid OS、Bndroid Mobile、Bndroid Runtime、BndroidBox 等自有命名。

## 2. Rust-first 工程原则

Bndroid OS 推荐把 Rust 作为主语言。系统级工程里，Rust 的价值主要体现在内存安全、并发安全、类型建模和模块边界。

### 2.1 Rust 用在什么地方

Rust 适合用于：

- 内核核心对象。
- 内存管理抽象。
- 调度器。
- IPC。
- Handle/Capability 模型。
- 用户态系统服务。
- 包管理器。
- 权限管理器。
- 日志系统。
- 更新系统。
- 原生应用框架。
- SDK 工具。
- Android 兼容桥接层。
- 测试工具。

### 2.2 C/C++ 用在什么地方

C/C++ 可以作为兼容层存在：

- AOSP 组件。
- ART。
- Bionic。
- Skia。
- Vulkan loader。
- OpenGL ES/EGL。
- 第三方硬件库。
- 某些厂商 HAL。

更好的工程边界是：核心系统由 Rust 控制，C/C++ 组件通过 FFI 隔离，所有跨边界调用都经过清晰封装。

### 2.3 Rust 内核开发方式

内核 crate 使用 `#![no_std]`，并按如下模块组织：

```text
kernel/
├── arch/aarch64
├── boot
├── memory
├── task
├── scheduler
├── ipc
├── handle
├── syscall
├── interrupt
├── driver
├── time
├── security
└── log
```

内核对象建议用强类型表达：

- `ProcessId`
- `ThreadId`
- `Handle`
- `Capability`
- `VmObject`
- `Channel`
- `Message`
- `AddressSpace`
- `PhysicalPage`
- `UserPtr<T>`

这样能减少系统调用参数混乱和权限误用。

## 3. 推荐系统架构

```text
+--------------------------------------------------+
| Applications                                     |
| Native Rust Apps | Android Apps | Web Apps       |
+--------------------------------------------------+
| App Frameworks                                   |
| Rust UI SDK | Android Shim | Web Runtime        |
+--------------------------------------------------+
| System Services                                  |
| AppManager | PackageManager | PermissionManager  |
| WindowServer | Compositor | InputServer         |
| AudioServer | NetworkServer | CameraServer       |
| NotificationServer | PowerServer | UpdateServer    |
+--------------------------------------------------+
| AndroidBox                                       |
| ART | Bionic | Binder | Framework Shim       |
| Linux ABI Subset | Surface Bridge | HAL Bridge     |
+--------------------------------------------------+
| Security Layer                                   |
| Code Signing | Sandbox | Capability | Audit     |
+--------------------------------------------------+
| Kernel                                           |
| Scheduler | VM | IPC | Handles | Drivers       |
+--------------------------------------------------+
| Hardware                                         |
| ARM64 | GPU | Display | Touch | Audio | Camera |
+--------------------------------------------------+
```

这套架构的优点是：

- 主系统清晰独立。
- Android 兼容层被隔离在 AndroidBox 中。
- 原生应用和 Android 应用可以共存。
- 系统服务可以统一控制权限、窗口、输入、音频、网络。
- Rust 能覆盖大部分高风险系统组件。

## 4. 内核详细 TODO

### 4.1 启动阶段

第一目标是在 QEMU ARM64 中启动 Rust kernel。

TODO：

- [x] 建立 AArch64 裸机 target。
- [x] 编写 `boot.S`。
- [x] 设置异常级别。
- [x] 初始化栈。
- [x] 清理 BSS。
- [x] 进入 Rust `kernel_main`。
- [x] 初始化串口。
- [x] 输出 boot log。
- [x] 解析设备树。
- [x] 初始化页表。
- [x] 打开 MMU。

验收标准：

- QEMU 能启动。
- 串口输出内核版本。
- panic 能输出文件和行号。
- 内核能识别可用内存范围。

### 4.2 内存管理

内存管理是系统稳定性的基础。

TODO：

- [x] 物理页分配器（固定 metadata、可回收、保留区排除与复用记账）。
- [x] bitmap/fixed-metadata allocator 原型。
- [x] 256 KiB first-fit 内核堆分配器。
- [x] 页表管理（内核动态映射与每进程私有 TTBR0）。
- [x] 用户态地址空间生命周期（每进程私有 TTBR0；init ASID 1；child first-fit ASID；通用有界 VMA/frame 集合逐页建图与权限校验、全 leaf/物理帧唯一性和进程间整组隔离验证；失败回滚，ASID TLBI 后页表、leaf 与元数据完整回收复用）。
- [ ] 内核态地址空间。
- [ ] VMO 对象。
- [ ] mmap。
- [ ] shared memory。
- [ ] copy-on-write。
- [ ] page fault handler。
- [ ] memory pressure event。

更好的设计是把可共享内存抽象为 `VmObject`，图形 buffer、IPC 大消息、Android ashmem/memfd 都可以建立在 VMO 上。

### 4.3 线程与调度

TODO：

- [x] 有界 Thread/Process object（generation PID、独立 HandleTable 与 16 KiB exception stack；feature-off default 为七 dynamic child/process capacity 8 与 11 contexts，M45—M53 input/recovery leaf 为八 dynamic child/process capacity 9 与 12 contexts，容量仍非通用）。
- [x] Context switch。
- [x] Kernel/exception stack（每个 live context 独立 16 KiB，并校验底部 canary）。
- [x] User stack（每进程 4 页 RW/NX，低/高各 1 页 guard；init/child 运行时触达全部 4 页）。
- [x] 单核 timer-driven round-robin scheduler。
- [ ] Priority scheduler。
- [x] 有界 timer wait queue 与 Sleep/wake。
- [x] ARM generic timer interrupt。
- [ ] Futex-like primitive。
- [ ] UI priority。
- [ ] Background priority。
- [ ] CPU accounting。

移动系统更好的调度策略：

- 前台 UI 线程优先。
- 音频线程低延迟。
- 后台应用降级。
- 充电和低电量采用不同策略。
- AndroidBox 内进程可被分组管理。

### 4.4 IPC 和 Handle

IPC 是这个系统的核心。建议采用 handle-based channel IPC。

TODO：

- [x] `Channel` 对象（当前双向容量 8）。
- [x] typed inline `Message`（scalar、最长 64-byte、或 64-byte + 1 handle）。
- [x] 小消息复制（精确 usercopy fixup 与失败 rollback）。
- [ ] 大消息 shared memory。
- [x] generic-object handle passing（当前对象含 Channel/Event/immutable VMO/SystemDirectory/Surface/InputCapability；move-only transfer 与 image-selecting startup bootstrap 在事务失败时保持源 handle/FIFO head，Surface 与唯一 InputCapability 本身不可 duplicate/transfer）。
- [x] capability check（READ/WRITE/DUPLICATE/TRANSFER/WAIT/SIGNAL，rights 不提升）。
- [x] 单对象阻塞等待（ABI v10 保留 `ObjectWait` + 对象专属 signal mask；当前八槽 generation/epoch token，最终七项 pending）。
- [x] 固定两项 wait-any 与相对 timeout（ABI v10 保留 `ObjectWaitMany`；poll/infinite/finite、全参数验证、最低 index、deadline-first exact-token 仲裁）。
- [x] 有界 1—8 项 wait-any 数组（当前 ABI-v23 保留 syscall 22 `ObjectWaitManyArray`；8-byte canonical LE item、最多 64-byte usercopy、copy+全项 validation-before-ready、最低 index、真实八项 block/wake、显式 completion kind 与 exact token）。
- [x] 非消费式 Channel 头部探测（ABI v10 syscall 20 `ChannelPeek`；READ、reserved-zero、kind/length、queued-before-peer-close，且明确与 read 非原子）。
- [x] manual-reset Event（`EventCreate`/`EventSignal`/`EventClear`、`SIGNALED`、可转移与直接阻塞唤醒）。
- [ ] synchronous call。
- [ ] asynchronous call。
- [x] dead peer notification（level-triggered `PEER_CLOSED`，由 ObjectWait 阻塞等待）。
- [x] service discovery 原型（独立、init-only 监督的 EL0 `BSM1` v1 broker，严格 `BSA1` v1 attach、固定容量 4、两代重启/重绑；manager2 常驻但尚非产品 ServiceManager）。
- [ ] 超过 8 项/通用任意长度 wait-any、wait-all、显式 cancel、高精度/SMP timer 与同步/异步 IPC timeout。
- [ ] audit hook。

IDL 示例：

```text
service PermissionManager {
  Check(app_id: String, permission: String) -> PermissionState;
  Request(app_id: String, permission: String) -> PermissionDecision;
  Revoke(app_id: String, permission: String) -> Status;
}
```

Rust 生成接口示例：

```rust
pub trait PermissionManager {
    fn check(&self, app_id: AppId, permission: Permission) -> PermissionState;
    fn request(&self, app_id: AppId, permission: Permission) -> PermissionDecision;
    fn revoke(&self, app_id: AppId, permission: Permission) -> Status;
}
```

### 4.5 系统调用

原生系统调用建议少而稳定：

- [x] feature-off default/M53 的 ABI v23 已实现有界 Channel/Event/wait/process ABI、syscall 23
  `FileOpenAt`、24 `VmoRead`、25/26/27 Surface API、28/29/30 GraphicsBuffer create/write/buffer-present、31 `ProcessTerminate`、ABI-v19 32—36 map/unmap/queue/acquire/release、ABI-v20 `FRAME_READY` bit 4/syscall 37 `SurfaceFrameAcquire` 与 ABI-v21 `KEY_READY` bit 5/syscall 38 `SurfaceReadKey`；ABI-v22 复用 `READABLE`/`PEER_CLOSED` 并新增 register-only syscall 39 `InputAcquire` 与 40 `InputReadEvent`；ABI-v23 新增 register-only syscall 41 `InputSessionInfo`。`ProcessWait` 返回 exit code 与 `Exited/Faulted/Killed` reason，两个旧存储调用仍只覆盖固定 boot catalog。M54 `app-data-runtime` 与历史 M59 child `app-data-async-recovery-runtime` 使用 ABI v24/capability-scoped AppData 调用且不含 StorageServer；M55—M64 StorageServer branch 使用 ABI v25/syscall 47—52；M65 使用 ABI v26/syscall 53；M66 使用 ABI v27/syscall 54；M67/M68/M69/M70/M71 分别使用 ABI v28/v29/v30/v31/v32 且均不新增 syscall；M72 使用 ABI v33/init-only syscall 55 `ServiceManifestOpen`；M73 使用 ABI v34/init-only syscall 56 `ServiceSupervisorReport`；M74 使用 ABI v35 且不新增 syscall。ABI-v26 raw 54、ABI-v32 raw 55、ABI-v33 raw 56、ABI-v34 与 ABI-v35 raw 57 都保持 `UnknownSyscall`。

- process_create
- process_start
- process_kill
- thread_create
- thread_exit
- channel_create
- channel_read
- channel_write
- channel_call
- vmo_create
- vmo_map
- event_create
- event_wait
- timer_create
- handle_close
- handle_duplicate

Android 兼容需要额外 Linux ABI 子集：

- openat
- read
- write
- close
- mmap
- munmap
- mprotect
- futex
- epoll
- socket
- connect
- bind
- listen
- accept
- ioctl subset
- getpid
- gettid
- clock_gettime
- nanosleep
- sigaction
- clone subset
- prctl subset

更好的做法是把 Linux ABI 兼容放在 AndroidBox 层，主系统 API 保持干净。

### 4.6 块设备驱动

TODO：

- [x] M21（历史）从 FDT 有界发现 32 个 coherent、direct-root `virtio,mmio` transport，并确认只有一个 active block device。
- [x] M21/M22 历史只读路径仅接受 modern MMIO v2 与 `VERSION_1|RO`；M25 normal 改为要求 writable+`VERSION_1|FLUSH`，M22 race build 仍物理只读。
- [x] M22 解析 `interrupt-parent` 与 GICv2 三 cell specifier；active raw `0/47/1` 严格解析为 edge-rising SPI 47 / INTID 79，并配置 CPU 0 target/trigger/priority。
- [x] 两个 536-byte request slot 使用 descriptor head 0/3，在等待前真正同时提交两个读请求；hard-IRQ handler ACK transport、drain used ring，正常证据 `max_outstanding=2`、`poll_fallbacks=0`。
- [x] 在 descriptor 提交前拒绝越界访问；M25 的 11 类互斥 storage negative 另覆盖 DATA super/slots/generation gap、read-only 与 missing-FLUSH device。
- [x] feature-gated race 测试覆盖两请求 timeout、reset/re-negotiate/rebuild、两个旧 token 失效、late IRQ 作为 spurious ACK，以及无 double completion 的双请求恢复。
- [x] M23 allocation-free sector block boundary、protective MBR/主备 GPT CRC、双镜像 FAT16、有界 cluster chain、8.3 root/nested lookup/read 与 `/system` 单只读 VFS；正常路径 142 parser/144 total IRQ reads。
- [x] M24 把验证后的两个文件缓存为 immutable `BootfsCatalog<2>`/VMO，以 move-only root capability 和 ABI v13 open/read 暴露给 EL0；运行期 disk read/mapping/shared memory/write/persistence 均为 0。
- [x] M25 private DATA GPT entry、format-epoch-bound 双 CRC record、bounded raw WRITE/FLUSH/readback、跨 QEMU `0→1→2` 与 corrupt-newest fallback/recommit。
- [x] M26 strict `qemu,fw-cfg-mmio` discovery、fw_cfg DMA/ramfb 和 modern
  virtio-input keyboard；headless screenshot 与 QMP A-event sequence 均有独立验收脚本。
- [x] M27 opaque scene + alpha cursor compositor、mutable dirty redraw、virtio-tablet touch 与 interaction test。
- [x] M28（历史）kernel-owned clickable shell、Settings tap/damage 与 14-event/6-sample screenshot；323 项 host tests。
- [x] M29 protocol+raster/session 地基：canonical 64-byte `bndr-ui` 与严格全验证后 raster 的纯 `SurfaceSession`。
- [x] M29 运行时接入：ABI v14 syscall 25/26/27、EL0 唯一 capability/session、真实 `KernelFallback→UserspaceBound` 单向 ownership、ServiceManager 内嵌 SurfaceServer，以及独立 input event 分类；337 项 host tests。
- [x] M30 运行时拆分：ABI v15 六镜像 catalog、独立 SurfaceServer/Launcher、单独 UI Channel pair、七进程/19-handle 常驻拓扑；349 项 host tests。
- [x] M31 independent App/focus routing：ABI v16 七镜像 catalog、独立 SurfaceServer/Launcher/App、两对 UI Channel、八进程/21-handle 常驻拓扑；`BUC1` client-only attach、focus generation、Present v2 cancellation/同帧重试与 press-to-release capture 已固定。默认 host suite 为 359 项，trace feature 另启用 3 项 kernel 解码/路由测试。
- [x] M32 transferable GraphicsBuffer：ABI v17 两槽 XRGB8888、`BUP1`、App→SurfaceServer 衰减句柄、generation/scrub、失败原子性与真实 raster/copy-present；23 handle、377 host tests 和完整矩阵已通过。
- [x] M33 app lifecycle/minimal window management：ABI v18 canonical 64-byte `ALC1`/`UBP1`/`USC1`，7 个严格事务、App1 graceful exit/reap、App2 同槽下一 generation；专用 profile 收敛为 35 ALC、14 USC、`10/2/2/8` process、29 handle、26 endpoint/13 pair 与 object/many/array pending=`8/2/6`，host suite 406（`20/19/31/73/0/263`）。
- [x] M34 App crash/owner-death recovery：专用 `app-crash-recovery-runtime` 以 10 事务、ALC1 `46/10/19/1`、USC1 `9/9/1`、PID generation `1→2→3` 和 process `11/3/3/8` 验证 App2 peer-close 清理与 App3 同槽重启；dedicated QEMU 与完整回归矩阵均已通过。
- [x] M35/M36/M37 图形恢复链：mapped single-slot queue、consumer owner-death release，以及 generation-2 SurfaceServer restart/双 client rebind/resident mapped App frame recovery 均由各自专用 QEMU 验收。
- [x] M38 producer-death orphan/two-slot reuse：两个完整 307200-byte backing scrub/zero proof、generation-3 双槽复用、第三分配 OOM 与零终态 graphics 资源由专用 QEMU/checker 验证；host suite 418 与完整矩阵均已通过。
- [ ] 通用文件/metadata 写入、完整 crash consistency 和真实硬件 IOMMU/DMA/cache coherency/SMP 恢复。

## 5. 用户态系统服务详细 TODO

### 5.1 Init

TODO：

- [ ] 读取 boot config。
- [x] 由 `init` 通过 `ProcessSpawn`/`ProcessWait` 启动并监督两代独立 EL0 ServiceManager 原型。
- [ ] 启动 LogServer。
- [ ] 启动 SecurityServer。
- [ ] 启动 PackageManager。
- [ ] 启动 WindowServer。
- [x] 启动 bounded InputServer（M45—M53 独立八镜像 leaf；非产品级通用启动管理）。
- [x] 监控 ServiceManager 状态并执行一次 generation-safe 重启（其他服务与生产策略仍待实现）。
- [x] ServiceManager 请求重启后保留 dependent、观察 peer-close 并重绑。
- [x] M48 `ServiceSupervisor::<1>` 单 InputServer 策略：strict `BSH1`、首个 process-exit 重启、首次 healthy/第二次 health-timeout、fixed 200 ms health watchdog、30 ms restart backoff、budget 1、runtime quarantine 与 degraded ack/UI。
- [x] M49 固定 `ServiceSupervisor::<2>`、单 `InputServer -> SurfaceServer` soft edge、双服务 health/restart 与恢复顺序；通用多服务 DAG、动态策略与安全模式仍待办。
- [ ] 安全模式。

### 5.2 ServiceManager

TODO：

- [x] 服务注册（固定四项 registry）。
- [x] 服务发现（覆盖存在与不存在 lookup）。
- [x] 协议/接口版本（严格 64-byte `BSM1` v1 frame 与 `BSA1` v1 supervisor attach frame）。
- [ ] 权限声明。
- [x] init-attested bootstrap session role/owner PID（仅限 capability bootstrap，不是通用调用者身份）。
- [ ] 通用调用者身份识别。
- [x] capability endpoint 分发与连接 echo 原型。
- [x] 有界移除/重注册与 stale instance 拒绝原型。
- [x] 独立 manager 的 init-only 监督、同槽 generation +1 重启和 dependent 重绑原型。
- [ ] 审计日志。

M9 历史里程碑只完成 `init` 内托管原型，M12 移出 manager，M13 拆成四镜像，M14 增加两次 post-ready echo，M15 增加 Unregister/Reply 与十阶段生命周期。M16 再加入 kind/opcode 通用分派与一次固定空间 post-cleanup round，M17 加入 kernel-stamped writer identity、manager 固定 PID allowlist、delegated-endpoint 拒绝及对抗性 reducer。M18 历史切片让两个 Client 同时 outstanding，但 secondary 随后退出；M19 加入 ABI v12 八项 wait-array。服务层的现行基线 M20 改为两个常驻 Client 和两条独立、认证的 manager session，回复沿各自 session 路由，并完成一次停滞隔离、revoke/reattach 与 stale-lease 拒绝；M22 在其下方完成 IRQ/two-outstanding 块驱动，M23 再加入只读 GPT/FAT16/VFS，M24 增加 capability-scoped immutable boot-file VMO，M25 增加 DATA-bounded durable record。它们都没有把服务泛化为任意多客户端 runtime，产品 ACL/list、持久状态和审计仍未完成。

### 5.3 PackageManager

PackageManager 负责原生应用和 Android APK。

当前只有隔离单包生命周期子集：Install-0/Update-0 已有本地 QEMU 证据；
Uninstall-0 只允许受信离线 host 的 boot-time `BNDUNS01` `fw_cfg` 请求，其
双同代 tombstone 与 reinstall 约束已有 43 项 package-store host tests，并通过
12-boot QEMU gate。它没有运行时服务、Settings 操作、managed app data 或通用
PackageManager API。

TODO：

- [ ] 安装 `.bapp`。
- [ ] 卸载 `.bapp`。
- [ ] 更新 `.bapp`。
- [ ] 安装 APK。
- [ ] 解析 AndroidManifest.xml。
- [ ] 提取图标和应用名。
- [ ] 验证签名。
- [ ] 创建沙箱目录。
- [ ] 注册启动入口。
- [ ] 注册 intent/action。
- [ ] 回滚失败安装。

### 5.4 PermissionManager

权限模型：

- 网络
- 摄像头
- 麦克风
- 相册
- 联系人
- 定位
- 蓝牙
- 通知
- 后台运行
- 文件访问
- 剪贴板
- 传感器

TODO：

- [ ] 权限数据库。
- [ ] 安装时权限声明。
- [ ] 运行时权限弹窗。
- [ ] 一次性权限。
- [ ] 使用期间权限。
- [ ] 后台权限。
- [ ] 权限撤销。
- [ ] 权限访问日志。
- [ ] Android 权限映射。
- [ ] 原生权限映射。

### 5.5 AppManager

TODO：

- [ ] 原生应用启动。
- [ ] Android 应用启动。
- [ ] 前后台切换。
- [ ] 生命周期事件。
- [ ] 进程优先级。
- [ ] OOM 策略。
- [ ] 崩溃收集。
- [ ] 最近任务。
  当前 SurfaceServer-owned Overview 只有容量一的 App 身份卡片，没有任务实体、
  缩略图、后台状态、历史、清理或恢复，因此不满足此项。
- [ ] 后台冻结。
- [ ] 通知唤醒。

## 6. 图形和输入系统

### 6.1 图形路线

第一阶段：framebuffer。
第二阶段：virtio-gpu。
第三阶段：Vulkan。
第四阶段：真机 GPU。

组件：

- DisplayServer
- SurfaceManager
- WindowServer
- Compositor
- AnimationEngine
- RenderScheduler
- ScreenshotService
- ColorManager

TODO：

- [x] M26 kernel-owned 320×480 XRGB8888 ramfb static splash 与 headless QMP full-frame 验收。
- [ ] 创建 Surface。
- [ ] 分配 Buffer。
- [ ] 提交帧。
- [ ] VSync。
- [ ] Layer tree。
- [ ] Damage tracking。
- [ ] 窗口焦点。
- [ ] 安全截图权限。
- [ ] 防录屏 layer。
- [ ] 刷新率策略。

### 6.2 输入系统

TODO：

- [x] M26 modern virtio-input keyboard eventq、IRQ/recycle 与 QMP A-down/SYN/A-up/SYN。
- [ ] 触控输入。
- [ ] 多点触控。
- [x] M43 有界 QEMU 键盘输入、capacity-32 Surface key FIFO、focus-scoped route 与八字节 UTF-8 editor。
- [x] M45 独立 bounded InputServer、唯一 InputCapability 与 broker-only route/focus/capture/text-context/IME。
- [x] M47 一次 InputServer restart/reacquire、fixed 30 ms backoff、Surface resync 与 scoped permission denial（历史 674 host/338 kernel）。
- [x] M48 `ServiceSupervisor::<1>` + strict fixed-64-byte `BSH1`、process-exit 重启、第二次 health-timeout、budget-1 runtime quarantine 与 degraded UI（694 host/338 kernel）。
- [ ] 鼠标输入。
- [ ] 手写笔。
- [ ] 手势识别。
- [x] bounded Launcher/App 输入焦点与 unfocused key drop。
- [ ] 输入法。
- [ ] 安全密码输入。
- [ ] Android MotionEvent 映射。
- [ ] Native PointerEvent 映射。

## 7. AndroidBox 详细设计

> 当前实现分为隔离 profile：基础 `androidbox-dex0` 保留 DEX-0、Activity-0 回归与
> Resources-1 pre-compatibility slice，只解析仓库 fixture，并在纯数据
> ActivityLifecycle-1 中先解释两指令 exact public no-argument constructor，再执行固定
> `onCreate→setContentView(int)`；它不安装或发布 package state。独立
> `androidbox-apk-install0` 的原始实证以仓库自有 APK-v2 单签名 Resources-1 fixture
> 完成 signature-first admission、单包持久事务、首启安装、两次无源恢复和只读安装快照；
> 同一严格 Resources-1 shape 又通过本机 `org.bndroid.macdemo` no-probe APK 的安装/恢复。
> 同一隔离 profile 另以同包、同证书、递增版本的 v3 fixture 完成 generation-2
> Update-0、零写 replay/recovery 与 rollback/tamper 拒绝。这些只满足严格受限
> Install-0/Update-0。boot-time Uninstall-0 的 canonical
> request 绑定 durable identity，双同代 tombstone 逻辑撤销 APK，但 blob 不擦除、
> 没有 managed app data；12-boot QEMU gate 已验证 generation
> `1→2→removed 3→reinstall 4`。三者仍不满足通用
> installer/updater/uninstaller、运行时 UI、ART/Bionic、Binder、
> ActivityThread、ResourceManager、qualifier/alias、任意 View/layout、通用
> Framework/Activity、Surface/MotionEvent 或“首个通用 ART/Framework APK”。

### 7.1 AndroidBox 的位置

目标 AndroidBox 将成为 Android 应用兼容环境，并包含：

- APK installer
- app_process
- ART
- Bionic
- dynamic linker
- Binder compatibility
- Android Framework shim
- Package bridge
- Permission bridge
- Surface bridge
- Input bridge
- Audio bridge
- Network bridge
- Storage bridge

### 7.2 ART 和 Bionic

TODO：

- [ ] 移植 Bionic libc。
- [ ] 适配 pthread。
- [ ] 适配 linker。
- [ ] 适配 JNI。
- [ ] 移植 ART。
- [ ] 支持 boot image。
- [ ] 支持 dex loading。
- [ ] 支持 JIT 或解释执行。
- [ ] 支持 classloader。
- [ ] 支持 multidex。
- [ ] 支持 native library loading。

更好的第一版可以先以解释执行或最小 JIT 为目标，先跑通功能，再优化性能。

### 7.3 Binder

TODO：

- [ ] Binder device compatibility。
- [ ] binder_open。
- [ ] binder_mmap。
- [ ] binder_ioctl subset。
- [ ] transaction。
- [ ] reply。
- [ ] oneway。
- [ ] death recipient。
- [ ] Binder/AndroidBox service manager（不等同于 M9—M16 的 Bndroid BSM1 ServiceManager 原型）。
- [ ] Java Binder。
- [ ] Native Binder。
- [ ] AIDL。
- [ ] permission hook。

Binder 可以映射到底层 Bndroid IPC，也可以实现一套 AndroidBox 内部 Binder runtime。更好的路线是：AndroidBox 内对应用暴露 Binder，对系统服务转接到 Bndroid IPC。

### 7.4 Android Framework Shim

第一批 API：

- Activity
- Context
- Intent
- Bundle
- Handler
- Looper
- View
- TextView
- Button
- ImageView
- SharedPreferences
- SQLite
- HttpURLConnection

第二批 API：

- Service
- BroadcastReceiver
- ContentProvider
- Notification
- JobScheduler
- AlarmManager
- Clipboard
- DownloadManager
- WebView

第三批 API：

- Camera2
- MediaCodec
- OpenGL ES
- Vulkan
- SensorManager
- Bluetooth
- LocationManager
- BiometricPrompt

### 7.5 Android 服务映射

| Android Service | Bndroid Service |
|---|---|
| ActivityManager | AppManager |
| PackageManager | PackageManager |
| WindowManager | WindowServer |
| InputManager | InputServer |
| AudioManager | AudioServer |
| NotificationManager | NotificationServer |
| LocationManager | LocationServer |
| CameraManager | CameraServer |
| ConnectivityManager | NetworkServer |
| PowerManager | PowerServer |
| ClipboardManager | ClipboardServer |
| SensorManager | SensorServer |

## 8. 原生应用 SDK

### 8.1 `.bapp` 包格式

```text
MyApp.bapp
├── manifest.toml
├── signature
├── bin/main
├── lib
├── assets
├── resources
├── localization
└── privacy
```

### 8.2 声明式 UI

原生 UI 组件：

- Text
- Image
- Button
- List
- ScrollView
- Navigation
- TabView
- Sheet
- Alert
- TextField
- Toggle
- Slider
- Picker
- VideoView
- WebView

更好的 UI 方向是：Rust 声明式 UI + 系统统一动画 + GPU 合成。这样原生应用能获得流畅体验，也能保持系统风格一致。

### 8.3 SDK 工具

TODO：

- [ ] bndroid new
- [ ] bndroid build
- [ ] bndroid run
- [ ] bndroid install
- [ ] bndroid log
- [ ] bndroid sign
- [ ] bndroid package
- [ ] bndroid emulator
- [ ] bndroid debug
- [ ] bndroid profile

## 9. 安全体系

### 9.1 应用签名

TODO：

- [ ] 开发者证书。
- [ ] 系统根证书。
- [ ] 包签名。
- [ ] 安装时验证。
- [ ] 启动时验证。
- [ ] 动态库验证。
- [ ] 更新签名一致性。
- [ ] 签名撤销列表。

### 9.2 沙箱

应用沙箱目录：

```text
/apps/{app_id}/container
/apps/{app_id}/data
/apps/{app_id}/cache
/apps/{app_id}/tmp
```

TODO：

- [ ] 每应用独立 UID 或 capability identity。
- [ ] 每应用独立数据目录。
- [ ] 文件访问通过 StorageServer。
- [ ] 网络访问通过 NetworkServer。
- [ ] 摄像头通过 CameraServer。
- [ ] 麦克风通过 AudioServer。
- [ ] 权限访问可审计。

### 9.3 安全启动和 OTA

TODO：

- [ ] Bootloader 验证 kernel。
- [ ] Kernel 验证 system image。
- [ ] System image 只读。
- [ ] OTA 包签名。
- [ ] A/B 分区。
- [ ] 失败回滚。
- [ ] rollback protection。
- [ ] recovery mode。

## 10. 文件系统和存储

推荐布局：

```text
/system
/system/bin
/system/lib
/system/services
/apps
/users
/data
/cache
/tmp
/config
/security
/runtime/android
```

TODO：

- [x] M21（历史）QEMU modern virtio-mmio v2 只读 polling block driver 与当时的 4096-sector 确定性 fixture。
- [x] M22 GICv2 SPI 79 中断驱动、固定两槽/two-outstanding block read、digest 验证、越界提交前拒绝，以及 legacy/corrupt/no-device 三项负测。
- [x] M22 timeout/reset/token invalidation/late-spurious/recovery race 自测；恢复前后只保留两个 DMA frame 且无 double completion。
- [x] M23 确定性 8 MiB GPT/FAT16 fixture、主备 GPT CRC/entry mirror、双 FAT mirror、只读 `/system` 与根/嵌套文件证据。
- [x] 只读系统分区原型（单分区、单挂载、8.3 lookup/read）。
- [x] M24 两项/68-byte immutable boot catalog、move-only directory root、VMO open/read/attenuation/transfer EL0 证明。
- [x] M25 private `BNDROID_DATA` raw partition 与 kernel-owned 双槽 boot-count record；这不是通用用户数据文件系统。
- [x] 历史 M54 opt-in `BNDROID_APPDATA`（GPT index 2、LBA `128-2047`、1920 sectors）与 single-principal capability-scoped AppData；22 项 host tests、四次 offline/`-nic none` QEMU 与当时完整总 suite 均已通过，最终 marker 含 `app_data_runtime=1`；它由 kernel monitor 执行，`powercut_claim=0 general_runtime=0`。
- [x] 历史 M55 独立 ABI-v25 StorageServer：八镜像 ELF catalog、syscall 47—52、strict v2 4160-byte/最多 8-sector transport、无 DUP/TRANSFER volume/session、userspace namespace、100 Hz logical + one-shot physical scheduling、idle restart/rebind，以及三启动 `0→4→5→5`/第三次 AppData 零写零 flush；release 三启动、严格 Clippy/host/feature matrix 与 2026-07-17 完整 `./scripts/test.sh` 均已通过，终态含 `storage_server_static=1 storage_server_runtime_boots=3`。
- [x] 历史 M56 ABI-v25 fail-stop recovery：read/write/flush 三类 session-fatal timeout、old-owner 解绑后 kernel-only status-0 reset、identity/features/capacity 复核、queue DMA rebuild、两阶段 IRQ rearm 与 replacement epoch+1 durable remount；runtime/static checker 已通过。
- [x] 历史 M57 ABI-v25 repeated recovery：两轮串行 `WRFWRF`、suppression `2/2/2`、IRQ-safe broker access、Running-owner `ServiceAbandoned`、physical submission gate，以及一次 IRQ commit abort 后真正成功 retry；runtime/static checker 已通过。
- [x] 历史 M58 ABI-v25 StorageServer 分支前身：七次 physical recovery 拆为四相位，driver borrow 跨相位释放，每个 cooperative step 对 reset status 或带 generation 的 capacity tuple 至多采样一次；async coordinator 持有 StorageServer rearm/broker/admission policy commit；七个 timer/双 worker/已认证 EL0 progress window、零 masked polling，以及被计量的 driver/control 区间均短于 timer period已通过 runtime/source/parser/static checker；这不是全内核 DAIF 时长证明。
- [x] 历史 M59 cooperative unification 双账本：ABI-v24 AppData 对一次 read fault 实测 steps/pending/yields=`4/3/5`、wait/dispatch=`2/2`、mask `53500/86187<625000`；ABI-v23 timeout 实测 steps/yields=`4/3`、windows=`3/3`、dispatches `9`、workers `539135/376383`、mask `59375/87188<625000`、`el0_progress_claim=0`。物理 rearm commit 不隐式开 admission，各 coordinator 显式开 gate；两账与 StorageServer profile 始终隔离。
- [x] 历史 M60 ABI-v25 StorageServer fault policy：`WRFWRFR`、kernel-prearmed permanent authority、attempt cap/backoff/Probation 与 boot-local Offline 已封口。
- [x] 历史 M61 ABI-v25 fault-latched owner liveness：fatal completion 同窗启动不可续期 250 ms ticket；7 arms/6 cooperative/1 forced、epoch-6 flush stall、live volume close denied、真实 `ObjectWait` abandonment、ordinary reaper、epoch-7 replacement 与后续 Offline 已封口。
- [x] 历史 M62 ABI-v25 terminal quarantine：一次 kernel-only proof deferral、三步/two-Pending fallback、一次模拟 physical error、reverified IRQ/DMA Offline boundary 与零 attempt/ticket/EL0 控制已封口；parser/policy/mismatch=`204/8/6`、host=771、feature-kernel=353。
- [x] 历史 M63 ABI-v25 persistent unclosed-boot hint/reprobe：80-byte 双槽 payload、legacy upgrade、同一镜像双启动、fresh probe、default compatibility、persisted/record-derived Offline 与 EL0 controls 为 0。
- [x] 历史 M64 ABI-v25 clean close/no-later-storage：exact open-session capability、两次 clean close、prior-closed 第二启动、pre-EL0 admission/IRQ seal、post-close storage mutation=0；parser/persist/mismatch=`93/31/4`、feature-kernel=368。
- [x] 历史 M65 ABI-v26 bounded shutdown orchestration：init-only Prepare/Commit、两个 client drain、post-Prepare spawn rejection、StorageServer final flush/readback/exit/reap、M64 durable close 与 storage/IRQ/shutdown seal；parser/mismatch/host/ABI=`133/4/4/1`、feature-kernel=372。
- [x] 历史 M66 ABI-v27 resident platform shutdown：八个认证 resident 节点、10 条依赖、三波逆拓扑 quiesce、kernel topology proof、九 child exit/reap、StorageServer final flush/readback、opaque fail-closed token 与两次 QEMU semihosting 自退出。
- [x] 历史 M67 ABI-v28 unified product runtime：真实 M45 UI/InputServer convergence、认证 power key 116、Launcher/App AppData、final StorageServer、M66 shutdown closure、双启动 exact screenshot/disk/self-exit，明确 `general_runtime=0 real_phone_claim=0`。
- [x] 历史 M68 ABI-v29 bounded product-service liveness：BSH1 `3/2/1` probe/Healthy/withheld、100 ms timeout、30 ms backoff、restart budget 1、同 slot generation + 1、一次 killed replacement、replacement mount/Healthy 与两次完整自退出均已封口。
- [x] 历史 M69 ABI-v30 bounded two-service dependency liveness：固定 StorageServer+App、一条 hard edge、BSH1 `8/7/1`、三次 40 ms cadence、App block/resume ACK、100 ms timeout、30 ms backoff、同槽下一代 replacement 与最终双服务 Healthy 均已封口；不新增 syscall。
- [x] 历史 M70 ABI-v31 QEMU PSCI shutdown：strict `/psci`、compatible/method、PSCI_VERSION、opaque kernel seal、HVC SYSTEM_OFF、无 semihosting与两次 QEMU self-exit 均已封口；不新增 syscall，不声称 PMIC/hardware poweroff。
- [x] 历史 M71 ABI-v32 五服务连续监督：事务式 5-service/4-edge 目录与批量 probe、21 个 cadence 轮/16 个额外健康轮、同窗两服务瞬态 miss 容忍与独立恢复、一次超限 StorageServer replacement，以及两次 PSCI QEMU self-exit 均已封口；不新增 syscall，明确 `arbitrary_soak_claim=0 real_phone_claim=0`。
- [x] M72—M75：immutable BMF1、事件监督、外部 BMS1 签名门和双槽持久 rollback floor 已依次封口。
- [x] M76/ABI-v37：有序 fixture keyring、持久 `BNDRKEY1` key policy、key 2→3→4 的同盘迁移、冗余修复/只读稳态、坏签名/已退休 key 拒绝与 public-only split-signing 已由五正两负 QEMU 门封口。
- [x] M77/ABI-v38：init-only syscall 57、signature-first BMA1 精确绑定、双槽 `BNDRMAU1` exact-sequence hash-chain audit、write/flush/readback、maintenance session 对 mutating supervisor report 的门禁和只读 UI query 已由六正三负 QEMU 门封口。
- [x] M78/ABI-v39：不新增 syscall；独立双槽 `BNDRMEX1` completion ledger、前序完成门禁、`audit2/execution1` 精确同授权零 audit 写恢复、`audit2/execution2` kernel-validated completion 与 completed-replay pre-EL0 拒绝，已由两次 PSCI 正启动、一次宿主中断和三次负启动封口。
- [x] M79/ABI-v40：不新增 syscall；相对 sector 9/10 的双槽 376-byte `BNDRMST1` 固定记录 rotation1/rotation2/drain，支持 M78 迁移、已持久步骤只读 reconciliation、三个 post-marker cut、损坏最新槽回退/修复，并把 terminal chain 绑定进 aggregate completion；五次 PSCI 正启动、三次宿主中断和三次负启动通过。
- [x] M80/ABI-v41：不新增 syscall；相对 sector 11/12 的双槽 424-byte `BNDRMPL1` 为三个固定 operation 记录精确 plan/operation-instance/idempotency identity 与九次 `PREPARED→APPLYING→CONFIRMED` transition，只允许 apply 前补偿；prepared/applying/effect/cancel 四次宿主中断、result-unknown/effect-observed 恢复、损坏最新槽回退/修复和五份终态磁盘收敛均通过。
- [ ] 下一本地 P0：把固定三操作计划改为签名、数据驱动的 bounded plan，覆盖重复 authorization/rotation sequence、所有合法 cancel race、意外 effect divergence、更广双槽损坏组合与更长非确定性 soak；随后推进多 App 持久存储与包生命周期。可信硬件单调后端、BSP、启动链、真实控制器/PMIC、RPMB/eFuse、hotplug、真实 power-cut、SMP/IOMMU、刷写和真机恢复，都必须等用户指定目标并另行明确授权。普通 AppData I/O 仍 busy-spin。
- [ ] 通用用户数据文件系统。
- [ ] 产品级多 App 私有目录（M54 只实现 principal 1 的固定容量原型）。
- [ ] 共享媒体库。
- [ ] 文档选择器。
- [ ] Android scoped storage。
- [ ] 加密存储。
- [ ] Keychain。
- [ ] Backup。
- [ ] Restore。
- [ ] Quota。

## 11. 路线图

### 0 到 3 个月

- Rust no_std kernel。
- QEMU ARM64 启动。
- 串口日志。
- 页表。
- 内存分配。
- 异常处理。
- 简单线程。
- 初始 IPC。

### 3 到 6 个月

- 用户态 init。
- ServiceManager（M20 已有固定双常驻 session 与一次停滞隔离，通用多客户端仍待办）。
- 存储（历史 M25/M54 与 M55—M71 完成 raw record、fixed AppData、StorageServer 恢复、durable hint/close、resident shutdown、统一 UI、FDT+PSCI QEMU exit 与有界五服务监督；M72—M79 依次增加 BMF1/BMS1、事件监督、持久 rollback/key policy、维护授权/完成/步骤账本；当前 ABI-v41/M80 在相对 sector 11/12 增加双槽 424-byte `BNDRMPL1`，以精确 plan/operation/idempotency identity 记录三个 operation 的九次 prepare/apply/confirm transition，支持 result-unknown/effect-observed reconciliation、apply 前 compensation、四个宿主中断和损坏槽回退/修复，并把 terminal plan chain 绑定进 aggregate completion。但产品仍固定五服务/四边/两次 F5/三项 operation，没有生产 HSM custody/ceremony/授权恢复；QEMU audit/execution/step/plan disk 不是 trusted RPMB/eFuse，也不抵抗 host replay/erase/tamper，宿主中断不是物理 power-cut。仍没有 BSP/PMIC、硬件 poweroff/hotplug，也不是 POSIX/通用文件系统或真机掉电恢复，`external_effect_exactly_once_claim=0 arbitrary_resume_claim=0 arbitrary_service_set_claim=0 trusted_monotonic_backend=0 production_key_claim=0 hsm_claim=0 rpmb_claim=0 efuse_claim=0 hardware_powercut_claim=0 general_runtime=0 real_phone_claim=0`）。
- LogServer。
- 文件系统原型。
- framebuffer/UI（M31 已有独立 userspace SurfaceServer + Launcher + App、两对 UI Channel 与显式 focus；共享图形 buffer/WindowManager/text 仍待办）。
- 简单输入（M45 已拆出 bounded InputServer，M47 完成一次 restart/reacquire，M48 完成单服务 watchdog/quarantine，M49 再完成固定双服务单 soft-edge 恢复；通用多服务策略、timeout/slop/cancel、真实触屏/多点/完整手势仍待办）。
- 历史监督增量（M49 已完成）：`ServiceSupervisor::<2>` 依赖感知 SurfaceServer+InputServer，固定 `InputServer -> SurfaceServer` soft edge；Surface exit、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与重启风暴抑制均已有五阶段 QEMU 证据，ABI 保持 v23。
- 历史交互增量（M50 已完成）：M49 恢复后拒绝 phone 外 `(16,32)`，再把 App `(136,184)` 的 physical `6/7` 经两条 BWE 闭环到 Present sequence 5/frame 3、output frame 4/write generation 4 与 health `2/2` resident。
- 历史焦点增量（M51 已完成）：继承 M49/M50，以 screen `(80,96)`、physical `8/9`、BIC sequence 3 完成 Launcher capture、focus `App/3→Launcher/4` 与 Present command 6/frame 4/scene 11/output 5/write generation 5；ABI/syscall/八 ELF/capacity 9/12-context 均不变。
- 历史 roundtrip 增量（M52 已完成）：继承完整 M49/M50/M51 前缀，以 App `(136,184)`、physical `10/11`、BIC sequence 4 完成 focus `Launcher/4→App/5`、Present command 6/frame 4/scene 12/output 6/write generation 6；M52 自身 lifecycle focus 账本仍为 2，APCK 不计入其 17-bit、Channel `8/8` trace。
- 历史 lifecycle-focus 增量（M53 已完整封口）：以 M52 为严格 feature parent，在 session 2 对 Launcher/App 双客户端同步 `Ready/App1/RFCK→Launcher2/LFCK→App3/AFCK`；Channel `14/14`（BUE 8 + ACK 6），M52 APCK 另作 boundary `1/1`，physical `1..11`、event 17、Input `App/4`、compositor `App/5`、output 6 与 framebuffer 均无新增。
- StorageServer branch 增量：M55 已封口独立 ABI-v25 userspace namespace、kernel raw-sector broker、三启动 `0→4→5→5` 与第三次零写；M56 在相同 ABI 上封口 old-owner teardown、kernel reset/rebuild/rearm 与 epoch+1 remount；M57 再封口串行 `WRFWRF` 与 rollback/retry；M58 把七次 physical recovery 拆为四相位 cooperative executor 并实证 admission gate 与跨窗口进度。whole disk 因 DATA boot counter 改变，不是 stable equality 证据。
- 历史 M59 存储恢复增量：共享 `cooperative-block-recovery` 被 M58 StorageServer、ABI-v24 AppData 与 ABI-v23 timeout self-test 复用；后两者是隔离账而非 M58 子 profile，不安装彼此 runtime，也不得拼接计数。
- 历史 M70 产品增量：kernel chain 为 M70→M69→M68→M67→M66→M65→…→M55；在继承 M69 exact UI/AppData/dependency-liveness/shutdown seal 的同时，完成 strict `/psci`、PSCI_VERSION 1.1、opaque descriptor 与无 semihosting QEMU SYSTEM_OFF。下一硬件阶段须先指定并授权目标，再做 BSP/PMIC、hotplug/device replacement 和真实硬件验证；目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak 仍待推进。
- 历史 M71 产品增量：kernel chain 为 M71→M70→M69→M68→M67→M66→M65→…→M55；在继承 M70 exact UI/AppData/PSCI seal 的同时，完成事务式五服务/四依赖目录、事务式 batch、21 轮/107 probes、同窗两服务瞬态漏报恢复与一次超限 StorageServer replacement。它仍为编译期给定目录和注入式 bounded witness，`arbitrary_soak_claim=0 general_runtime=0 real_phone_claim=0`。
- 原生 Hello World。
- `.bapp` 原型。

### 6 到 12 个月

- WindowServer。
- Compositor。
- PackageManager。
- PermissionManager。
- AppManager。
- 原生 UI SDK。
- 严格受限 APK Install-0（原始仓库 fixture 与同 shape 的本机 Mac no-probe APK
  均已完成受限安装/恢复）。
- 严格受限 APK Update-0（已完成同包同证书 v2→v3、generation 2）。
- 严格受限 APK Uninstall-0（boot-time path、43 项 package-store tests 与
  12-boot QEMU gate 已完成；无运行时 UI/API）。
- 通用 Android APK parser/installer（任意合规 APK、多包、更新/卸载仍未完成）。
- Bionic 基础适配。

### 12 到 18 个月

- ART 启动。
- Binder 兼容。
- Activity 生命周期。
- Android View 显示。
- Surface bridge。
- 输入 bridge。
- 网络和存储。
- 跑通首个依赖 ART、ActivityThread 与通用 Framework 的 APK。

### 18 到 24 个月

- WebView。
- Audio。
- Notification。
- Camera2 基础。
- MediaCodec 基础。
- SDK 完善。
- Emulator。
- Debugger。
- Compatibility database。

## 12. 最高优先级 TODO

P0：

- [x] Rust workspace。
- [x] QEMU ARM64 kernel 原型。
- [x] boot log。
- [x] VM 基础。
- [x] 单核 scheduler 基础。
- [x] Channel/Event IPC 基础。
- [x] ELF `init`。
- [x] 独立、init-only 监督的 ServiceManager 容量 4 原型（两代 bootstrap、M14/M15 基线与 M16 一次 post-cleanup 有界复用；完整 transcript/Channel 图/wait token 持续稳定）。
- [x] M21（历史）QEMU modern virtio-mmio v2 只读轮询块设备与三项存储负测。
- [x] M22 GICv2 SPI 中断 completion、两请求并发队列、无 polling fallback 和 timeout/reset/late-IRQ recovery。
- [x] M23 block boundary、双份 GPT/FAT16 校验、`/system` 只读 VFS、142 parser/144 total IRQ reads 与六类 storage negative。
- [x] M24 ABI v13 capability-root `FileOpenAt`、immutable `VmoRead` 与完整 EL0 正负路径；历史 248 项 host tests。
- [x] M25 bounded DATA records、WRITE/FLUSH/readback、跨重启 generation/recovery、269 项 host tests与 11 类 storage negative。
- [x] M26 strict fw_cfg DMA/ramfb、virtio-input keyboard、283 项 host tests 与
  `check-framebuffer.sh`/`check-input.sh`。
- [x] M27 最小双层软件 compositor、可变 redraw、pointer/touch 与 interaction test；297 项 host tests。
- [x] M28 kernel-owned 可点击 shell、三个 app target + home、Settings generation-1 commit 与 full-frame UI 验收。
- [x] M29 已把 `bndr-ui`/raster/session 接入 syscall/EL0，完成唯一 capability/session、真实单向 ownership、输入 FIFO/coalescing 与 ramfb/headless 验收；完整套件为 `userspace_surface=1 userspace_ui=1`。
- [x] M30 已拆分独立 SurfaceServer/Launcher，以单独 UI Channel pair 交换 canonical frame/event；ABI v15、六镜像、七进程与 349 项 host tests 已验证。
- [x] M31 已加入独立 App、第二对 UI Channel、显式 focus/按焦点 input、Present v2 focus generation 与 `PresentCancelled` 同帧重试；ABI v16、七镜像、八进程、默认 359 项 host tests已验证。QEMU capture trace 证明 Settings 内按下、拖到 Home 区再抬起的本地 sequence `1..3` 全部只到 App，Launcher 无泄漏且不触发 focus/frame commit；全程 24 个全局 input sample、13 个连续 commit，截图 hash 不变。
- [x] M32 已加入 transferable GraphicsBuffer/buffer-present 与真实 App raster；baseline 为 24 input/13 commit（4 Launcher legacy + 9 App buffer），stress 为 36 input/18 commit，默认 377 host tests及完整 `./scripts/test.sh` 均通过。
- [x] M33 专用 `app-lifecycle-runtime` 已完成 7 事务、generation-safe App replacement、Surface slot install/retire 与最终 7 项监督 wait-array；默认 M32 normal 路径继续保留独立认证账本。
- [x] M34 专用 `app-crash-recovery-runtime` 已完成 App2 owner-death 通知、原子资源清理、Launcher focus fallback 与 App3 generation-safe restart；dedicated QEMU 与完整 `./scripts/test.sh` 均已通过。
- [x] M37 专用 `graphics-surface-restart-runtime` 已完成一次 SurfaceServer generation-2 restart、Launcher/App 重绑与 resident mapped frame 恢复；`scripts/check-graphics-surface-restart.sh` 严格验收。
- [x] M38 专用 `graphics-producer-orphan-runtime` 已完成一次 producer-first orphan reclamation、完整双槽 scrub/zero proof 与 safe reuse；`scripts/check-graphics-producer-orphan.sh` 严格验收。
- [x] M48 已完成 ABI-v23 不变的单 InputServer `ServiceSupervisor::<1>`/`BSH1` watchdog、process-exit 重启、第二次 health-timeout、budget-1 runtime quarantine、degraded UI 与 strict trace/validator/topology；694 host/338 kernel、四阶段 QEMU 与完整总套件均已通过。
- [x] M49 `ServiceSupervisor::<2>` 依赖感知 SurfaceServer+InputServer；`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。
- [x] M50 保持 ABI-v23/syscall 0—41/八镜像/capacity 不变，完成恢复后 phone 外拒绝、App physical `6/7`→两条 BWE→Present sequence 5/frame 3→output frame 4/write generation 4→health `2/2` resident 的 QEMU 封口。
- [x] M51 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，继承 M49/M50 后以 physical `8/9`、screen `(80,96)`、BIC sequence 3 完成 Launcher capture、focus `App/3→Launcher/4`、Present command 6/frame 4/scene 11/output 5/write generation 5 与 health `2/2` resident；全量 suite 已固定 `post_recovery_focus=1`。
- [x] M52 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，继承完整 M49/M50/M51 前缀后以 App physical `10/11`、BIC sequence 4 完成 focus `Launcher/4→App/5` roundtrip、Present command 6/frame 4/scene 12/output 6/write generation 6；全量 suite 已固定 `post_recovery_focus_roundtrip=1`。
- [x] M53 保持 ABI-v23/syscall 0—41/八 ELF/capacity 9/12-context 不变，以 M52 为 feature parent，在 session 2 完成双客户端 `Ready/App1/RFCK→Launcher2/LFCK→App3/AFCK` 同步；Channel `14/14`（BUE 8 + ACK 6），M52 APCK 独立 boundary `1/1`，physical `1..11`、event 17、Input `App/4`、compositor `App/5`、output 6 与 framebuffer 均不新增；全量 suite 已固定 `post_recovery_lifecycle_focus=1`。
- [x] M54 opt-in ABI-v24 AppData 完整封口：GPT index 2/LBA `128-2047`、single principal 1、path/depth/file/entries/live=`64/4/4096/32/128KiB`、`READ|WRITE|DUPLICATE` 且无 `TRANSFER`；fresh `0→2`、upgrade `2→3`、stable 3 零写、corrupt-newest fallback 2/re-upgrade 3 已通过四次 offline/`-nic none` QEMU，22 项 host tests 覆盖 292+965 crash points，完整 suite marker 含 `app_data_runtime=1`。QEMU 仍为 `powercut_claim=0`。
- [x] M55 ABI-v25 standalone StorageServer：syscall 47—52、strict v2 batch、userspace namespace、kernel raw-sector broker 与三启动 `0→4→5→5` 已封口。
- [x] M56 ABI-v25 fail-stop StorageServer recovery：三类 session-fatal fault、old-owner teardown、kernel reset/rebuild/rearm 与 epoch+1 remount 已封口。
- [x] M57 ABI-v25 repeated StorageServer recovery：串行 `WRFWRF`、一次 IRQ commit rollback 与成功 retry 已封口。
- [x] 历史 M58 ABI-v25 StorageServer 分支前身：七次四相位 cooperative physical recovery、跨窗口 timer/worker/EL0 progress 与零 masked polling 已封口；不作全内核 DAIF 证明。
- [x] 历史 M59 双账本 cooperative unification：ABI-v24 `app-data-async-recovery-runtime` 与 ABI-v23 `storage-irq-timeout-self-test` 隔离运行并保留原 marker。
- [x] M60 ABI-v25 bounded fault policy：`WRFWRFR`、attempt cap 3、2×2 backoff、Probation、boot-local sticky Offline、kernel-prearmed permanent authority 与终态 IRQ/DMA direct proof 已封口；default host 754、M60 kernel 336、parser M58/AppData/IRQ/M60=`18/25/29/111`、46/46 `-nic none`，suite 新增 `storage_server_fault_policy_static=1 storage_server_fault_policy=1`。
- [x] M61 ABI-v25 fault-latched owner retirement：不可续期 250 ms physical-counter grace、exact monitor kill、真实 ObjectWait abandonment、ordinary reaper barrier 与 epoch-7 replacement 已封口；不声称通用健康 heartbeat。
- [x] M62—M64：terminal quarantine fallback、persistent health/fresh reprobe 与 exact-session durable close/no-later-storage 已分别封口。
- [x] M65 ABI-v26 init-only 两阶段 StorageServer shutdown、final flush/readback/exit/reap 与 durable close/seal 已封口。
- [x] M66 ABI-v27 fixed resident graph、kernel topology proof、三波逆拓扑 quiesce、opaque token 与两次 QEMU-only self-exit 已封口。
- [ ] 存储下一 P0：推进目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak；用户指定并授权目标后，再做 BSP/PMIC、hotplug/device replacement 与真实控制器/硬件适配。
- [ ] PackageManager。
- [ ] PermissionManager。
- [ ] `.bapp`。
- [ ] 原生 Hello World。

P1：

- [ ] WindowServer。
- [ ] Compositor。
- [x] 严格受限 APK Install-0：原始仓库 APK-v2 单签名 Resources-1 fixture 完成
  signature-first admission、单包持久事务、首启安装与两次无源恢复；同一严格 shape
  的本机 `org.bndroid.macdemo` no-probe APK 也完成安装/恢复，仍不代表任意 APK。
- [x] 严格受限 APK Update-0：仅同包、同 v2 signer certificate、`versionCode`
  严格递增的 v2→v3 fixture 原子发布到 generation 2；同源 replay/无源恢复零写，
  rollback 与篡改 source 拒绝且磁盘不变。
- [x] 严格受限 APK Uninstall-0 存储/kernel path：只接受 boot-time
  `BNDUNS01`，双同代 tombstone、逻辑 APK revocation、无 managed package data，
  package-store 共 43 项 host tests，并已留存 12-boot QEMU gate 与整盘证据。
- [ ] 通用 Android APK parser/installer：任意合规 APK、多包、更新/卸载及生产 signer
  policy。
- [ ] Bionic。
- [ ] ART。
- [ ] Binder。
- [ ] Activity。
- [ ] Surface bridge。
- [ ] Input bridge。

P2：

- [ ] WebView。
- [ ] Audio。
- [ ] Notification。
- [ ] Camera。
- [ ] MediaCodec。
- [ ] OpenGL ES。
- [ ] Vulkan。
- [ ] OTA。
- [ ] 真机适配。

## 13. 执行建议

最好的执行方式是把项目拆成几个可验证的里程碑，每个里程碑都能运行、能演示、能测试。

第一里程碑追求 QEMU 启动和日志，第二里程碑追求有界用户态服务，第三里程碑由 ABI-v20/M42 建立 persistent userspace window session，并由 ABI-v21/M43 完成 focus-scoped hardware keyboard 与 bounded UTF-8 text-editor slice；M44 再完成 SurfaceServer 内三键 touch soft-keyboard，ABI-v22/M45 完成独立 bounded InputServer，M46 完成一次 Surface restart/rebind，ABI-v23/M47—M53 完成 InputServer/双服务恢复与 lifecycle-focus 同步。历史 ABI-v24/M54 加入 kernel-monitor fixed AppData；M55—M66 依次完成 standalone StorageServer、恢复、boot-local Offline、owner/quarantine、durable hint/close、resident shutdown 与 QEMU-only exit；ABI-v28/M67 统一真实 UI/InputServer、AppData、认证 power key 与 exact UI/self-exit；M68—M79 依次完成监督、PSCI、manifest 签名/rollback/key rotation、维护授权/完成/步骤账本。当前 ABI-v41/M80 沿 M80→M79→M78→M77→M76→M75→M74→M73→M72→M71→M70→M69→M68→M67→M66→…→M55 增加双槽 `BNDRMPL1`、三个 operation-instance/idempotency identity、九次 prepare/apply/confirm transition、result-unknown/effect-observed reconciliation、apply 前 compensation、四个宿主中断与 terminal-plan-chain binding；下一本地阶段把固定计划改为签名、数据驱动的 bounded plan，并推进多 App 持久存储/包生命周期。可信单调硬件、BSP/PMIC、真实控制器/硬件与真机适配必须等用户指定目标并另行明确授权，之后才验证 hotplug、真实 power-cut、SMP/IOMMU、POSIX/general runtime、产品级 IME、网络、音频与电话。

M16—M20 的服务拓扑与 M29—M31 的 Surface/process split 均保留为历史下层。默认 M32 路径仍收敛为八进程、20 endpoint/10 pair、23 handle；M33 专用 profile 为 29 handle、26 endpoint/13 pair并证明一轮 graceful replacement；M34 在相同最终拓扑上证明一次 App peer-close cleanup 和第三代重启。三条证据必须分开解释：M34 的 dedicated QEMU 与完整矩阵均已通过，但它仍不是任意 App/Launcher/Surface 故障下的产品 runtime。

这条路线能把一个超大型系统工程拆成可完成的小步骤。Rust 作为主语言可以提升安全性和维护性，AOSP/C/C++ 组件作为兼容层接入，主系统保持自研和干净。


## 14. Rust 实施规范补充

### 14.1 Workspace 规划

Bndroid OS 的工程可以从一开始就按 workspace 管理，避免后期重构成本过高。推荐把内核、用户态服务、共享协议、SDK 工具和 AndroidBox 桥接层拆成独立 crate。

```text
Bndroid/
├── Cargo.toml
├── kernel
├── crates
│   ├── bndr-abi
│   ├── bndr-ipc
│   ├── bndr-capability
│   ├── bndr-manifest
│   ├── bndr-package
│   └── bndr-ui
├── services
│   ├── init
│   ├── service_manager
│   ├── log_server
│   ├── package_manager
│   ├── permission_manager
│   ├── app_manager
│   ├── window_server
│   └── input_server
├── androidbox
├── sdk
├── tools
├── system_apps
└── docs
```

TODO：

- [x] 建立根 workspace。
- [x] 建立 `bndr-abi`，存放系统调用号、基础类型、错误码与 `UserImageId`（feature-off default/M53 与历史 M59 timeout 账为 ABI v23；M54/M59 AppData 为 ABI v24；M55—M64 StorageServer branch 为 ABI v25/syscall 47—52；M65 为 ABI v26/syscall 53，仍不新增镜像或 rights，各 profile 计数保持隔离）。
- [x] 建立 `bndr-sm`：保留无分配 `BSM1` v1 codec/容量 4 registry，M48 新增 strict fixed-64-byte `BSH1`、`ServiceIdentity`、health/fault/quarantine frame 与定容 `ServiceSupervisor`；测试由 31 增至 51。
- [ ] 建立 `bndr-ipc`，存放 IPC 编解码和 IDL 生成结果。
- [ ] 建立 `bndr-capability`，存放权限位、capability token、调用者身份。
- [ ] 建立 `bndr-manifest`，解析 `.bapp` 和系统服务 manifest。
- [ ] 建立 `bndr-package`，处理包签名、资源索引、安装事务。
- [x] 建立并接入 `bndr-ui`：Present/window/lifecycle wire 与 tracker，以及 M43 64-byte `BTI1`/`BTE1`、八字节 UTF-8 `TextEditor`、one-scalar preedit、revision/ack 与 focus/session/window 拒绝验证。

### 14.2 Rust 错误模型

系统服务应统一使用明确错误类型。错误码要能跨 IPC 传递，不能只在本进程内有意义。

推荐错误分类：

- `InvalidArgument`
- `PermissionDenied`
- `NotFound`
- `AlreadyExists`
- `Unavailable`
- `Timeout`
- `OutOfMemory`
- `InvalidState`
- `Unsupported`
- `Internal`

TODO：

- [ ] 统一错误枚举。
- [ ] 错误码和字符串分离。
- [ ] IPC 层传递错误码。
- [ ] LogServer 记录完整错误上下文。
- [ ] SDK 层把系统错误转换为开发者友好错误。

### 14.3 Capability API

Capability 是系统安全模型的核心。应用访问摄像头、麦克风、网络、文件、通知、定位时，不是直接调用硬件，而是向系统服务申请能力。

TODO：

- [ ] 定义 `CapabilityToken`。
- [ ] 定义 token 生命周期。
- [ ] 支持一次性 token。
- [ ] 支持使用期间 token。
- [ ] 支持后台 token。
- [ ] 支持系统服务 token。
- [ ] 支持 token revoke。
- [ ] 支持 token audit。

### 14.4 第一个月开发任务

第 1 周：

- [x] 创建 workspace。
- [x] 创建 kernel crate。
- [x] 配置 AArch64 target。
- [x] 编写 linker script。
- [x] QEMU 输出第一行日志。

第 2 周：

- [x] 初始化页表。
- [x] 实现物理页分配。
- [x] 实现 panic 输出。
- [x] 实现最小内核堆。
- [x] 解析设备树内存范围。

第 3 周：

- [x] 建立异常向量。
- [x] 建立 timer interrupt。
- [x] 实现线程结构（固定有界 context）。
- [x] 实现上下文切换。
- [x] 实现 round-robin 调度。

第 4 周：

- [x] 实现系统调用入口（feature-off default/M53 与历史 M59 timeout 为 ABI v23；M54/M59 AppData 为 ABI v24 syscall 42—46；M55—M64 StorageServer 为 ABI v25 syscall 47—52；M65 为 ABI v26 并只新增 init-only syscall 53。build wrapper 闭合 M65→M64→M63→M62→M61→M60→M58→M57→M56→M55 kernel chain、要求 M65 kernel/userspace 匹配并拒绝 userspace M63/M64 与 profile 混配）。
- [x] 实现 Channel IPC 原型（同一 typed FIFO 承载 scalar/byte/64-byte-plus-one-generic-handle 消息；move-only `OwnedHandle`、read/write rollback、self-transfer 拒绝与 dead-receiver drain 保持事务和所有权语义；非零单调 ID 的保守 DAG 拒绝 equal/reverse 所有权边并阻止引用环）。
- [x] 实现 Channel/Event 类型专属 level signals 与阻塞等待（Channel `READABLE`/`WRITABLE`/`PEER_CLOSED`、Event `SIGNALED`、`Rights::WAIT`、七槽 epoch/PID/handle token；commit/Signal edge 后锁外 scan/wake）。
- [x] 实现固定两项 wait-any 与相对 timeout（完整参数验证、最低 ready index、poll/infinite/finite、单次 counter 采样的 deadline-first exact-token 仲裁；100 Hz 最多晚一个 tick且不提前）。
- [x] 实现 syscall 22 有界八项 wait-any 数组（canonical LE、整段 copy 与全项校验先于 readiness、最低 index、真实八项 block/wake；completion kind 与 generation-qualified token 精确匹配）。
- [x] 实现可转移 manual-reset Event（幂等 signal/clear、`Rights::SIGNAL`、失败 rollback、未读销毁与直接 block/wake 证据）。
- [x] 实现独立受监督 ServiceManager 原型（严格 64-byte `BSM1`/`BSA1` v1、容量 4 epoch-namespaced registry；manager1 非标准 Exit 请求、同槽 generation +1 manager2，provider/client 跨故障存活、peer-close、重绑和两轮直连 echo；role/重复/未知/stale 拒绝与完整清理）。
- [x] 实现 M14 有界 post-ready loop，并在 M15 增加 txid `0x201`—`0x208` 的 provider-driven 十阶段动态生命周期、因果确认与 cleanup 后精确空闲拓扑。
- [x] 实现 M16 kind/opcode 驱动 manager 分派、txid `0x301`—`0x304` 的 post-cleanup round、固定空间 transcript 与精确空闲图复原。
- [x] 实现 PAN 保持开启、UAO 强制关闭的精确 exception-table usercopy，并证明未登记 current-EL fault 仍 fatal。
- [x] M31 all-or-none catalog 选择七个 pairwise-distinct AArch64 `ET_EXEC`（M13—M20 的四镜像与 M30 六镜像 catalog 为历史基线）；每个 fresh child 独占 address space、用户帧/栈与 16 KiB 内核栈，容量拒绝不消费 handle，monitor 只回收 terminal manager1 并以 generation +1 复用其 slot。
- [x] init 向内核发第一条 IPC。
- [x] M16 静态验证已通过：166 host tests、fmt、workspace check、default/all-feature AArch64 Clippy 与四个 userspace bin Clippy。
- [x] M16 完整 `./scripts/test.sh` exit 0：Debug EL1/EL2/max 与 Release EL1/EL2 五个 normal QEMU variant 及全部 negative/rollback/fault checks 通过；不可变 901712-byte Debug image 的 SHA256 为 `ee3d46e5a59f6c8e33809bc6a6e61a435818d11930729d027ee8033eb900516e`，修正 CR 归一化与外层汇总后同 SHA 的 16 路压力 runner exit 0、`passed=16 failed=0`。M15 SHA256 `25af0fa3d037ce37ff22242c6684fbf0de7631138505e6833adc97bc137d04ba` 仅为历史。
- [x] M17 完成 atomic sender identity、固定 PID ACL、delegated-endpoint 拒绝及对抗性 reducer；这些是历史安全基线，不等于产品 caller credential。
- [x] M18 完成同镜像双 Client 同时 outstanding、共享 ingress、每请求 private reply 与 secondary exit/reap；这些 private reply 只属于 M18 历史协议。
- [x] M19 完成 ABI v12 syscall 22 有界八项 wait-array，并以 183 项 host tests、双 Clippy、五种 normal QEMU 与全部 negative/rollback/fault checks 收口。
- [x] M20 完成两条独立常驻 Client session、session-routed reply、一次 stalled-secondary revoke/reattach/stale-lease 隔离及五进程精确空闲图；同一完整矩阵已从头通过。本轮没有重新固定 immutable image hash，也没有执行 16 路压力，且 `general_runtime=0`。
- [x] M21 完成 FDT 有界 virtio-mmio 发现、modern v2 `VERSION_1 | RO` feature negotiation、queue 8/two-DMA-frame 只读轮询驱动、双 sector digest 与越界预提交拒绝；199 项 host tests、双 Clippy、五种 normal QEMU 和含三项存储负测的完整 `./scripts/test.sh` 从头 exit 0。M20 服务证据保持不变，本轮仍未固定 immutable image hash、未做 16 路压力，且 `general_runtime=0`。
- [x] M22 完成 FDT `interrupt-parent`/GICv2 resolve、SPI target/trigger、queue 8/two-slot（stride 536、head 0/3）真实 two-outstanding、hard-IRQ completion 且 `poll_fallbacks=0`；另以 feature-gated race 验证 timeout/reset、两个旧 token 失效、late-spurious ACK 和无 double-completion recovery。208 项 host tests（`15/19/31/143`）、双 Clippy、五种 normal QEMU、三项存储负测及完整 `./scripts/test.sh` 从头 exit 0。仍未固定 immutable image hash、未做 16 路压力，且 `general_runtime=0`。
- [x] M23 完成确定性 8 MiB fixture、allocation-free block reader、protective MBR 与主备 GPT/entry CRC、FAT16 双镜像及有界 chain、`/system` 单只读 VFS 和两个文件 digest；正常路径为 142 parser/144 total IRQ reads。host suite 总计 226（`15/19/31/161`），storage negative 扩至六类；scheduler no-switch fault 只在 storage/heap evidence 后显式 arm。正常 BOOT 升为 M23，timeout/reset/late-spurious recovery BOOT 仍标 M22。未固定 immutable kernel image、未做 16 路压力，且没有写入、持久化、EL0 storage 或通用 runtime。
- [x] M24 完成 immutable `BootfsCatalog<2>`/VMO、move-only directory root、ABI v13 `FileOpenAt`/`VmoRead` 与内容/FNV/partial/EOF/bounds/bad-address/attenuation/stale/zero-payload-transfer 证明；aggregate ledger `588/455/132`，host suite 248（`16/19/31/0/182`），完整矩阵从头 exit 0。仍无 runtime disk read、mapping/shared memory、write、persistence 或通用 runtime。
- [x] M25 完成 private DATA partition、format-epoch-bound 双槽 CRC record、bounded raw WRITE/FLUSH/readback、跨 QEMU `0→1→2` 与 corrupt-newest recovery；host suite 269（`16/19/31/0/203`），仍为 `filesystem_write=0 crash_consistency=0 general_runtime=0`。
- [x] M26 完成 strict fw_cfg DMA/ramfb 与 virtio-input keyboard；host suite 283
  （`16/19/31/0/217`），完整矩阵输出 `BNDROID_TEST_SUITE_OK ... framebuffer=1 input_qmp=1 ...`，
  仍只有 static splash/keyboard 且 `general_runtime=0`。
- [x] M27 完成 opaque scene + alpha cursor compositor、dirty redraw 与独立 virtio-tablet；
  host suite 297（`16/19/31/0/231`），完整矩阵输出
  `BNDROID_TEST_SUITE_OK ... keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 ...`，
  仍没有用户态 surface/window/widget/text/gesture 栈且 `general_runtime=0`。
- [x] M28（历史）完成 kernel-owned clickable shell、Settings generation-1 damage commit 与
  `check-ui.sh` full-frame proof；host suite 323（`16/19/31/11/0/246`），完整矩阵输出
  `BNDROID_TEST_SUITE_OK ... clickable_ui=1 ui_screenshot=1 ...`，仍为
  `owner=kernel userspace_surface=0 general_runtime=0`。
- [x] M29 完成 ABI v14 EL0-owned single Surface、input `1..21`、frame `1..13` 与 ramfb/headless 验收；host suite 337（`16/19/31/17/0/254`），完整矩阵输出 `userspace_surface=1 userspace_ui=1`；该历史阶段 handles 总计 17、manager wait items 为 5。
- [x] M30 完成 ABI v15 六镜像与独立 SurfaceServer/Launcher UI IPC；当前七进程、18 endpoint/9 pair、19 handle，host suite 349（`16/19/31/29/0/254`），完整 `./scripts/test.sh` 从头 exit 0。
- [x] M31 完成 ABI v16 七镜像与独立 SurfaceServer/Launcher/App UI IPC；当前八进程、20 endpoint/10 pair、21 handle，默认 host suite 359（`16/19/31/39/0/254`）。默认/全 feature check 与 Clippy、五种 normal QEMU、全部负测、storage/persistence/IRQ race、24-input/13-commit capture baseline 与连续两次 36-input/19-commit Present cancellation/retry stress 均通过，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M32 完成 ABI v17 transferable GraphicsBuffer/buffer-present；八进程、20 endpoint/10 pair、23 handle，默认 host suite 377（`19/19/31/45/0/263`），五种 normal QEMU、完整负测/storage/persistence/IRQ race 与 24-input/13-commit buffer baseline/36-input/18-commit stress 均通过，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M33 完成 ABI v18 app lifecycle/minimal window management 专用 profile；ALC/USC=`35/14`、7 transactions、App1 graceful Exited/reap、App2 同槽 generation+1，process=`10/2/2/8`、29 handle、26 endpoint/13 pair、object/many/array pending=`8/2/6`，host suite 406（`20/19/31/73/0/263`）。
- [x] M34 完成 App crash/owner-death recovery 专用 profile；ALC messages/transactions/states/crashes=`46/10/19/1`、USC commands/acks/owner-deaths=`9/9/1`、PID generation `1→2→3`、process=`11/3/3/8`、termination reasons=`2/0/1`、terminate=`1/1`，29 handle、26 endpoint、8 exact wait token、abandoned=`0/0/1`、GraphicsBuffer generation 3；411 host tests、dedicated QEMU 与完整 `./scripts/test.sh` 均已通过。
- [x] M35 共享映射 GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；App RW/SurfaceServer RO 共享 75 页，416 host tests 与完整矩阵通过。
- [x] M36 consumer owner-death abandon/release + producer recovery；Acquired QEMU、Queued/Acquired host coverage、75 页重写和零最终 graphics mappings/handles 已通过。
- [x] M37 SurfaceServer 自动重启/rebind + resident mapped App 帧恢复；专用 QEMU 与完整从头矩阵均已通过。
- [x] M38 producer-death orphan + two-slot scrub/reuse；dedicated QEMU strict validator、checker、host suite 418 与完整矩阵均已通过。
- [x] M39/ABI-v20 software frame clock、single grant gating、三次 mapped commit 与最终 Ready opportunity；dedicated QEMU exact marker 已通过。
- [x] M40 resident two-buffer swapchain ownership/scheduling、ABA commit、post-copy release 与最终 B3 Acquired；dedicated QEMU/checker 已通过。
- [x] M41 bounded userspace multi-window compositor：z-order、occlusion、damage、raise、focus/input capture 与两槽输出已由 strict QEMU/checker 验证。
- [x] M42 持久事件驱动窗口会话、phone 边界拒绝、peer-close 清理、generation-safe App 重建与通用事件序列。
- [x] M43 focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44 SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45 独立 bounded InputServer、唯一 InputCapability、capacity-64 broker 与 Surface FIFO 零 fallback。
- [x] M46 SurfaceServer restart/rebind、route epoch `1→2`、gap release、App capture/contact cancel；dedicated QEMU、M45 regression、650 host/335 kernel 与完整总套件均已通过。
- [x] M47 InputServer 自身 restart/reacquire、route resync、fixed 30 ms backoff/budget 1、scoped audit/permission denial；dedicated QEMU、674 host/338 kernel 与完整总套件均已通过，quarantine 仅 host-verified。
- [x] M48 单 InputServer `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、process-exit 重启、第二次 health-timeout、budget-1 runtime quarantine、degraded UI 与 strict trace/validator/topology；694 host/338 kernel、dedicated 四阶段 QEMU、M45/M46/M47 隔离回归与完整总套件均已通过。
- [x] M49 `ServiceSupervisor::<2>` 依赖感知 SurfaceServer+InputServer；`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。
