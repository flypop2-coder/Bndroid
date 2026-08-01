# Bndroid OS 30000 字完整蓝图文档

> 本文件保留历史蓝图与里程碑背景，不再维护独立待办。当前精炼架构、模块合并规则
> 和未来任务统一见 [`TODO.md`](TODO.md)；已验证事实以
> `IMPLEMENTATION_STATUS.md` 和专项证据为准。

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

> 执行状态（2026-07-27）：历史 M71 里程碑为 ABI-v32/M71 `unified-product-continuous-supervision-runtime`。它严格扩展匹配的 M70 kernel/userspace closure，字面 kernel chain 为 M71→M70 `unified-product-psci-shutdown-runtime`→M69 `unified-product-multiservice-liveness-runtime`→M68 `unified-product-liveness-runtime`→M67 `unified-product-runtime`→M66 `resident-platform-shutdown-runtime`→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55。M71 不新增 syscall、镜像或 capability right，syscall 上限仍为 54；ABI v32 只更新证据契约。它保留历史 M70 的 strict FDT/PSCI 1.1、无 semihosting QEMU `SYSTEM_OFF`，并新增事务式五服务目录（ServiceManager、SurfaceServer、InputServer、StorageServer、App）、4 条依赖（3 hard、1 soft）、事务式批量 probe、每服务 missed-probe tolerance=1、16 轮额外健康 soak，以及 SurfaceServer/InputServer 同轮各漏一次后独立恢复；StorageServer 连续漏报仍走真实 100 ms timeout、30 ms backoff、同槽下一代 replacement 与 App block/resume。每次启动精确为 21 轮、107 probes、104 Healthy、3 missed，20 轮全健康、18 个 batch/90 个 batched probes、2 个瞬态恢复和 1 个升级故障；两次同盘启动都通过真实 UI/AppData closure 与 PSCI self-exit。它仍是 bounded、故障注入、single-core QEMU 研究原型，`arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是 PMIC、硬件 poweroff 或真机证明。feature-off/default 仍为 ABI v23，历史 M55—M64 为 ABI v25；M65/M66/M67/M68/M69/M70/M71 分别为 ABI v26/v27/v28/v29/v30/v31/v32。M33—M71 共有三十九个 opt-in leaf，加 default M32 为四十份独立账本；历史 M59 双账仍隔离。

> 历史 M56 是 kernel-only fail-stop recovery：`OutcomeUnknown`/`RequiresReset` 都是 session-fatal。旧 StorageServer 退出、session/capability 清理和 volume unbind 完成后，kernel 才执行 virtio status 0 reset，复核 device identity、相同 features/capacity，以仍独占的 DMA pages 重建 queue，并用 prepare/commit 两阶段重新 arm IRQ；旧 GIC pending/active 只在最初 disable 时清一次，re-enable 后保持 pending，并在 DAIF 屏蔽下完成 ISR/queue/status tail audit 后才 commit。失败则 rollback 并继续 fail-closed。新 StorageServer 只能以 owner epoch `+1` reacquire 并 remount durable volume，旧 session 不恢复。物理 commit 从不隐式开 admission。build wrapper 闭合 M66→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55 kernel chain；M65/M66 要求 kernel/userspace feature 匹配，M63/M64 userspace 继续被拒绝。

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

> 历史 M59 把同一四相位 cooperative physical engine 交给 AppData 与低层 timeout 两个独立 coordinator 使用；M58 StorageServer 也继续消费该 shared engine。ABI-v24 AppData child 只注入一次 read `QueueNotify` notification loss，先冻结 read 的 `RequiresReset` outcome，kernel 绝不重放原请求；userspace 只允许对零输出的 `FileOpenAt` `Unavailable` 重试一次，mutation/`OutcomeUnknown` 与第二次 `Unavailable` 一律不重试。每个 monitor turn 至多推进一个物理 phase，跨 turn 不保留 device/workspace borrow；物理完成后还必须观测真实 timer、双 worker、同一已认证 owner 的两次 wait 与两次不同 dispatch change，才可 rearm、显式 open admission 并 finalize。其成功历史实测为：

```text
APPDATA_ASYNC_RECOVERY_OK abi=24 cases=1 fault_order=R read_requires_reset=1 injected_reads=1 unavailable_read_retries=1 blind_mutation_retries=0 recovery_starts=1 physical_completions=1 recovery_commits=1 recovery_failures=0 async_steps=4 physical_pending_returns=3 coordinator_yields=5 step_completion_relation=1 timer_progress_windows=1 worker_progress_windows=1 el0_progress_windows=1 authenticated_waits=2 wait_dispatch_changes=2 driver_timeouts=1 driver_resets=1 masked_poll_iterations=0 max_step_masked_ticks=53500 max_control_masked_ticks=86187 timer_period_ticks=625000 long_daif_masks=0 final_active=0 gate_open=1 invariant_errors=0
APPDATA_RUNTIME_OK abi=24 phase=created version=1 boot_generation=0 committed_generation=2 entries=2 files=1 directories=1 submissions=11 completions=11 retrievals=11 mutations=2 reads=1 lists=3 conflicts=1 expected_terminal=4 disk_reads=6682 disk_writes=580 disk_flushes=4 old_or_new=1 full_readback=1 resident=1 errors=0 async_recovery=1
BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified
```

> ABI-v23 低层 timeout 自测也改用 shared engine。三个 `Pending` 返回之间都跨过真实 timer 与双 worker 进度，物理完成后显式 rearm/open，旧 `STORAGE_IRQ_RACE_OK` 精确证据保持不变；该内核自测没有 EL0 waiter，因此明确记录 `el0_progress_claim=0`：

```text
STORAGE_IRQ_COOPERATIVE_RECOVERY_OK starts=1 physical=1 steps=4 yields=3 step_yield_relation=1 timer_progress_windows=3 worker_progress_windows=3 timer_dispatches=9 worker0_work=539135 worker1_work=376383 masked_poll_iterations=0 max_step_masked_ticks=59375 max_control_masked_ticks=87188 timer_period_ticks=625000 long_daif_masks=0 final_active=0 gate_open=1 el0_progress_claim=0
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

> 历史 M64 在 M63 上增加 exact-session clean close 与 no-later-storage 边界。kernel 在 fresh live probe 后把刚打开 health session 的 generation/epoch 作为仅驻内存能力保存；只允许该精确 generation 在 live contract 仍匹配、device healthy/idle、无 in-flight request 时向 inactive slot 写 `boot_open=false`，随后 flush、完整 readback 并确认旧 slot 未变。

> M64 directed runtime 在任何 EL0 进程启动前完成 close，随后屏蔽本地 IRQ、封闭 storage admission、禁用 logical/physical block IRQ、复核所有 request terminal，并证明没有后续 storage mutation 后 halt。同一镜像第二次启动从 prior closed 记录重新 open、fresh probe，再 close 到 generation 4；`boot_open=false` 只移除未闭合 hint，绝不替代 live probe。它没有 EL0 shutdown control，不持久化/恢复 Offline，也不声称完整 userspace shutdown、PSCI/硬件 poweroff、真实 power-cut、SMP 或 general runtime。当前 log 为 83 行、15855 字节，SHA-256 为 `a1d620dc22bae3853c1ac02ea40e24a65a6c3e7ac2930ab53cccaa379564c113`：

```text
STORAGE_CLEAN_SHUTDOWN_REBOOT_OK boots=2 legacy_upgrades=1 clean_closes=2 prior_closed=1 unclosed_hints=0 reprobe_required=0 reprobe_verified=2 contract_changes=0 final_generation=4 final_slot=0 prior_generation=3 prior_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_unchanged=1 appdata_unchanged=1 unused_data_unchanged=1 changed_data_bytes=153 offline_persisted=0 offline_from_record=0 el0_started=0 el0_controls=0 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M64 kernel-owned clean boot-session close and no-later-storage boundary verified
```

> 历史 M65 使用 ABI v26/syscall 53 实现 init-only 两阶段 shutdown。两个固定 workload client 退出并被回收、StorageServer idle、broker/物理 I/O terminal 后，`Prepare` 才关闭新进程与 StorageAcquire/Connect/Accept admission；client 的 shutdown 调用和 Prepare 后 spawn 均被拒绝。StorageServer 执行最终 flush、同 generation 全量 recover/readback、ACK、正常退出；init 回收后发布精确 `InitReady` 并 `Commit`。kernel monitor 再验证全部账本，调用 M64 durable close，封闭 storage/IRQ/shutdown gate 后 halt。

> 同一 writable image 双启动证明 AppData generation 5→6、health `1→2`/`3→4` clean close、两次 server flush/readback/exit，且 DATA/APPDATA 外字节不变。当前 log 为 91 行、18869 字节，SHA-256 `2c338c42a97a869375ccca50ec33cd8d84556b71cbcf7f56bffe8c44be58c645`：

```text
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_REBOOT_OK boots=2 userspace_shutdowns=2 clients_drained=4 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 offline_persisted=0 offline_from_record=0 el0_started=1 el0_controls=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M65 userspace StorageServer shutdown orchestration and durable close verified
```

> M65 仍只是 single-core storage-profile directed proof，没有完整 resident UI/service graph、PSCI/硬件 poweroff、真实断电、SMP 或 general runtime。

> 历史 M66 在 M65 上加入 ABI v27/syscall 54 `ServiceShutdown`。init 同时启动 StorageServer 与八个认证 resident 节点；Prepare 前 kernel 直接核验 10 个 live process、9 对 init control、10 对 dependency Channel、38 个 endpoint、39 个 handle、rights/空队列、唯一 StorageVolume 与 PID/image/node 绑定。提前 quiesce SurfaceServer 被拒后，按 `Primary/Secondary/Launcher/App→Provider/InputServer→ServiceManager/SurfaceServer` 三波逆拓扑关闭；Launcher/App 还执行真实 AppData workload。

> 九个 child 全部 exit/reap、StorageServer final flush/readback、M65 Commit、M64 durable close、storage/shutdown seal、block IRQ disable 与本地 IRQ mask 完成后，kernel 才创建 opaque `ValidatedShutdown` token。M66 唯一 backend 是 AArch64 QEMU semihosting `SYS_EXIT_EXTENDED`；同一 writable image 两次启动都必须由 QEMU 自身 status 0 退出，host kill 不算成功。该历史 89 行/19200 字节日志 SHA-256 为 `656186fb9e4275bed2f64d95484160e97cdbe7960a74f90e209f566b7aff81de`：

```text
RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed
```

> 这只是 fixed-graph、single-core、emulator-only proof；不是完整产品 UI runtime，也不是 PSCI/PMIC/硬件 poweroff，不证明真实掉电、SMP、general runtime 或真机。

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

> 历史 M71 入口为 `CARGO_NET_OFFLINE=true BNDROID_PROFILE=release ./scripts/check-unified-product-continuous-supervision-runtime.sh` 与 `CARGO_NET_OFFLINE=true ./scripts/check-unified-product-continuous-supervision-static.sh`；M70 及更早 checker 保持隔离。M71 parser 自测 positive/serial-negative/host-negative=`4/18/3`，持久日志为 `target/m71/unified-product-continuous-supervision-runtime.log`。

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

> 历史 M71 阶段的 83 个顶层 shell scripts 内共有 52/52 个 QEMU launch，均恰有一个 `-nic none`。M67—M70 的隔离 source/feature/parser/双启动 gate 当时全部保留；M71 新增 continuous-supervision source/feature/parser/双启动 gate。该阶段完整 suite 包含 `unified_product_continuous_supervision_static=1 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6`；M70/M71 的 PSCI 双启动合计 `qemu_psci_self_exits=4`，M66—M71 六份双启动 self-exit 账本合计 `qemu_self_exits=12`。

> 2026-07-27 历史 M71 当时最终源码上的完整离线 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已 exit 0，精确终态为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 app_data_async_recovery=1 storage_server_static=1 storage_server_recovery_static=1 storage_server_repeated_recovery_static=1 storage_server_async_recovery_static=1 storage_server_fault_policy_static=1 storage_server_owner_liveness_static=1 storage_server_terminal_quarantine_static=1 storage_server_persistent_health_static=1 storage_server_clean_shutdown_static=1 storage_server_shutdown_orchestration_static=1 resident_platform_shutdown_static=1 unified_product_static=1 unified_product_liveness_static=1 unified_product_multiservice_liveness_static=1 unified_product_psci_shutdown_static=1 unified_product_continuous_supervision_static=1 storage_recovery_unification_static=1 storage_server_runtime_boots=3 storage_server_recovery=1 storage_server_repeated_recovery=1 storage_server_async_recovery=1 storage_server_fault_policy=1 storage_server_owner_liveness=1 storage_server_terminal_quarantine=1 storage_server_persistent_health_reboot=1 persistent_health_boots=2 storage_server_clean_shutdown_reboot=1 clean_shutdown_boots=2 storage_server_shutdown_orchestration_reboot=1 shutdown_orchestration_boots=2 resident_platform_shutdown_reboot=1 resident_platform_shutdown_boots=2 unified_product_reboot=1 unified_product_boots=2 unified_product_ui_interactions=2 unified_product_liveness_reboot=1 unified_product_liveness_boots=2 unified_product_liveness_recoveries=2 unified_product_multiservice_liveness_reboot=1 unified_product_multiservice_liveness_boots=2 unified_product_multiservice_liveness_recoveries=2 unified_product_psci_shutdown_reboot=1 unified_product_psci_shutdown_boots=2 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6 qemu_psci_self_exits=4 qemu_self_exits=12 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1 storage_irq_cooperative_recovery=1
```

> 同一次完整回归还允许 M51 `GraphPrepared` 后 Launcher/App 两个已认证 rebind ACK 以任一合法串行顺序到达；每角色 exact-once、PID/image/token 与 payload 校验没有放宽。

> 历史 M55 authority split 保持封印：canonical StorageServer 独占 `StorageAcquire`，volume/session 是 `READ|WRITE|WAIT` 且无 `DUPLICATE|TRANSFER`，client 只可请求 READ/WRITE 非空子集；kernel broker 只管理 raw-sector owner/epoch/session/token/bounds/completion/cleanup，EL0 StorageServer 执行 path namespace 与 AppData COW volume policy。精确边界为 `kernel_namespace_ops=0 namespace_policy=0 block_driver=kernel`。

> raw block request 是 strict `SBRQ` v2、exact 4160 bytes（64-byte header + 4096 data），offset 24 为 little-endian `u16 sector_count`。read/write count 1—8，所有 reserved、read data、write unused tail 必须为零；flush 只能是 LBA/count/data 全零，并以 checked add 保证 `lba + count <= 1920`。server read-ahead/cache 与 contiguous-write coalescing 上限均为 batch 8；batch 不是原子 multi-sector transaction，底层仍逐扇区执行。

> 历史 M55 logical timer 保持 100 Hz；storage completion 只把 physical compare 临时拉近为 one-shot immediate IRQ，并在正常 exception-return boundary 给 newly-woken context 一次 preference。真实 timer IRQ、TrapFrame、TTBR/translation 与 stack switch 已覆盖（`trapframe_switch=1`）。精确账式是 `completion_scans=candidate_scans+no_candidate_scans`、`candidate_scans=preferred_requests+preferred_coalesced`、`preferred_requests=preferred_dispatches+stale+preferred_pending`；ready 时 `stale=0 preferred_pending=0`，并满足 `0<immediate_interrupts<=immediate_requests<=I/O completions`。completion 可能早于 client 进入 `StorageTake` 阻塞态，此时没有 candidate waiter；旧的 `preferred_requests==all I/O completions` 是错误不变量。不能把 one-shot 写成提高 logical tick。M55 的 idle restart 不等于 M56 的 session-fatal in-flight reset。

> 历史 M55 三次同镜像启动形成 AppData generation `0→4→5→5`。boot 1 的 read batches/write batches/flushes/completions=`1425/289/12/1726`，boot 2=`765/45/2/812`，boot 3=`606/0/0/606`；对应 read/write sectors 为 `11400/2121`、`6120/290`、`4848/0`，三次 `max_batch_sectors=8 errors=0 bounds_rejections=0`。boot 3 零 AppData write/flush；boot 2/3 AppData partition SHA-256 同为 `d105d3eecbeee5e77774c1d37f83e11406e50e020d6abd48c0b4a56f9980b089`。whole disk 每次因 `BNDROID_DATA` boot counter 变化，稳定证明范围只能是 `BNDROID_APPDATA` partition。

> M55—M70 与 M59 双账本是历史前缀，历史 M71 由隔离 checker 封口。M70 继承固定双服务 dependency-liveness，并把最终 QEMU exit 绑定到 strict FDT `/psci`、PSCI_VERSION 1.1 与 HVC SYSTEM_OFF；M71 再加入事务式五服务/四边目录、批量 probe、16 个额外健康轮、同窗两服务瞬态漏报恢复和一次升级 StorageServer replacement。它仍是 bounded single-core QEMU `arbitrary_soak_claim=0 general_runtime=0 real_phone_claim=0` research prototype，不是真手机、不可刷机、不可日用，也不证明 PMIC/hardware poweroff。下一硬件 P0 必须先由用户指定并授权目标，再做 BSP、启动链、真实控制器/PMIC、真实 power-cut、SMP/IOMMU 与真机验证；目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak 仍待推进。

> 紧随其后的 `BNDROID_TEST_SUITE_OK` 整行是历史 M55 完整套件的唯一终态 seal；M56—M66 与 M59 的各自证据不能彼此替代。

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

> ABI-v22 原样保留 syscall 0—38，复用 `READABLE`/`PEER_CLOSED`，新增 register-only syscall 39 `InputAcquire` 与 40 `InputReadEvent`。只有 live InputServer 可取得唯一、不可复制/转移、`READ|WAIT=0x101` 的 InputCapability。capacity-64 broker 终态 enqueued/dequeued/pending=`59/59/0`、high-water 1、coalesced 0、sequence `1—59`/next 60、physical=`pointer47+key12`；Surface pointer/key FIFO reads 为 0。strict canonical 64-byte `BIC1`/`BIE1` 使 InputServer 拥有 route/focus/capture/text-context/IME，SurfaceServer 保留 compositor/window/output。

```text
INPUT_SERVER_KERNEL_OK abi=22 pid=4294967302 session=1 capacity=64 enqueued=59 dequeued=59 pending=0 high_water=1 coalesced=0 sequence_next=60 process_capacity=9 dynamic_capacity=8 scheduler_contexts=12 route=broker-only surface_fifo=0
INPUT_SERVER_OK abi=22 protocol=1 wires=BIC1/BIE1 wire=64 owner=unique pid=4294967302 session=1 capacity=64 events=59/59/0 high_water=1 coalesced=0 sequence=1-59 next=60 physical=pointer47+key12 surface_fifo=0/0 legacy_key_reads=0 routes=generation-qualified focus=server capture=server ime=server processes=10/1/1/9 process_capacity=9/8 input_server=1/3 handles=36 endpoints=30 pairs=15/2 waits=9/2/7 topology=resident final_state=ready final_app_resident=1
BOOT_OK: M45 dedicated InputServer routing, capture, and input-method ownership verified
```

> `scripts/check-input-server.sh` 与 `target/bndroid-m45-input-server-visible.ppm`、`target/bndroid-m45-input-server-hidden.ppm`、`target/bndroid-m45-input-server.ppm` 已通过；visible/final SHA-256 为 `2b140efbc26f0f48cb7c0b33e0299a2e24d4720bc207fd780490c10741f5fcf3`/`1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994`。host 为 599 default（`26/51/19/64/31/112/0/296`）+24 directed（`11 ui_trace + 10 window_trace + 3 persistent`）=623 executed/unique，input-server kernel 334；完整 suite 新增 `input_server=1`。

> M45 是独立有界系统服务，仍非产品级 InputServer/IME；缺 candidate/locale、grapheme、shaping/font、任意 Unicode、多点、物理手机硬件、restart/backoff/audit/permission integration；它的精确 marker、截图和 623/334 历史账本保留如上。

> **M46 历史完整封口**

> M46 不改变 ABI：仍为 v22，syscall 0—40 全部不变。它是从冻结 M41 checkpoint 独立启动的 leaf，不串接 M42—M45 的物理输入 transcript；确定性 M41 semantic replay 精确为 trace/command/event/output=`23/9/14/6`，该前缀不进入 kernel broker。strict `BIR1` route transcript 为 8 条；`BSR1` restart transcript 为 11 条，六种消息均为 fixed 64 bytes，方向计数 Surface→Init/Init→Surface/Surface→Client/Client→Surface=`4/1/3/3`。Surface pointer/key FIFO=`0/0`，legacy key reads=`0`。

> InputServer 在 route epoch 1 保存 App capture 与 sequence floor 2；旧 Surface session 1 在 App contact established/ack=`1/1` 后于 recovery frame 7 冻结并退出，同一 process slot 的 replacement Surface 以 PID generation+1、session 2 重建。gap 中唯一 release 被排队；route epoch `1→2` 后按 stale release 忽略且不路由，App contact cancel=`1`、终态 `none`。broker enqueued/dequeued/pending/high-water=`3/3/0/1`，gap release=`1`。终态 process created/exited/reaped/live=`11/2/2/9`，handle/endpoint/pair=`36/30/15`，exact waits object/many/array=`9/2/7`。

> 图形恢复严格区分旧 Surface session 1 frame 7 与新 session 2 frame 1，累计 output frame=`8`；pool epoch=`2`、per-slot=`[1,0]`，replacement 重新取得冻结 scanout 并保留可见 released cursor。精确运行 marker 为：

```text
INPUT_SERVER_CAPTURE_CANCEL_OK old_epoch=1 new_epoch=2 target=app cancel=1 stale_release=ignored routed=0 text_delta=0 client_contact=1/1/1 final=none
BOOT_OK: M46 InputServer SurfaceServer restart rebind, route-epoch gap recovery, and capture cancellation verified
```

> `scripts/check-input-server-surface-restart.sh` 已验证 pre/frozen/post 三相恢复；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`。dedicated M46、M45 `scripts/check-input-server.sh` regression 与完整 suite 都从头 exit 0；host 账本为 626 default + 24 directed = 650 executed/unique，M46 kernel 为 335，suite 终态固定 `input_server_surface_restart=1`。

> M46 只证明固定一次会话、单 App capture、一次 SurfaceServer 重启的 bounded boot witness，不是通用恢复或现实可用手机系统；其 ABI-v22 marker、截图与 650/335 历史账本保留如上。

> **M47 历史完整封口**

> M47 将 head 推进到 ABI-v23：syscall 0—40 原样保留，只新增 register-only syscall 41 `InputSessionInfo`，以 `(InputCapability handle, 0, 0)` 返回当前 `session` 与 `acquisition_floor`。strict canonical fixed-64-byte `BIR1`/`BIP1`/`BIC1`/`BIE1` 固定 Init、SurfaceServer 与两代 InputServer 的恢复边界。SurfaceServer 越权调用 `InputAcquire` 被精确拒绝为 `permission_denied`，audit=`1`，handle/session delta 均为 0，没有产生隐式 capability 或会话。

> runtime 仅执行一次重启：旧 InputServer 位于 session/route epoch `1/1`，退出后 broker 保持 unbound 且 pending=`0`；Init 在不提前 spawn 的情况下精确等待 30,000,000 ns，restart budget 为 `1/1`，随后 replacement InputServer 以 session/epoch `2/2` reacquire。Surface session 始终为 1，通过 snapshot + 两条 route 重建 launcher/app，终态 focus/capture/text 为 `app/none/none`。broker enqueued/dequeued/pending/high-water=`3/3/0/1`，release=`1`，unbound drop=`0`；process created/exited/reaped/live=`11/2/2/9`，handle/endpoint/pair=`36/30/15`，exact waits object/many/array=`9/2/7`。

```text
INPUT_SERVER_RESTART_PERMISSION_OK caller=surface syscall=input_acquire status=permission_denied audits=1 handles_delta=0 sessions_delta=0
INPUT_SERVER_RESTART_GAP_READY floor=1 broker=unbound surface=alive pending=0
INPUT_SERVER_RESTART_BACKOFF_OK attempt=1 requested_ns=30000000 timeout=1 early_spawn=0 budget=1/1
INPUT_SERVER_RESYNC_OK snapshot=1 routes=2 focus=launcher capture=none text=none floor=1 bic=6 bie=7
INPUT_SERVER_RESTART_ROUTE_OK sequence=2-3 target=app focus=launcher/app capture=down/up final_capture=none text_delta=0
INPUT_SERVER_RESTART_OK abi=23 protocol=1 wires=BIR1/BIP1/BIC1/BIE1 sessions=1/2 surface_session=1 epochs=1/2 restart=1 backoff=fixed-30ms budget=1/1 quarantine=host-verified broker=3/3/0/1 releases=1 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/2/2/9 handles=36 endpoints=30 pairs=15 waits=9/2/7 errors=0
BOOT_OK: M47 bounded InputServer restart, backoff, Surface resync, and permission denial verified
```

> `scripts/check-input-server-restart.sh` 已验证 pre/gap/post 三相；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`。host 账本为 650 default + 24 directed = 674 executed/unique，M47 kernel 为 338；dedicated M47、M45/M46 regression 与完整 `./scripts/test.sh` 均从头 exit 0，suite 终态固定 `input_server_restart=1`。

> `quarantine=host-verified` 只表示 M47 策略的重复 crash 终止分支已由 host test 验证；M47 QEMU runtime 仅证明一次 InputServer restart。其 ABI-v23 marker、三截图、650 default + 24 directed = 674 host 与 338 kernel 历史账本严格保留，不能用 M48 的 runtime quarantine 或 694 host 新账本回写。

> **M48 历史完整封口**

> M48 保持 ABI v23 与 syscall 0—41 完全不变，也不增加镜像、process capacity 或 scheduler context。独立 `service-supervisor-runtime` 是第十六个 leaf profile；运行时只注册一个 InputServer，并且源码只实例化 `ServiceSupervisor::<1>`，因此它是单服务 bounded witness，不得表述为 SurfaceServer+InputServer 多服务监督已经完成。

> `bndr-sm` 的测试从 31 增至 51。新增 allocation-free、fixed-capacity `ServiceSupervisor` 与 strict canonical little-endian fixed-64-byte `BSH1`：service kind、process generation、generation-qualified PID、双向 sequence、opcode、flags、fault class、restart attempt/budget、interval 以及全部 reserved/padding 在状态推进前 fail closed。kernel trace 只接受精确 sender/direction/order 的 canonical transcript；QEMU validator 还严格核对 marker schema/顺序、same-slot PID generation `+1`、无陈旧 leaf marker、精确 resident topology 与 phase-specific pixel ledger。

> runtime 首先把旧 InputServer 的退出分类为首个 `process-exit` fault，以 attempt 1/budget 1、fixed 30 ms backoff 启动 replacement，并复用 M47 的 session/epoch `1→2`、route resync 与 broker gap 边界。replacement 对 Probe sequence 1 返回匹配的 Healthy；第二个 Probe 已被精确 dequeue 后故意静默，第二次 200 ms deadline 因此形成 `health-timeout`。attempt 2 超出 budget `1/1` 后不再 spawn：Init 发送 canonical Quarantine，终止并 reap unhealthy replacement，broker 保持 unbound，SurfaceServer 常驻并回送 DegradedAck、提交 frame 9/write generation 6 的红色 `32×24` degraded badge。终态 process created/exited/reaped/live=`11/3/3/8`，handle/endpoint/pair=`31/26/13`，object/many/array waits=`8/2/6`。

```text
SERVICE_SUPERVISOR_ARMED wire=BSH1 input_pid=<generation-qualified-pid> generation=1 session=1 epoch=1 surface_session=1 budget=1
SERVICE_SUPERVISOR_FIRST_FAULT_OK class=process-exit attempt=1 budget=1 gap=1 broker=unbound
SERVICE_SUPERVISOR_RESYNC_READY sessions=1/2 epochs=1/2 routes=2 floor=1 bic=6 bie=7
SERVICE_SUPERVISOR_HEALTH_OK probes=1 healthy=1 input_pid=<generation-qualified-pid> generation=2 timeout_ns=200000000
SERVICE_SUPERVISOR_WATCHDOG_OK probes=2 healthy=1 requested_ns=200000000 timeout=1 class=health-timeout
SERVICE_SUPERVISOR_QUARANTINE_OK attempt=2 budget=1 reason=restart-budget-exhausted degraded=1 frame=9 generation=6
SERVICE_SUPERVISOR_OK abi=23 protocol=1 wire=BSH1 faults=process-exit/health-timeout probes=2/1 watchdog=fixed-200ms restart=1 budget=1/1 quarantine=runtime degraded=1 broker=unbound releases=2 unbound_drops=0 surface_reacquires=0 surface_fallback=0 processes=11/3/3/8 handles=31 endpoints=26 pairs=13 waits=8/2/6 errors=0
BOOT_OK: M48 generic ServiceSupervisor watchdog, runtime quarantine, and degraded UI verified
```

> `scripts/check-service-supervisor.sh` 已验证 `target/m48/service-supervisor-{pre,gap,recovered,degraded}.ppm` 四个互异阶段；SHA-256 依次为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`、`32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`。checker 精确验证 badge/cursor damage、trace marker schema 与顺序、generation-qualified identity、broker-only 输入和 `31/26/13`、`8/2/6` topology，最终 seal 为：

```text
SERVICE_SUPERVISOR_QMP_OK armed=1 first_fault=process-exit resync=1 health=1 watchdog=health-timeout quarantine=runtime degraded=1 broker=unbound pointer=136/184/move-down-up screenshots=pre-gap-recovered-degraded pre_sha256=9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992 gap_sha256=c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324 recovered_sha256=5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65 degraded_sha256=32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff markers=armed/fault/resync/health/watchdog/quarantine/runtime/boot
```

> 历史 M48 当时的 default workspace 为 670（ABI/compositor/ELF/input/SM/UI/init/kernel=`26/51/19/106/51/120/0/297`），其中 `bndr-sm` 相对 M47 从 31 增至 51；另有 `11 ui_trace + 10 window_trace + 3 persistent = 24` directed feature-filter tests，合计 694 executed/unique host tests，`service-supervisor-runtime` kernel all-tests 为 338。dedicated M48 QEMU、M45/M46/M47 隔离回归、静态构建/Clippy matrix 与最终 `CARGO_NET_OFFLINE=true scripts/test.sh` 全量复跑均已 exit 0。精确 suite marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M48 仍只监督一个 InputServer，绝非通用多服务恢复或现实可用手机系统；以上内容现作为历史封口保留。

> **M49 历史完整封口**

> M49 保持 ABI v23 与 syscall 0—41 完全不变。独立 `service-dependency-runtime` 实例化 `ServiceSupervisor::<2>`，同时注册 SurfaceServer 与 InputServer，并建立唯一的 `InputServer -> SurfaceServer` soft dependency。服务自身故障进入 hard-blocked；InputServer 故障把仍存活的 SurfaceServer 推入 soft-degraded，而 SurfaceServer 故障不终止仍健康的 InputServer。固定容量 DAG 对 self/duplicate/cycle、未注册节点、过期 identity 和错误恢复顺序全部 fail closed。

> QEMU 先让 Surface generation 1 在 pointer-down checkpoint 后退出；Input generation 1 跨 route gap 存活，同槽 Surface generation 2 以 session 2、route epoch 2、floor 3 恢复并提交 teal frame 1。真实 2 s finite wait 固定该阶段，Surface 随后在 100 ms deadline 内返回 `Healthy`。Input generation 1 再真实 dequeue Probe 但不回 `Healthy`，100 ms watchdog 分类 `health-timeout`；Surface 保持存活并提交 frame 2/write generation 2 的红色 32×24 route-lost badge。真实 2 s degraded hold 与 fixed 30 ms restart backoff 后，Input generation 2 同槽启动，以 session 2、route epoch 3/floor 3 完成 BIR/BIP、六 BIC/七 BIE snapshot；Surface 提交 distinct green frame 3，Input2 返回 `Healthy`，两个 impact 收敛为 `unaffected`，没有 restart storm。终态 process=`12/3/3/9`、handle/endpoint/pair=`36/30/15`、wait=`9/2/7`。

```text
SERVICE_DEPENDENCY_ARMED services=2 dependency=input-soft-surface surface_pid=<generation-qualified-pid> input_pid=<generation-qualified-pid> surface_generation=1 input_generation=1 session=1 epoch=1 floor=0 budget=1
SERVICE_DEPENDENCY_SURFACE_GAP fault=process-exit surface_generation=1 attempt=1 input_alive=1 route_epoch=2 floor=2
SERVICE_DEPENDENCY_SURFACE_RECOVERED surface_generation=2 surface_session=2 input_generation=1 input_session=1 route_epoch=2 floor=3 frame=1
SERVICE_DEPENDENCY_SURFACE_HEALTHY probes=1 healthy=1 surface_generation=2 timeout_ns=100000000
SERVICE_DEPENDENCY_WATCHDOG service=input generation=1 probes=1 reads=1 healthy=0 timeout_ns=100000000 impact=input-hard/surface-soft
SERVICE_DEPENDENCY_DEGRADED surface_alive=1 input_alive=0 phase=route-lost frame=2 write_generation=2 floor=3
SERVICE_DEPENDENCY_INPUT_REBOUND input_generation=2 input_session=2 route_epoch=3 floor=3 bic=6 bie=7
SERVICE_DEPENDENCY_INPUT_HEALTHY probes=1 reads=1 healthy=1 input_generation=2
SERVICE_DEPENDENCY_STABLE cadence_ns=2000000000 restarts=1/1 impacts=unaffected/unaffected created=12 exited=3 reaped=3 live=9
SERVICE_DEPENDENCY_OK abi=23 protocol=1 services=2 dependency=input-soft-surface health_messages=5 probes=3 probe_reads=3 healthy=2 watchdog=1/1 bir=8+3 bip=4 bic=6 bie=7 surface=1/2 input=1/2 sessions=1/2 epochs=1/2/3 floor=0/2/3 frames=1/2/3 outputs=3 restarts=1/1 impacts=unaffected/unaffected created=12 exited=3 reaped=3 live=9 reasons=exited1/killed2 handles=36 endpoints=30 pairs=15 waits=9/2/7 topology=resident final_state=ready
BOOT_OK: M49 dependency-aware SurfaceServer and InputServer supervision verified
```

> `scripts/check-service-dependency.sh` 默认导出 `CARGO_NET_OFFLINE=true`，只使用本地 Unix QMP，并以 `-nic none` 显式禁用 QEMU 网络。pre/gap/surface-recovered/degraded/recovered 五阶段 SHA-256 依次冻结为 `9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992`、`767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14`、`5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65`、`32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff`、`a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d`。M49 历史 suite 精确整行为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M49 仍只是单核 QEMU 上固定两服务、单 soft edge、固定窗口与脚本故障的研究/模拟器系统，不是生产手机系统，也未达到真机可用。任意 App/window、产品级 IME/candidate/locale/font shaping/Unicode/multitouch、网络/蜂窝/Wi-Fi、音频、电源、安全启动、沙箱、安全更新、产品驱动与真实硬件闭环仍缺。

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

> M51 保持 ABI v23、syscall 0—41、八镜像 all-or-none catalog、process capacity 9 与单核 12-context 不变，并完整重放 M49/M50 前缀。M50 output frame 4 稳定后，screen `(80,96)` 的 Launcher down/up 形成 physical sequence `8/9`：down 由 Launcher 路由并 capture，focus 从 `app/3` 推进到 `launcher/4`，严格 BIC command sequence 为 3；release 后 capture 清零。该扩展的 channel writes/reads 精确为 `8/8`，Launcher 随后提交 Present command 6/frame 4，Surface 提交 scene 11、output frame 5、write generation 5；Surface/Input health 保持 `2/2`，两服务终态 resident。

```text
POST_RECOVERY_FOCUS_ARMED abi=23 surface_session=2 input_session=2 route_epoch=3 physical_floor=7 focus=app/3 output_frame=4
POST_RECOVERY_LAUNCHER_CAPTURED physical=8 target=launcher events=1 focus=launcher/4 capture=1
POST_RECOVERY_LAUNCHER_PRESENTED physical=8/9 command=6 launcher_frame=4 scene=11 output_frame=5 write_generation=5 focus=launcher/4 capture=0
POST_RECOVERY_FOCUS_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=8..9 target=launcher events=2 capture=0 launcher_present=1 output_frame=5 focus=launcher/4 health=2/2 resident=1 errors=0
BOOT_OK: M51 post-recovery App-to-Launcher focus and frame verified
```

> `scripts/check-post-recovery-focus.sh` 默认强制 `CARGO_NET_OFFLINE=true`、仅连接本地 Unix QMP，并以 `-nic none` 禁用 QEMU 网络。它冻结 focus 前后的 framebuffer PPM damage rect=`64/80/40/32` 与 `1280` 个差异像素；armed SHA-256 为 M50 presented 的 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`，presented SHA-256 为 `cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729`。checker 的终态封印为 `POST_RECOVERY_FOCUS_QMP_OK pointer=80/96/down-captured-up target=launcher focus=app3-launcher4 output=4-5 diff_pixels=1280 damage=64/80/40/32 armed_sha256=798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974 presented_sha256=cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729 markers=m49-prefix/m50-prefix/focus-armed/launcher-captured/launcher-presented/focus/boot`。

> 默认关闭的 `ui-stale-present-evidence` 只由 directed `scripts/check-ui.sh` 显式启用，不改变默认/生产路径：24 input/13 commit baseline 后，App frame 10 在 SurfaceServer 已读取 focus generation 8 时被确定性挂起，Home generation 9 通过正常协议取消它，generation 10 用相同 local frame id 重试，最终 Home generation 11 收敛为 36 input/18 个连续 commit。`kernel/build.rs` 还对每个外部 override ELF 路径注册内容变化监听；即使路径不变，ELF 重建也会触发重新嵌入，避免沿用陈旧 userspace 镜像。

> M51 历史完整 `./scripts/test.sh` 已从头 exit 0；精确终态 marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M51 仍只是单核 QEMU、固定两服务、固定 Launcher/App 双窗口与脚本化交互的有界研究实现，不是真实手机，也没有硬件网络。蜂窝/电话、Wi-Fi/网络、音频、电源、产品驱动、安全启动、应用沙箱、安全更新、产品级 IME/Unicode/multitouch、任意 App/window 与真机闭环均未完成。

> **M52 历史完整封口**

> M52 保持 ABI v23、syscall 0—41、八个 pairwise-distinct AArch64 `ET_EXEC` 的 all-or-none catalog、process capacity 9 与单核 12-context 不变；显式 opt-in 的 `post-recovery-focus-roundtrip-runtime` 完整重放 M49 dependency recovery、M50 App interaction 与 M51 Launcher focus 前缀。M51 终态精确为 InputServer event 14、physical 9、route sequence 4、focus sequence 3、Launcher focus generation 4、dependency snapshot step 7，以及 output frame/recovery epoch/write generation `5/5/5`；Surface/Input health 仍为 `2/2` 且 resident。M52 不增加 syscall、signal bit、镜像、capability 或 scheduler context。

> 本地 Unix QMP 在 screen `(136,184)` 注入 App down/up。physical 10 down 产生 BIE event 15；SurfaceServer 将其换算为 compositor logical `(80,120)`、App local `(32,40)`，capture App，并把 focus 从 `Launcher/4` 推进到 `App/5`。SurfaceServer 随即发送 BIC `SetFocus` sequence 4，目标 WindowRef=`id2/gen1`；InputServer 以 BIE event 16 ACK，focus sequence `3→4`、dependency snapshot step `7→8`。physical 11 up 产生 BIE event 17 并清除 capture。App 收到 command 5、scene 11、focus generation 5 的 down/up 两条 BWE InputRoute，然后提交 BWC Present command 6/frame 4、local damage `(56,72,40,32)`、color `0x00844ec7`；content generation 推进到 8，Presented 位于 scene 12，最终 output frame/recovery epoch/write generation=`6/6/6`。

> BIC 的 wire ACK 是上述 BIE event 16；App 消费 Presented 后另发的 sender-authenticated private 8-byte completion ACK 才使用 magic `0x4d35325f4150434b`。该私有确认晚于 17-bit kernel trace 封口；冻结 trace 内 BIE/BIC/BWE/BWC writes/reads 分别为 `3/3`、`1/1`、`3/3`、`1/1`，channel 总计严格 `8/8`。App local damage 映射为 compositor global `(104,152,40,32)`；再叠加 phone Surface origin `(56,64)` 后，checker 在 framebuffer PPM 中冻结 `(160,216,40,32)`，恰有 `1280` 个差异像素，不能把 compositor 坐标与 PPM 坐标混写。

```text
POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED abi=23 surface_session=2 input_session=2 route_epoch=3 physical_floor=9 focus=launcher/4 output_frame=5
POST_RECOVERY_APP_CAPTURED physical=10 target=app events=1 focus=app/5 capture=1
POST_RECOVERY_APP_PRESENTED physical=10/11 command=6 app_frame=4 scene=12 output_frame=6 write_generation=6 focus=app/5 capture=0
POST_RECOVERY_FOCUS_ROUNDTRIP_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=10..11 target=app events=2 capture=0 app_present=1 output_frame=6 focus=app/5 health=2/2 resident=1 errors=0
BOOT_OK: M52 post-recovery Launcher-to-App focus roundtrip and frame verified
```

> `scripts/check-post-recovery-focus-roundtrip.sh` 默认强制 `CARGO_NET_OFFLINE=true`、只连接本地 Unix QMP，并以 `-nic none` 禁用 QEMU 网络。它完整锁定 M49/M50/M51 前缀；M50/M51/M52 三张截图 SHA-256 依次为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`、`cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729`、`97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`。M50→M51 与 M51→M52 的 PPM damage 分别为 `(64,80,40,32)` 与 `(160,216,40,32)`，两次都恰有 `1280` 个差异像素。checker 的精确终态封印为：

```text
POST_RECOVERY_FOCUS_ROUNDTRIP_QMP_OK pointer=136/184/down-captured-up target=app focus=launcher4-app5 output=5-6 diffs=1280/1280 damage=64/80/40/32+160/216/40/32 m50_sha256=798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974 m51_sha256=cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729 m52_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb markers=m49-prefix/m50-prefix/m51-prefix/roundtrip-armed/app-captured/app-presented/roundtrip/boot
```

> M41 共享前缀的 sender-authenticated scheduling stabilization 与默认关闭的 `ui-stale-present-evidence` 均继续回归；后者仍由 directed `scripts/check-ui.sh` 显式启用，并确定性收敛到 36 input/18 个连续 commit，不改变默认路径。M52 历史完整离线 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已从头 exit 0；精确终态 marker 为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M52 只证明单核 QEMU、固定 Launcher/App 双窗口、固定两服务和脚本化 Launcher→App focus roundtrip；lifecycle focus generation 仍为 2，不能把 InputServer/compositor focus generation 5 误写成 lifecycle focus。它不是真实或量产手机系统，也没有真实硬件或硬件网络；蜂窝/电话、Wi-Fi/网络、音频、电源、产品驱动、安全启动、应用沙箱、安全更新、产品级 IME/Unicode/multitouch、任意 App/window 与真机闭环仍未完成。

> **M53 历史完整封口**

> M53 保持 ABI v23、syscall 0—41、八镜像 all-or-none catalog、process capacity 9 与单核 12-context 不变；显式 opt-in 的 `post-recovery-lifecycle-focus-runtime` 是 M52 `post-recovery-focus-roundtrip-runtime` 的 feature child，并完整重放 M49 dependency recovery、M50 App interaction、M51 Launcher focus 与 M52 App focus roundtrip 前缀。M53 不增加 syscall、signal bit、镜像、capability 或 scheduler context；M52 的 marker、截图和冻结 trace 均继续作为不可改写的历史前缀。

> authenticated BSR1 rebind 后，Launcher 与 App 都清空旧 session tracker。Surface session 2 先向两个 client 各发送一个 Ready，再向两端同步 App/Phone focus generation 1；两个 client 各自回送 RFCK。M51 physical 8 的 BIC focus ACK 完成且 BWE down 尚未投递时，SurfaceServer 向两端实时同步 Launcher/None generation 2，随后收齐两个 LFCK；M52 physical 10 的 BIC focus ACK 完成且 BWE down 尚未投递时，再向两端实时同步 App/Phone generation 3，随后收齐两个 AFCK。这是 session 2 上的双 client 实时 lifecycle event/ACK 收敛，不是从 compositor metadata 重建的离线摘要。

> M53 新增 lifecycle channel 的 writes/reads 严格为 `14/14`：Ready 两条加三代双 client Focus 共 BUE `8/8`，RFCK/LFCK/AFCK 各两个共 ACK `6/6`。M52 App 的 sender-authenticated private APCK `0x4d35325f4150434b` 仍单独记为 `m52_boundary=1/1`；它晚于 M52 的 17-bit kernel trace 封口，绝不回填或计入 M52 冻结的 BIE/BIC/BWE/BWC `8/8`，也不计入 M53 lifecycle `14/14`。

> 最终 physical sequence 仍为 `1..11`，InputServer 最终 event 仍为 17、focus=`App/4`，compositor focus=`App/5`；output frame/recovery epoch/write generation 仍为 `6/6/6`。M53 自身不注入新 QMP input，不新增 BIE/BIC/BWE/BWC，也不产生新 output commit/frame；其新增证据仅是 M52 完成边界之后的 lifecycle 双 client 收敛。

```text
POST_RECOVERY_LIFECYCLE_SESSION_READY surface_session=2 clients=launcher/app ready_events=2 focus=app/1 lifecycle_events=2 ack=rfck/2 channels=6/6
POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED physical=8 clients=launcher/app focus=launcher/2 lifecycle_events=2 ack=lfck/2 channels=10/10 compositor=launcher/4 input=launcher/3
POST_RECOVERY_LIFECYCLE_APP_SYNCED physical=10 clients=launcher/app focus=app/3 lifecycle_events=2 ack=afck/2 channels=14/14 compositor=app/5 input=app/4
POST_RECOVERY_LIFECYCLE_FOCUS_OK abi=23 surface_session=2 input_session=2 route_epoch=3 physical=1..11 clients=2 ready_events=2 focus_events=6 acks=rfck2/lfck2/afck2 lifecycle=app/1-launcher/2-app/3 compositor=app/5 input=app/4 channels=14/14 m52_boundary=1/1 output_frame=6 health=2/2 resident=1 errors=0
BOOT_OK: M53 post-recovery lifecycle focus convergence verified
```

> `scripts/check-post-recovery-lifecycle-focus.sh` 默认强制 `CARGO_NET_OFFLINE=true`、只连接本地 Unix QMP，并以 `-nic none` 禁用 QEMU 网络。它锁定完整 M49—M52 历史前缀及上述四个 M53 guest marker；M52 与 M53 截图 SHA-256 均为 `97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`，M52→M53 diff=`0`。此前 M50→M51、M51→M52 的 diff 仍分别为 `1280/1280`，damage 仍分别为 `(64,80,40,32)` 与 `(160,216,40,32)`。checker 的精确终态封印为：

```text
POST_RECOVERY_LIFECYCLE_FOCUS_QMP_OK physical=1..11 m53_input=0 lifecycle=app1-launcher2-app3 clients=2 ready=2 focus_events=6 acks=rfck2/lfck2/afck2 channels=14/14 m52_boundary=1/1 compositor=app5 input=app4 output=6-6 diffs=1280/1280/0 damage=64/80/40/32+160/216/40/32+none m50_sha256=798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974 m51_sha256=cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729 m52_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb m53_sha256=97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb markers=m49-prefix/m50-prefix/m51-prefix/m52-prefix/session-ready/launcher-synced/app-synced/lifecycle-focus/boot
```

> 历史 M53 当时完整离线 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 的 suite marker 已包含 M53 字段：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1
```

> M53 仍只是单核 QEMU 上的固定两服务、固定 Launcher/App 双窗口与脚本化 physical `1..11` 研究原型，不是真实或量产手机系统。它没有扩展产品边界：任意 App/window、产品级 IME/candidate/locale/font shaping/Unicode/multitouch、通用服务依赖图与并发恢复、AndroidBox、通用文件写入/cache 与存储安全、网络/蜂窝/Wi-Fi/音频/电源、真实硬件驱动和硬件网络、secure boot、应用 sandbox、安全 update、开发工具链与真机闭环仍未完成。

> **M54 历史 capability-scoped crash-safe AppData 封口（opt-in）**

> M54 的 `app-data-runtime` 是 M53 `post-recovery-lifecycle-focus-runtime` 的显式 opt-in feature child。只在该 profile 中，`bndr-abi` 为 ABI v24，并新增 syscall 42 `AppDataRootOpen`、43 `FileReplaceAt`、44 `DirectoryCreateAt`、45 `UnlinkAt` 与 46 `DirectoryReadAt`；读取复用既有 `FileOpenAt`/`VmoRead`。feature-off default 与 M53 profile 继续编译为 ABI v23，syscall 42—46 在旧 profile 不构成可用接口。M54 没有新增用户镜像、dynamic process slot 或 scheduler context。

> M54 fixture 在原有 index 0 `BNDROID_SYS` LBA 2048—16350 和 index 1 `BNDROID_DATA` LBA 64—127 之外，增加 private GPT index 2 `BNDROID_APPDATA` LBA 128—2047，固定 1920 sectors/960 KiB；其 type GUID 与 partition GUID/format epoch 独立于旧 DATA。AppData 格式由分区 relative LBA 0 的最终不可变 superblock、relative LBA 1/2 的双 checkpoint，以及 base relative LBA 3/961 的双 bank 组成。每次正常 mutation 全量写 inactive snapshot、flush、发布 inactive checkpoint、再次 flush并完整 readback；旧 checkpoint/bank 在新 checkpoint durable 之前保持不变。

> 整个 AppData 卷的格式与 ABI 均为有界集合：最多 32 个 file/directory entry；canonical UTF-8 root-relative path 最多 64 bytes、最多 4 层；单文件最多 4096 bytes；所有 live file payload 合计最多 128 KiB。当前 runtime 只启用稳定 principal 1 的 primary App，提供显式 create-directory、open/read/list、unlink-empty 与带 create-only/exact CAS 的 atomic file replace；这些能力不等价于 POSIX fd/filesystem。

> 正常 mutation 的切点矩阵覆盖 289-sector snapshot、1-sector checkpoint 和两次 flush，共 292 个 crash point；每个切点恢复结果只能是完整 old 或完整 new，不能拼接两个 bank。首次格式化另有 965 个 sector-atomic/Eager replay 切点：第一笔 durable 写是在分区首 sector 上发布 sealed 512-byte format intent，随后完整 scrub/write bank 0、发布 checkpoint 0，最后才用最终 immutable superblock 替换 intent。该证明明确假设 512-byte sector atomic；对 torn 首 intent、伪 intent、范围外污染或 torn 最终 superblock 只做 fail-closed，不声称可从任意 torn-sector 写恢复。

> block descriptor 一旦提交，无法证明是否完成的 mutation deadline/device failure 被提升为 `SubmittedMutationOutcomeUnknown`，最终 ABI 返回 `OutcomeUnknown`，而不是谎报未提交。此时 storage recovery latch 保持关闭：逻辑 IRQ gate 与真实 GIC block IRQ line 都先 disarm，reset/rearm 路径清理 pending/旧 completion并重建 virtio queue，最后才 clear pending、重新 enable GIC 与开放 latch。恢复前的新 I/O 被拒绝；这仍不是 exactly-once、journal replay 或 anti-rollback。

> AppData root 默认只有 `READ|WRITE|DUPLICATE`，不含 `TRANSFER`。唯一成功 root 属于 authenticated App image/principal 1；init、Launcher、SurfaceServer generation 1/2 四个非 App 身份均被拒。read-only attenuation 后 mutation 被拒，向 `TRANSFER` rights escalation 的 duplicate 被拒；12 个 canonical path/address validation probe、`/system` 写和旧 `BNDROID_DATA` 越权写均不产生副作用。四次 QEMU 都只发布一次下列 authority seal：

```text
APPDATA_AUTHORITY_OK principal=1 root_success=1/6 non_app_denied=4 identities=init/launcher/surface-v1/surface-v2 attenuated_write_denied=1 transfer_escalation_denied=1 validation_rejected=12 system_write_rejected=1 data_write_rejected=1
```

> M54 专用 boot monitor stack 从 default/M53 的 128 KiB 提升为 256 KiB；AppData service 在 IRQ-masked 边界进入前调用 stack-headroom assertion，必须保留至少 160 KiB，防止 no_std debug 栈帧覆盖页表。该变化只属于 `app-data-runtime`，不是默认 kernel stack 的全局扩大。

> `scripts/check-app-data-runtime.sh` 使用独立 target、强制 `CARGO_NET_OFFLINE=true`，四次 QEMU 均显式 `-nic none`，并完整重放 M49—M53 的 11 个 QMP input 与冻结截图前缀。三次连续启动同一 writable image，再对 boot2 副本破坏最新 checkpoint 一个 byte，得到下列精确 mount/runtime 证据：

```text
APPDATA_MOUNT_OK format=1 formatted=1 generation=0 checkpoint_slot=0 bank=0 valid_snapshots=1 rejected_snapshots=1 entries=0 live_bytes=0 reads=2501 writes=961 flushes=4 full_readback=1 fail_closed=1
APPDATA_RUNTIME_OK abi=24 phase=created version=1 boot_generation=0 committed_generation=2 entries=2 files=1 directories=1 submissions=10 completions=10 retrievals=10 mutations=2 reads=1 lists=3 conflicts=1 expected_terminal=3 disk_reads=6682 disk_writes=580 disk_flushes=4 old_or_new=1 full_readback=1 resident=1 errors=0
APPDATA_MOUNT_OK format=1 formatted=0 generation=2 checkpoint_slot=0 bank=0 valid_snapshots=2 rejected_snapshots=0 entries=2 live_bytes=24 reads=581 writes=0 flushes=0 full_readback=1 fail_closed=1
APPDATA_RUNTIME_OK abi=24 phase=upgraded version=2 boot_generation=2 committed_generation=3 entries=2 files=1 directories=1 submissions=12 completions=12 retrievals=12 mutations=1 reads=2 lists=4 conflicts=1 expected_terminal=4 disk_reads=7554 disk_writes=290 disk_flushes=2 old_or_new=1 full_readback=1 resident=1 errors=0
APPDATA_MOUNT_OK format=1 formatted=0 generation=3 checkpoint_slot=1 bank=1 valid_snapshots=2 rejected_snapshots=0 entries=2 live_bytes=24 reads=581 writes=0 flushes=0 full_readback=1 fail_closed=1
APPDATA_RUNTIME_OK abi=24 phase=stable version=2 boot_generation=3 committed_generation=3 entries=2 files=1 directories=1 submissions=11 completions=11 retrievals=11 mutations=0 reads=2 lists=4 conflicts=1 expected_terminal=4 disk_reads=6393 disk_writes=0 disk_flushes=0 old_or_new=1 full_readback=1 resident=1 errors=0
APPDATA_MOUNT_OK format=1 formatted=0 generation=2 checkpoint_slot=0 bank=0 valid_snapshots=1 rejected_snapshots=1 entries=2 live_bytes=24 reads=292 writes=0 flushes=0 full_readback=1 fail_closed=1
APPDATA_RUNTIME_OK abi=24 phase=upgraded version=2 boot_generation=2 committed_generation=3 entries=2 files=1 directories=1 submissions=12 completions=12 retrievals=12 mutations=1 reads=2 lists=4 conflicts=1 expected_terminal=4 disk_reads=5820 disk_writes=290 disk_flushes=2 old_or_new=1 full_readback=1 resident=1 errors=0
BOOT_OK: M54 capability-scoped crash-safe AppData runtime verified
```

> boot1 为 `created v1/gen 0→2`，boot2 为 `upgraded v2/gen 2→3`，boot3 为 `stable v2/gen 3→3` 且 AppData 分区逐 byte 不变；损坏最新 checkpoint 的恢复启动以 `valid=1/rejected=1` 选择旧完整 gen 2，再升级为 gen 3，绝不混合 snapshot。这个 corruption test 不是断电模拟，checker 也固定 `powercut_claim=0`。最终 checker seal 为：

```text
APPDATA_RUNTIME_QMP_OK abi=24 boots=3 phases=created-v1/upgraded-v2/stable-v2 generations=0-2-3 persistent_image=1 authority=unique prefix=m49-m53 qmp_inputs=11/11/11 screenshots=frozen disk_scope=data64-127/appdata128-2047 recovery=corrupt-newest-checkpoint/fallback-gen2/reupgrade-gen3 old_or_new=1 mixed_snapshot=0 powercut_claim=0 boot='M54 capability-scoped crash-safe AppData runtime verified'
```

> canonical M54 image SHA-256 为 `577a9c422b0edfb05f18f02120c0632d9f64c632664465106fb9876b08ffc1cf`；boot1/boot2/boot3-final SHA-256 分别为 `71fa78984aecc47f1bbf696d941e578047bbc801389343d660a6809da99e5dee`、`6ff9f3a4d48dc180d374a0ae78fdc1c38fc1f3086c82935b2fb985d8c98fd3f6`、`ca28d583bea47b1b1edd31d23228475d4af94cc43cf3fa9bff084a29ab645e2e`，恢复镜像最终也为 `ca28d583...645e2e`。相对 canonical 的 changed LBA 为 boot1 `71`（DATA 1 + AppData 70）、boot2 `73`（DATA 2 + AppData 71）、boot3 `73`、recovery `73`；MBR、主/备 GPT、`/system`/FAT 与其它 LBA 均逐 sector 不变。每次的 M50/M51/M52/M54 截图 SHA-256 都依次为 `798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974`、`cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729`、`97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`、`97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb`，diff=`1280/1280/0`。保留证据位于 `target/bndroid-appdata-m54.P1CoH7/`，发布用截图副本位于 `target/m54/`。

> `bndr-appdata` 的 22 项历史 host tests 覆盖正常操作、292 个 mutation 切点和 965 个首次格式化切点；历史 M54 专用四启动 checker 与当时完整离线 `./scripts/test.sh` 均 exit 0。该段只描述 M54 kernel-monitor service；M55—M69 历史 StorageServer/shutdown/unified-product/liveness 与历史 M70 QEMU PSCI 证据见文首，不能回写成 M54 架构。它们都不是 POSIX 或产品文件系统，也都缺真实 UFS/eMMC/NVMe/手机控制器与真机断电恢复；`general_runtime=0`。

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

> M31 在 M30 的独立 SurfaceServer/Launcher 边界上增加独立 EL0 App 与第二对 UI Channel。SurfaceServer 仍是唯一 Surface capability owner；Launcher 和 App 都无法 acquire Surface。64-byte `BUC1` v1 控制协议以 client-only `AttachAppEndpoint` 转移 App 的 server endpoint，只有 Launcher 可用严格递增 transition id 发起 `SetFocus`。SurfaceServer 以三项 wait-array 轮换等待 Surface、Launcher 与 App，两个 client 各用单对象 wait；两对通道都固定 kernel-stamped generation-qualified sender PID。

> Present protocol 为 v2/64-byte。Launcher/App frame 携带 focus generation；server 只提交当前焦点且 generation 匹配的 client frame，并把两条本地连续 frame 序列映射成一条全局连续 commit 序列。后台或 stale-generation present 返回 `PresentCancelled`，不推进序列，允许同一 frame id 重试。默认关闭的 `ui-stale-present-evidence` 提供确定性 directed QEMU 证据：24 input/13 commit baseline 后，frame 10 在 SurfaceServer 已读取 generation 8 时挂起，Home generation 9 取消它，generation 10 以同一 local frame id 重试，最终 Home generation 11 收敛为 36 input/18 个连续 commit；无需尝试多个调度相位。输入在 pointer down 时选择 Home→Launcher 或当前 App 并 capture 到 release；QEMU surface trace 也证明 Settings 内按下、拖到 Home 区再抬起的 App 本地 sequence `1..3` 全部只到 App，Launcher 无泄漏且不触发 focus/frame commit。Launcher 独立绘制 Home，App 独立绘制 Phone/Messages/Settings。

> M32 增加两槽、page-aligned 的静态 XRGB8888 GraphicsBuffer pool：每槽固定 `208×368`、logical/backing `306176/307200` bytes。App 以 `READ|WRITE|DUPLICATE|TRANSFER=0x0f` 创建，用 syscall 29 进行非空、4-byte 对齐且单次最多 4096-byte 的规范化写入，再把 `READ|TRANSFER=0x09` 衰减副本随首个 64-byte `BUP1` `BufferPresent` 原子转移给 SurfaceServer。M32 历史 wire 为 v1；当前 v2 保留 Full geometry 和 client/global frame id、focus generation、buffer generation，并新增 byte 40..48 的 System UI revision；mobile frame 必须绑定当前非零 revision。最后一个引用释放时完整擦除 backing 并推进 slot generation。

> M33 专用 profile 在 ABI v18 上固定三种 canonical 64-byte v1 wire：`ALC1` app lifecycle、`UBP1` startup endpoint bootstrap 和 `USC1` Init↔SurfaceServer supervisor control。Init 驱动 `Launch1→Activate1→Suspend1→Resume1→Terminate1→Launch2→Activate2` 七个严格事务；内核 committed-write trace 精确接受 ALC1 35 条消息和 USC1 14 条消息，USC1 Install/Activate/ShowLauncher/Retire 命令计数为 `2/3/1/1`，两类 trace 的 decode、authentication 与 tracker error 均为 0。

> M33 QEMU 收敛证明 App1 在 terminate ack、Surface 退休旧 endpoint/buffer/capture 后以正常 Exited 路径退出并被 `ProcessWait` 回收；App2 复用同一 PID slot、PID generation 严格加一，最终保持 active/resident。created/exited/reaped/live=`10/2/2/8`，最终 29 handles、26 endpoint/13 pair；其中 M20 core 8 pair 外有 Init↔Surface supervisor、Init↔Launcher lifecycle、Init↔App lifecycle、Surface↔Launcher UI、Surface↔App UI 五对常驻窗口通道。最终等待账本为 8 项 object token，其中 wait-many 2、wait-array 6。

> M34 专用 profile 把脚本扩展为十个严格事务：在 M33 七步之后，Init 以 ABI 18 `ProcessTerminate` 杀死正阻塞于 `WaitArray` 的 App2；SurfaceServer 从 UI peer-close 原子撤销旧 App endpoint/buffer、焦点、input capture 与 pointer state，切回 Launcher，并向 Init 发送 generation-qualified USC1 `OwnerDied`。Init 接受该自发事务后发布 ALC1 `Crashed/ProcessExited`，再启动并激活 App3。App PID 在同一 slot 上严格 `generation 1→2→3`，最终 App3 active/resident，GraphicsBuffer generation 为 3。专用 QEMU 精确得到 ALC message/transaction/state/crash=`46/10/19/1`，USC command/ack/owner-death=`9/9/1`，USC transaction=`10`，operation Install/Activate/ShowLauncher/Retire=`3/4/1/1`；process created/exited/reaped/live=`11/3/3/8`，exit reasons Exited/Faulted/Killed=`2/0/1`，terminate accepted/completed=`1/1`。最终仍为 29 handle、26 endpoint/13 pair 和 8 个 object wait token（many/array=`2/6`），终止遗弃 object/many/array token=`0/0/1`。

> buffer decode、producer/rights、focus/sequence、generation、全部像素和 counter 在首像素前验证，失败/取消不改变 scene、序列与计数。启动期以一次 4-byte seed 和 pre-focus cancelled present 验证句柄握手，所以启动账本 `created/write_calls/write_bytes/presents=1/1/4/0`；交互路径由 App raster 76544 个真实像素并由 SurfaceServer copy-present。early per-page W^X window 为此从 RAM 前 4 MiB 扩为 6 MiB，新增 L3 覆盖但不放松权限。这是有界 object transfer+copy，不是 `mmap`、共享 EL0 mapping 或零拷贝。

> M22 的无分配、两遍 FDT API 以固定容量 32 收集 enabled、direct-root `virtio,mmio` 节点，并解析根或节点级 `interrupt-parent`、GICv2 controller phandle、`#interrupt-cells = 3` 与 SPI specifier。当前 QEMU 树恰有 32 个 coherent transport，其中 1 个 active block device；其 raw specifier `[0,47,1]` 被严格解释为 edge-rising SPI INTID 79。GICv2 在开放本地 IRQ 前完成 disable/configure/target CPU0/priority/clear/enable。M25 正常设备必须物理可写，严格要求 `VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_FLUSH` 并拒绝 RO；独立的 M22 race build 仍使用物理只读设备。两者都拒绝 legacy MMIO 与 non-coherent DMA。

> queue 0 固定为 8 项 split ring，216-byte ring 独占一个物理 frame；第二个 frame 以 536-byte stride 打包两个 header/data/status request slot，总占用 1072 bytes，descriptor head 为 0/3。请求追踪器给每个槽分配 generation-qualified token，支持两个同时 outstanding、乱序 used completion、重复/未知/过期 completion 拒绝。M25 在同一所有权模型内加入 type 1 WRITE 与 type 4 FLUSH，二者都严格验证 `used.len=1`，且只有 WRITE 的 IRQ completion 完成后才发布 FLUSH。正常前台只发布请求并等待 IRQ 侧递增的完成代数，不轮询 used ring；IRQ 顺序为读取并 ACK virtio source、drain used entries、更新完成状态，再由公共 dispatcher EOI GIC。单一静态 storage owner 永久持有 driver 和两帧，并通过屏蔽本地 IRQ 的短临界区保证单核所有权。

> M23 在同一 IRQ-only owner 上增加 512-byte sector block abstraction、严格 GPT 与只读 FAT16/VFS；M23/M24 历史 fixture 的 SHA-256 为 `36f39e23da09401dc2f686217e2bb06d146a65cd14aa4b9b8b2f0fd4aa7b809b`。M24 只在系统文件大小和摘要全部验证后，才把固定 28+40=68 bytes 一次性复制到 immutable `BootfsCatalog<2>`/VMO；catalog 发布后的用户态 open/read 不再访问磁盘。M25 确定性镜像为 8 MiB/16384 sectors，SHA-256 改为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`：protective MBR、主 header LBA 1/entry array 2—33、备份 array 16351—16382/header 16383 均被严格验证；index 0 `BNDROID_SYS` 仍为 LBA 2048—16350 且只读，新 private index 1 `BNDROID_DATA` 为 LBA 64—127。DATA superblock 绑定其 GPT unique GUID 派生的 16-byte format epoch；两个 512-byte CRC record slot 若都有效，generation 必须相邻。提交顺序固定为 inactive-slot WRITE completion、FLUSH completion、重读双槽，旧已选槽始终保留；fresh boot 从 generation 0 提交到 1。下列 M25—M30 marker 只作为对应阶段历史证据；其中 ABI v14/v15 与旧五/七进程账本不能冒充 M31。

> M26 严格唯一发现 direct-root `qemu,fw-cfg-mmio`，验证 `QEMU` signature/features，并通过 big-endian DMA 配置 `etc/ramfb`。kernel-owned framebuffer 为 320×480 XRGB8888、stride 1280、614400 bytes/150 pages；九色 static splash 的 kernel digest 为 `0x6ef9c2b7d15fde25`。headless `check-framebuffer.sh` 用 QMP screendump 精确验证 153600 pixels、9 colors/9 samples；pixel SHA-256 为 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。M26 还绑定 coherent modern virtio-input keyboard：device ID 18、MMIO `0x0a003c00`、raw IRQ `0/46/1`/SPI 78、queue 8/one DMA frame，config name/key bitmap 必须含 A+Enter；block 保持 `0x0a003e00`/SPI 79。QMP A down/up 精确形成 A-down/SYN/A-up/SYN，completions/delivered/recycled=`4/4/4`、used/avail=`4/12`，drop/invalid/config IRQ/spurious 均为 0。这些是保留的 M26 历史证据。

> M27 在 M26 scanout 上加入两个 kernel-owned 软件层：完整 320×480 opaque scene 与 12×22 alpha cursor；移动时从 immutable scene 恢复旧 rect，只在 clipped/union dirty rect 内合成新 cursor，按下/释放有不同视觉与 digest。隐藏 cursor 的启动帧保持 FNV `0x6ef9c2b7d15fde25` 和 RGB SHA-256 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。输入驱动同时永久拥有 keyboard `0x0a003c00`/SPI 78 和 tablet `0x0a003a00`/raw `0/45/1`/SPI 77；每个 size-8 eventq 独占一个 DMA frame。tablet ABS X/Y 都为 `0..32767`，要求 `BTN_LEFT`/`BTN_TOUCH`，只在 SYN boundary 提交 report。`check-compositor.sh` 精确注入 raw X/Y `1234/23456`、SYN、touch down/SYN、touch up/SYN，共 7 events/3 samples，映射 pixel `12/342` 并触发 3 次 dirty redraw；前后全帧恰有 122 个变化 pixel，bbox `12/342/21/363` 完全位于 dirty rect `12/342/12/22`，最终 RGB SHA-256 为 `f24699248bcdfe5069fa13a40ef7990ba7a647cfe0ac818c04f671047e748f82`。这些是保留的 M27 历史证据。

> 交互运行 `./scripts/run-qemu.sh` 会同时挂载 ramfb、virtio keyboard 与 virtio tablet；`check-compositor.sh` 保留 M27 cursor 验收，当前 `./scripts/check-ui.sh` 显式启用默认关闭的 `ui-stale-present-evidence`，并验证 M32 `BUP1`/真实 App raster：baseline 为 24 input/13 commit（4 legacy Launcher + 9 buffer App）；随后 frame 10 在 SurfaceServer 已读取 focus generation 8 时挂起、在 Home generation 9 取消、于 generation 10 同帧重试，最终 Home generation 11 收敛为 36 input/18 commit。Settings buffer generation 为 `76/151/226`，最终 RGB SHA-256 为 `62b15db5e54d3bbca7bd23e74a60e7fc66d2566e2094be7718e7c8f1a0ebc5dd`；下列 M28 结果仅是历史基线。

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

> 独立 feature-gated race self-test 会故意抑制一批两个请求的通知直到超时；随后屏蔽 INTID 79、reset/re-negotiate/rebuild queue，使两个旧 token 失效，把模拟 late IRQ 只计为一次已 ACK 的 spurious notification，再在相同两个 DMA frame 上用两个新请求恢复读取。它要求没有 double completion，精确终态为：

```text
STORAGE_IRQ_RACE_OK timeout_requests=2 timeouts=1 resets=1 reset_tokens_invalidated=2 simulated_late_irq=1 spurious_acked=1 recovered_requests=2 recovered_completions=2 double_completions=0 dma_frames_before=2 dma_frames_after=2 digest0=0xbebd264b8c14cd72 digest1=0x8294de399174037c
BOOT_OK: M22 storage IRQ timeout/reset self-test recovered and M20 multi-session services verified
```

> M20 下层服务基线仍让 primary 与 secondary 两个 Client 常驻并分别拥有独立、generation-qualified manager session；Lookup 回复沿各自 session 路由，不使用 M18 的每请求 private reply Channel。固定 23 阶段 transcript 覆盖 secondary lease 1 的 txid `0x601` 停滞、primary 的 `0x602/0x603` 持续前进、lease 1 revoke/provider abort、secondary lease 2 reattach、旧 lease 双端拒绝及 `0x604` 完成。最终 phase/errors=`23/0`、attach=`2/2`、revoke/stale/primary-progress bitmap 均为 `0x3`、provider accept/echo/abort=`4/3/1`、secondary echo=`1`、idle bitmap=`0x3`，且没有 Client crash/restart。

> M32 默认路径认证基线（历史里程碑语义，当前由 ABI-v23 head 回归保留）最终常驻 `init`、manager2、provider、primary、secondary、SurfaceServer、Launcher、App 共 8 个进程，created/exited/reaped/live=`9/1/1/8`、live images=`1/1/1/2/1/1/1`、handles by image=`4/5/3/4/4/1/2`（共 23）。M20 core Channel 图保持 16 endpoint/8 pair，另有两对 UI Channel，合计 20 endpoint/10 pair；Surface capability 唯一归 SurfaceServer，GraphicsBuffer 同一 generation-qualified object identity 只由 App producer 与 SurfaceServer consumer 以 `0x0f/0x09` rights 持有。等待项为 `4/2/2/2/3/1/1`，object/many/array pending=`7/2/3`。这些数值不是 M33/M34/M35 专用 profile 的账本。

> M20 服务 transcript 与 M29—M31 旧账本只保留为历史基线。M32 default normal checker 继续精确固定 syscall `599/462/136`、copy calls/bytes `198/198` 与 `7835/17487`、Channel pair 48、duplicate/stale-close `190/74`、transfer `65/64`、ObjectWait/WaitArray calls `455/57`、23 handle，并到达 `BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified`。经完整矩阵验证的 ABI-v20/M42 封口 default workspace 为 500 tests（`20/41/19/31/99/0/290`），加 M41 feature 20 与 M42 feature 3 项为 523 unique/313 unique kernel；hardened persistent-window QEMU 3/3、M41 regression、all-feature Clippy 与完整 `./scripts/test.sh` 均已从头通过，最终 marker 与上方 exact suite marker 相同。

> 产品范围仍是 `general_runtime=0 crash_consistency=0`。M34—M48 分别只证明固定 lifecycle/death/restart/orphan/frame-clock/swapchain/multi-window/persistent-window、输入与单服务监督场景；M49 已完成固定 SurfaceServer+InputServer、单条 soft dependency、degraded→recovered 与 restart-storm 防护 witness，后续 M50—M53 继续复用该固定两服务前缀。它仍不是任意服务依赖图或产品 supervisor。系统也没有硬件 vblank/pageflip、DMA-BUF/IOMMU、任意 App/window 管理、产品级 InputServer、完整 IME/candidate/locale/font shaping/Unicode、多点或真实硬件触屏；网络、电话/蜂窝、Wi-Fi、音频、电源管理、产品级驱动、安全启动、应用沙箱、安全更新与可用产品 UX 也均未完成，当前绝不是现实可用的手机系统。

> M32 ABI-v17、M33/M34 ABI-v18 与 M35—M41 作为历史与专项基线继续保留；ABI-v20/M42、ABI-v21/M43/M44、ABI-v22/M45/M46 及 ABI-v23/M47 的完整历史封口保持不变。ABI-v23/M48 的 dedicated QEMU、四截图、host、隔离回归、静态/Clippy matrix 与最终全量 `scripts/test.sh` 均已通过。这仍不是多服务 supervisor、完整 compositor、产品级 IME/InputServer 或手机产品。

## 第 1 章：项目总纲

Bndroid OS 是一个 Rust-first 的自研移动操作系统。它采用非 Linux 主内核路线，系统整体设计强调安全、统一、可控、可维护和可扩展。项目目标是打造一个拥有自研内核、自研系统服务、自研应用框架、自研权限模型，同时具备 Android 应用兼容能力的现代移动操作系统。

更好的理解方式是：Bndroid OS 不是一个普通 Android ROM，也不是一个 UI 皮肤项目，而是一个从内核、系统服务、图形栈、应用模型、安全模型到兼容子系统都独立设计的操作系统项目。目标架构让 Android 应用通过 AndroidBox、原生应用通过 Bndroid Native SDK 运行，两者共享底层系统服务但由权限和沙箱隔离；当前实现尚未达到这套应用模型。

### 1.1 项目愿景

Bndroid OS 的长期愿景：

- 构建 Rust 优先的移动操作系统基础设施。
- 建立安全、流畅、统一的原生应用体验。
- 通过 AndroidBox 获得 Android 应用生态兼容能力。
- 在模拟器、开发板和真实 ARM64 移动设备上运行。
- 提供 SDK、模拟器、调试工具、应用打包工具和兼容性测试工具。
- 建立可持续维护的系统架构，而不是依赖一次性 hack。

### 1.2 系统能力目标

MVP 阶段能力：

- QEMU ARM64 启动。
- Rust kernel 输出日志。
- 用户态 init 启动。
- ServiceManager 工作（M20 已在 M17 认证/ACL 与 M18 并发基线上加入两个常驻 Client 的独立 session、session-routed reply、一次 stalled-secondary revoke/reattach/stale-lease 隔离及精确五进程空闲图；MVP 仍需任意接入与 crash/peer-close 隔离的产品级 resilient runtime、权限策略及审计）。
- framebuffer 显示。
- 触控或鼠标输入。
- 简单 Launcher。
- `.bapp` 原生应用安装和启动。
- 权限声明解析。
- Android APK 解析。
- ART/Bionic 最小运行。
- Binder 兼容原型。
- 简单 Android Activity 显示。

Beta 阶段能力：

- WindowServer 和 Compositor 成熟。
- 原生 UI SDK 可用。
- PackageManager 支持更新和回滚。
- PermissionManager 支持运行时授权。
- AndroidBox 支持常见工具类 APK。
- 支持网络、音频、通知、WebView。
- 支持开发者调试工具。
- 支持兼容性测试数据库。

正式阶段能力：

- 真机启动。
- 安全启动。
- OTA。
- 应用签名体系。
- 用户数据加密。
- Android 应用兼容性大幅提升。
- 原生应用生态初步可用。
- 系统服务崩溃可恢复。
- 图形性能达到日常使用水平。

## 第 2 章：Rust-first 技术战略

Rust 是 Bndroid OS 的主语言。系统中最重要、最敏感、最长期维护的部分都优先使用 Rust。

### 2.1 Rust 覆盖范围

Rust 用于：

- kernel core
- scheduler
- memory manager
- IPC
- handle table
- capability system
- driver framework
- init
- service manager
- log server
- package manager
- permission manager
- app manager
- window server
- input server
- update server
- native SDK
- CLI tools
- emulator tools
- compatibility test tools

### 2.2 Rust 的系统工程优势

Rust 在 Bndroid OS 中的价值：

- 用 ownership 降低 use-after-free 风险。
- 用 borrow checker 减少并发数据竞争。
- 用 enum 和 pattern matching 建模系统状态。
- 用 trait 建模服务接口。
- 用 Result 显式处理错误。
- 用 no_std 支持内核开发。
- 用 workspace 管理大型工程。
- 用 cargo test 支持大量纯逻辑测试。
- 用 FFI 和 C/C++ 组件集成。

### 2.3 C/C++ 的角色

AOSP、ART、Bionic、Skia、Vulkan、OpenGL ES 等组件仍然会使用 C/C++。更好的工程策略是：C/C++ 不主导系统架构，而作为兼容层或底层库存在。Rust 负责资源管理、权限检查、进程边界和服务接口，C/C++ 组件通过 wrapper 接入。

## 第 3 章：总体架构

```text
Bndroid OS
├── Applications
│   ├── Native Rust Apps
│   ├── Android Apps
│   └── Web Apps
├── Frameworks
│   ├── Native UI Framework
│   ├── Android Framework Shim
│   ├── Media Framework
│   └── Web Runtime
├── System Services
│   ├── Init
│   ├── ServiceManager
│   ├── LogServer
│   ├── AppManager
│   ├── PackageManager
│   ├── PermissionManager
│   ├── WindowServer
│   ├── Compositor
│   ├── InputServer
│   ├── AudioServer
│   ├── NetworkServer
│   ├── CameraServer
│   ├── LocationServer
│   ├── NotificationServer
│   ├── PowerServer
│   ├── StorageServer
│   ├── SecurityServer
│   └── UpdateServer
├── AndroidBox
│   ├── APK Installer
│   ├── ART
│   ├── Bionic
│   ├── Dynamic Linker
│   ├── Binder Compatibility
│   ├── Android System Service Shim
│   ├── Surface Bridge
│   ├── Input Bridge
│   ├── Audio Bridge
│   ├── Network Bridge
│   └── Storage Bridge
├── Security
│   ├── Code Signing
│   ├── Sandbox
│   ├── Capability Tokens
│   ├── Keychain
│   ├── Audit Log
│   └── Verified Boot
├── Kernel
│   ├── AArch64 Boot
│   ├── Virtual Memory
│   ├── Scheduler
│   ├── Process Manager
│   ├── Thread Manager
│   ├── IPC
│   ├── Handle Table
│   ├── Interrupt
│   ├── Timer
│   ├── Driver Interface
│   └── Linux ABI Subset for AndroidBox
└── Hardware
    ├── ARM64 SoC
    ├── Display
    ├── Touch
    ├── GPU
    ├── Audio
    ├── Camera
    ├── Wi-Fi
    ├── Bluetooth
    ├── Modem
    ├── Sensors
    └── Battery
```

## 第 4 章：内核完整 TODO

### 4.1 工程结构

推荐目录：

```text
kernel/
├── Cargo.toml
├── src
│   ├── main.rs
│   ├── arch
│   │   └── aarch64
│   ├── boot
│   ├── memory
│   ├── task
│   ├── scheduler
│   ├── ipc
│   ├── handle
│   ├── syscall
│   ├── interrupt
│   ├── timer
│   ├── driver
│   ├── security
│   └── log
└── linker.ld
```

### 4.2 Boot TODO

- [ ] 编写 AArch64 boot.S。
- [ ] 设置初始栈。
- [ ] 清理 BSS。
- [ ] 传递 device tree pointer。
- [ ] 进入 Rust kernel_main。
- [ ] 初始化 UART。
- [ ] 输出 boot banner。
- [ ] 初始化异常向量。
- [ ] 初始化页表。
- [ ] 打开 MMU。
- [ ] 初始化 kernel heap。
- [ ] 初始化 timer。
- [ ] 初始化 interrupt controller。

### 4.3 内存管理 TODO

- [ ] 解析物理内存区域。
- [ ] 标记 kernel image 占用区域。
- [ ] 实现 frame allocator。
- [ ] 实现 page table manager。
- [ ] 实现 kernel heap。
- [x] 实现通用有界 user address-space 生命周期（每进程私有 TTBR0；init ASID 1；child 动态 ASID；VMA/frame 集合逐页建图与权限校验、全 leaf/物理帧唯一性和进程间整组隔离验证；失败回滚，TLBI 后页表、leaf 与元数据完整回收复用）。
- [x] 实现有界 immutable VMO（M24 boot catalog 固定 2 项/68 bytes、自定义原子引用计数、`READ|DUPLICATE|TRANSFER`；尚无 MAP、shared memory 或通用 VMO pager）。
- [ ] 实现 mmap。
- [ ] 实现 shared memory。
- [ ] 实现 copy-on-write。
- [ ] 实现 page fault。
- [x] 实现用户栈低/高 guard page，并验证两页均未映射。
- [ ] 实现 memory accounting。
- [ ] 实现 memory pressure event。

### 4.4 调度器 TODO

- [x] 有界 Thread object 原型（固定 init + 一个可复用 child，独立 16 KiB exception stack，Zombie/Faulted 延迟回收）。
- [x] 有界 Process object 原型（generation PID、独立 address space/HandleTable、init + 1 child；尚非通用容量）。
- [ ] Job object。
- [x] Context switch（当前为单核固定 context）。
- [x] Kernel stack（固定 16 KiB + canary）。
- [x] User stack（每进程 4 页 RW/NX + 低/高 guard；init/child 运行时触达全部 4 页）。
- [x] Round-robin（固定 context）。
- [ ] Priority scheduling。
- [ ] Realtime class。
- [ ] UI class。
- [ ] Background class。
- [x] Sleep queue（当前容量 2）。
- [x] Timer wakeup。
- [ ] CPU usage accounting。
- [ ] AndroidBox process group。

### 4.5 IPC TODO

- [x] Channel create（当前容量 8、inline scalar/byte/transfer message）。
- [x] Channel read。
- [x] Channel write。
- [x] 单对象 Channel/Event wait（ABI v10 保留 syscall 15；Channel 为 level-triggered READABLE/WRITABLE/PEER_CLOSED，Event 为 SIGNALED，并校验对象专属 signal mask）。
- [x] 可转移 manual-reset Event（ABI v10 保留 syscall 16–18；SIGNAL/WAIT rights、signal/clear 边沿、阻塞唤醒、transfer rollback 与退出回收已验证）。
- [x] 固定两项 wait-any 与相对 timeout（ABI v10 保留 syscall 19；poll/infinite/finite、全参数验证、最低 ready index、单次 counter 采样的 deadline-first exact-token 仲裁）。
- [x] 有界 1—8 项 wait-any 数组（default/M53 ABI-v23 与 M54 ABI-v24 都保留 syscall 22 `ObjectWaitManyArray`；8-byte canonical LE item、最多 64-byte usercopy、copy+全项 validation-before-ready、最低 index、真实八项 block/wake、显式 completion kind 与 exact token）。
- [x] M24 只读存储 capability ABI（syscall 23 `FileOpenAt`、syscall 24 `VmoRead`；canonical root-relative UTF-8，`x2` 高/低 32 bit 为 offset/requested length，每次最多 4096 bytes、EOF 截短，并验证 bad-address/bounds/rights/stale/transfer）。
- [x] Channel 头部探测（ABI v10 syscall 20 `ChannelPeek`；READ/reserved-zero、非消费 kind/length、queued-before-peer-close，且与 read 非原子）。
- [ ] Channel call。
- [ ] Message encoding。
- [x] Handle passing（ABI v10 保留每条 transfer 最多 64 bytes + 1 个 move-only Channel/Event handle；image-selecting 跨进程 bootstrap 与拒绝不消费 source 已验证）。
- [x] Capability check（READ/WRITE/DUPLICATE/TRANSFER/WAIT/SIGNAL；duplicate 只能衰减，transfer 保持原 rights）。
- [x] Channel ownership DAG（单调非零 ID，仅允许较旧 transport 持有较新 Channel；保守拒绝反向 transfer 以消除跨 Channel 引用环）。
- [ ] Shared memory transfer。
- [ ] Async notification。
- [x] Dead peer signal（`PEER_CLOSED` level signal + blocking ObjectWait）。
- [ ] 超过 8 项/任意长度 wait-any、wait-all、显式 cancel、高精度/SMP timer 与通用 IPC timeout。
- [ ] Audit hook。
- [ ] Rate limit。

### 4.6 Handle 和 Capability

Handle 是进程访问资源的唯一方式。Capability 决定 handle 能做什么。

能力类型：

- READ
- WRITE
- EXECUTE
- MAP
- DUPLICATE
- TRANSFER
- WAIT
- SIGNAL
- ADMIN

TODO：

- [x] Handle table（当前 32 slot、24-bit generation）。
- [x] Handle duplicate。
- [x] Handle transfer（`OwnedHandle` source transaction/destination reservation，失败保持 source raw 或同一 FIFO 队首）。
- [x] Handle close。
- [x] Capability mask。
- [x] Rights reduction。
- [x] Object reference counting（当前 Channel、Event、immutable VMO 与 system-directory object）。
- [ ] Revoke。
- [ ] Debug inspect。

## 第 5 章：驱动框架

Bndroid OS 更适合用户态驱动模型。驱动运行在 DriverHost 中，通过受控 MMIO、IRQ 和 DMA 接口访问硬件。

### 5.1 DriverManager TODO

- [ ] Driver manifest。
- [ ] Driver signing。
- [ ] Device tree parsing。
- [ ] Device enumeration。
- [ ] Driver matching。
- [ ] DriverHost spawn。
- [ ] MMIO mapping。
- [ ] IRQ routing。
- [ ] DMA buffer。
- [ ] IOMMU domain。
- [ ] Hotplug。
- [ ] Power state。
- [ ] Crash restart。

### 5.2 块设备、只读文件与持久化数据路径（M25 下层）

- [x] 用固定容量 32 的两遍 FDT 结果收集 enabled direct-root `virtio,mmio` 节点，解析 GICv2 phandle、interrupt-parent 与三 cell SPI；当前 `[0,47,1]` 严格映射到 edge-rising INTID 79。
- [x] modern virtio-mmio v2 transport：reset 后按 `ACKNOWLEDGE -> DRIVER -> FEATURES_OK -> DRIVER_OK` 建立状态；M25 normal 严格要求可写设备、`VERSION_1 | FLUSH` 并拒绝 RO，M22 race build 保留物理 RO；两者都拒绝 legacy、non-coherent 与缺失 active block device。
- [x] queue 0/size 8 split ring：216-byte queue frame + 一个含两个 536-byte request slot 的 frame；head 0/3 同时 outstanding，generation token 支持乱序完成并拒绝 stale/duplicate completion。
- [x] GICv2 在本地 IRQ 开放前路由 INTID 79 到 CPU0；virtio ACK/drain/完成发布发生在 GIC EOI 前，正常前台不轮询 used ring。
- [x] physical-counter deadline、stable 16384-sector capacity、永久静态 owner，以及 timeout 后屏蔽/单次 race drain/reset/re-negotiate/旧 token 作废/同两帧恢复。
- [x] 同时读取 sector 0/1、确定性 digest、越界预提交拒绝，并保留 IRQ timeout/reset/late-event 路径。
- [x] M23 建立 512-byte sector block abstraction；严格验证 protective MBR、主备 GPT header/entry CRC 与完全相同的 128-entry array，并唯一选择 `BNDROID_SYS`。
- [x] M23 挂载双 FAT 镜像一致的只读 FAT16，支持有界 root/subdirectory 8.3 path lookup 和跨 cluster read；通过 `/system` VFS 读取两个固定文件并拒绝 traversal、mount escape 与 write。
- [x] M23/M24 历史 8 MiB fixture 与六类存储负测证据保留；M25 fixture SHA-256 为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`，11 类 storage negative 额外覆盖 RO、missing FLUSH、坏 DATA superblock、坏 slots 与 generation gap。
- [x] M24 将两个验证后的文件缓存为 immutable `BootfsCatalog<2>`/VMO，并用 move-only root capability、`FileOpenAt`/`VmoRead` 向 EL0 暴露；catalog 发布后 runtime disk read 为 0。
- [x] M25 验证 private `BNDROID_DATA` index 1/LBA 64—127、GUID-bound 16-byte format epoch、相邻双槽 CRC generation、type 1 WRITE 与 type 4 FLUSH；inactive-slot WRITE completion 后才 FLUSH，随后重读双槽并保留旧槽，fresh generation `0→1`。
- [x] M25 normal ledger 为 parser/read/write/flush=`273/280/1/1`、request/completion/IRQ completion=`282/282/282`、read/write bytes=`143360/512`；system 与 DATA 外写入在账本不变时被拒绝。
- [x] 跨 QEMU 同一 raw image 为 `0→1→2`、DATA 外不变、`changed_data_bytes=98`；破坏最新槽后回退 generation 0 并重新提交 1。
- [x] M54 opt-in `app-data-runtime`：ABI v24 syscall 42—46、private `BNDROID_APPDATA` index 2/LBA 128—2047、immutable superblock + 双 checkpoint + 双 bank；同一镜像三启动与坏最新 checkpoint 回退、292 个正常 mutation 切点及 965 个首次格式化 sector-atomic 切点均已封口。它仍是 kernel monitor 的有界 AppData slice，不是独立 StorageServer 或 POSIX 文件系统。
- [x] M55 独立 `storage-server-runtime`：ABI v25 syscall 47—52、standalone StorageServer ELF、无 DUP/TRANSFER volume/session、strict v2 4160-byte/最多 8-sector transport、userspace namespace、100 Hz logical + physical one-shot scheduling、idle restart/rebind 与三启动 `0→4→5→5` 已封口；第三次 AppData write/flush 为零。release 三启动、严格 Clippy/host/feature matrix 与 2026-07-17 完整 `./scripts/test.sh` 均已通过，终态含 `storage_server_static=1 storage_server_runtime_boots=3 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M56 完成三类受控 session-fatal in-flight failure、kernel-only device reset/re-negotiate/queue rebuild 与两阶段 IRQ rearm、replacement epoch+1 remount；失败仍 fail-closed。
- [x] M57 完成两轮串行 `WRFWRF`、driver suppression `2/2/2`、IRQ-safe broker access、Running-owner `ServiceAbandoned`、physical submission gate，以及一次 IRQ commit abort 后真正成功 retry。
- [x] M58 把七次 physical recovery 拆为四相位 cooperative executor：driver borrow 跨相位释放、reset/capacity 每个 step 至多单采样、attempt id 贯穿 rearm/broker/admission、gate 完整 commit 后才开，并证明 timer/双 worker/已认证 EL0 progress window 与零 masked polling/long DAIF mask。
- [x] M59 建立两份独立账本：ABI-v24 AppData child 在一次 read QueueNotify loss 后冻结 outcome、kernel 零重放、userspace 仅对零输出 `FileOpenAt` `Unavailable` 重试一次；ABI-v23 timeout 自测复用 shared engine 且明确 `el0_progress_claim=0`。两者均显式 rearm/open，物理 commit 不隐式开放 admission。
- [x] M60 在 M58 child 中完成 `WRFWRFR`（六次瞬态 + 一次模拟永久 read）、ticketed policy、cap 3、`2×2` backoff、Probation/Healthy、kernel-prearmed 永久故障授权和 boot-local sticky Offline；终态 IRQ/DMA/direct proof 与 Offline 后零新增 attempt/reset/submission 均已动态封口。
- [x] M61 在 M60 fatal completion 上建立不可续期 250 ms physical-counter ticket，完成 exact monitor retirement、真实 ObjectWait abandonment、ordinary reaper barrier 与 epoch-7 replacement recovery；这不是通用 healthy-service heartbeat。
- [x] M66 完成八节点/10-edge fixed resident graph、kernel topology proof、三波逆拓扑 quiesce、opaque fail-closed token 与两次 QEMU-only self-exit；这不是完整 UI runtime 或硬件 poweroff。
- [x] 历史 M71 ABI-v32 五服务连续监督：事务式 5-service/4-edge 目录与批量 probe、21 个 cadence 轮/16 个额外健康轮、同窗两服务瞬态 miss 容忍与独立恢复、一次超限 StorageServer replacement，以及两次 PSCI QEMU self-exit 均已封口；不新增 syscall，明确 `arbitrary_soak_claim=0 real_phone_claim=0`。
- [ ] 下一 P0：推进目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak；用户指定并授权真实目标后，再实现 BSP/启动链/PMIC、hotplug/device replacement、真实 power-cut、SMP/IOMMU、真实控制器与真机恢复。
- [x] M26 在该下层之上建立 fw_cfg DMA/ramfb 与 virtio-input keyboard event path；
  `ACCESS_PLATFORM`/non-coherent DMA、IOMMU、通用文件写入/cache、认证/anti-rollback 与真实硬件恢复仍属后续。
- [x] M27 在 ramfb/keyboard 上建立 opaque scene + alpha cursor compositor、dirty redraw 和
  独立 virtio-tablet 的 7-event/3-sample touch interaction；仍无用户态 UI 或真实触屏。
- [x] M28 在 M27 上建立 kernel-owned clickable shell、tap capture/hit-test、Settings
  generation-1 damage commit 与 14-event/6-sample full-frame proof；该历史阶段为 `userspace_surface=0`。
- [x] M29 建立 ABI v14 EL0-owned single Surface、唯一 capability/session、input FIFO/coalescing、degraded owner-death 与 ramfb/headless 验收；337 项 host tests，完整套件为 `userspace_surface=1 userspace_ui=1`。
- [x] M30 建立 ABI v15 六镜像 catalog、独立 SurfaceServer/Launcher、单独 UI Channel pair 与七进程/19-handle 常驻拓扑；349 项 host tests。
- [x] M31 建立 ABI v16 七镜像 catalog、独立 SurfaceServer/Launcher/App、两对 UI Channel 与八进程/21-handle 常驻拓扑；默认 359 项 host tests，trace feature 另有 3 项 kernel 解码/路由测试。
- [x] M32 建立 ABI v17 两槽 transferable GraphicsBuffer、`BUP1`、rights attenuation/transfer、generation/scrub、失败原子性与真实 raster/copy-present；八进程/23-handle、377 host tests 与完整矩阵通过。
- [x] ABI v18 新增 init-only syscall 31 `ProcessTerminate`，覆盖 Runnable/Waiting target、generation-qualified PID、Exited/Faulted/Killed 终止原因和 exact wait-token abandonment/reap 账本。
- [x] M33 专用 `app-lifecycle-runtime` profile 完成 canonical 64-byte `ALC1`/`UBP1`/`USC1`、七事务 App1→App2 生命周期与五对常驻窗口通道；406 host tests及严格 QEMU 账本通过。
- [x] M34 专用 `app-crash-recovery-runtime` profile 完成 canonical USC1 `OwnerDied`、peer-close 资源/焦点/capture 清理、Killed 回收与 App2→App3 generation-safe restart；411 host tests、严格专用 QEMU 账本和扩展完整矩阵均通过。

### 5.3 显示驱动 TODO

- [x] M26 kernel-owned QEMU ramfb：320×480 XRGB8888 static splash，headless full-frame 验收。
- [x] M27 kernel-owned mutable dirty redraw 与 alpha cursor。
- [x] M28（历史）kernel-owned clickable home/settings shell 与严格受损面 commit。
- [x] M29 userspace-owned 单 Surface、输入读取与严格 present 边界。
- [x] M30 独立 EL0 SurfaceServer + Launcher 的认证单 Surface IPC。
- [x] M31 独立 App、显式 focus、按焦点 input、press-to-release capture 与 Present v2 generation/cancellation。
- [x] M32 transferable GraphicsBuffer/buffer-present。
- [x] M33 通用 app lifecycle 与最小窗口端点管理（专用 profile）。
- [x] M34 unexpected owner-death/crash cleanup/restart（专用 profile；清理后的 input routing 已接线，但尚无崩溃窗口输入注入验收）。
- [x] M35 mapped/shared GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；App RW/SurfaceServer RO 共享 75 页，BBM+TLBI、EL0 volatile reads、416 host tests 与完整矩阵通过。
- [x] M36 consumer owner-death abandon/release + producer recovery；Acquired QEMU、Queued/Acquired host tests、75 页 EL0 重写与零最终 graphics mappings/handles 已通过。
- [x] M37 SurfaceServer 自动重启/rebind + resident mapped App frame recovery；专用 QEMU 与完整从头矩阵均已通过。
- [x] M38 producer-death orphan + two-slot scrub/reuse；dedicated QEMU strict validator、checker、host suite 418 与完整矩阵均已通过。
- [x] M39 ABI-v20 software frame clock/单 grant gated present。
- [x] M40 resident two-buffer software-paced swapchain、ABA commit 与 post-copy release。
- [x] M41 bounded userspace multi-window compositor：z-order、occlusion、damage、raise、focus/input capture 与两槽输出。
- [x] M42 持久事件驱动窗口会话、严格 phone 边界、peer-close 清理、generation-safe recreate 与通用事件序列。
- [x] M43 focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44 SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45 独立 bounded InputServer、唯一 InputCapability、capacity-64 broker 与 broker-only input route。
- [x] M46 SurfaceServer restart/rebind、route-epoch gap recovery 与 capture cancellation。
- [x] M47 InputServer 一次 restart/reacquire、fixed 30 ms backoff、Surface resync 与 InputAcquire permission denial。
- [x] M48 single-InputServer `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、首个 process-exit 重启、第二次 health-timeout、budget 1 runtime quarantine、degraded UI 与严格 trace/validator/topology；dedicated QEMU、host/static matrix 与最终全量套件均已通过。
- [x] M49 依赖感知 `ServiceSupervisor::<2>`：SurfaceServer+InputServer、`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口；ABI 保持 v23。
- [x] M50 保持 ABI-v23/syscall 0—41/八镜像/capacity 不变，完成恢复后 phone 外拒绝、App physical `6/7`→两条 BWE→Present sequence 5/frame 3→output frame 4/write generation 4→health `2/2` resident 的 QEMU 封口。
- [x] M51 保持 ABI/镜像/capacity/context 不变，继承 M49/M50 并完成 Launcher physical `8/9` capture/release、focus `App/3→Launcher/4`、Present command 6/frame 4→output frame 5/write generation 5、health `2/2` resident 的离线 QEMU 封口。
- [x] M52 保持 ABI/镜像/capacity/context 不变，继承 M49/M50/M51 并完成 App physical `10/11` capture/release、focus `Launcher/4→App/5`、Present command 6/frame 4→output frame 6/write generation 6 的离线 QEMU roundtrip 封口。
- [x] M53 保持 ABI/镜像/capacity/context 不变，作为 M52 feature child 完成 session 2 的 `Ready→App/1→Launcher/2→App/3` 双 client 实时同步与 RFCK/LFCK/AFCK 收敛；channel=`14/14`、M52 APCK boundary=`1/1`，且不新增 QMP input 或 output frame。
- [ ] 产品级 InputServer、完整 IME/candidate/locale/font shaping、任意 Unicode、多点与物理输入设备栈。
- [ ] Display controller。
- [ ] Plane。
- [ ] VSync。
- [ ] Brightness。
- [ ] Rotation。
- [ ] Color profile。
- [ ] HDR metadata。
- [ ] Secure display path。

### 5.4 触控驱动 TODO

- [x] M26 virtio-input keyboard discovery、IRQ eventq/recycle 与 QMP A-event 验收（非触控）。
- [x] M27 QEMU virtio-tablet discovery、ABS range/coordinate transform、BTN_TOUCH 与 SYN report。
- [x] M28（历史）单点 press/release capture 与 hit-test prototype；M29 已把连续 input sequence 交付 EL0 shell。
- [ ] tap timeout、movement slop、cancel 与 gesture arbitration。
- [ ] 真实硬件 touch device discovery。
- [ ] Multi-touch packets。
- [ ] Coordinate transform。
- [ ] Sampling rate。
- [ ] Palm rejection。
- [ ] Stylus。
- [ ] Haptics。

### 5.5 GPU 路线 TODO

- [ ] QEMU 阶段使用软件渲染。
- [ ] virtio-gpu 阶段支持基本加速。
- [ ] Vulkan loader 接入。
- [ ] EGL 接入。
- [ ] OpenGL ES 接入。
- [ ] Buffer sharing。
- [ ] Fence sync。
- [ ] Protected content。
- [ ] 真机 GPU 驱动适配。

## 第 6 章：系统服务完整 TODO

### 6.1 Init

- [x] 由 init-only `ProcessSpawn`/`ProcessWait` 启动并监督两代独立 EL0 BSM1 ServiceManager；M20 历史双常驻 Client round 最终为五进程、16 endpoint/8 pair。M45—M53 与 M54/M59 UI/AppData profiles 为 capacity 9/12 contexts；M55—M65 StorageServer profiles 为 capacity 8/11 contexts；M66/M67 使用九镜像 catalog、9 个 dynamic child/13 contexts。M66 heap 为 512 KiB；M67 专用 heap 为 1 MiB、worker exception stack 为 64 KiB；feature-off default 仍为 capacity 8/11 contexts。
- [ ] 启动 LogServer。
- [ ] 启动 SecurityServer。
- [ ] 启动 StorageServer。
- [ ] 启动 PackageManager。
- [ ] 启动 PermissionManager。
- [ ] 启动 AppManager。
- [ ] 启动 WindowServer。
- [ ] 启动 Compositor。
- [x] 启动 bounded InputServer（M45 独立服务；M47 一次重启/reacquire；M48 单服务 health/quarantine/degraded witness）。
- [ ] 启动 NetworkServer。
- [ ] 启动 AudioServer。
- [x] 监控 ServiceManager 并执行一次 generation-safe 重启（通用服务监督仍待实现）。
- [x] ServiceManager 请求重启时保留 provider/client、确认 peer-close 并重绑。
- [x] M48 对单个 InputServer 完成 process-exit/health-timeout 分类、fixed 30 ms backoff、budget 1 与 runtime quarantine。
- [ ] 通用非请求崩溃、多服务依赖恢复、restart-storm 防护与安全模式策略。
- [ ] 安全模式。

### 6.2 ServiceManager

- [x] Register service（有界 Registry、单调 instance、duplicate rejection）。
- [x] Find service（成功/未知 lookup、原子 endpoint transfer、connector/business Channel）。
- [ ] List services。
- [x] 内部 wire protocol version（固定 64-byte BSM1 `protocol=1` 与 BSA1 v1 supervisor attach；尚非对外稳定接口）。
- [x] init-attested bootstrap session identity（role/epoch/owner PID；不是通用 caller credential）。
- [ ] General caller identity。
- [ ] Permission requirement。
- [x] 启动期 session capability grant（通用授权策略仍待实现）。
- [x] 有界 peer-close failure observation/rebind（通用 restart notification API 仍待实现）。
- [ ] Audit log。

### 6.3 LogServer

- [ ] Kernel log bridge。
- [ ] Service logs。
- [ ] App logs。
- [ ] Android logs。
- [ ] Ring buffer。
- [ ] Persistent crash log。
- [ ] Log filter。
- [ ] Developer log streaming。

### 6.4 PackageManager

当前仅有隔离的单包生命周期子集：Install-0/Update-0 有本地 QEMU 证据；
Uninstall-0 只接受受信离线 host 的 boot-time `BNDUNS01` `fw_cfg` 请求，双同代
tombstone 与 reinstall 约束有 43 项 package-store host tests 和 12-boot QEMU gate。
它不是运行时服务，没有 Settings 卸载、managed app data 或通用 PackageManager API。

- [ ] `.bapp` install。
- [ ] `.bapp` uninstall。
- [ ] `.bapp` update。
- [ ] APK install。
- [ ] APK uninstall。
- [ ] AndroidManifest parser。
- [ ] Manifest TOML parser。
- [ ] Signature verification。
- [ ] App sandbox creation。
- [ ] Resource indexing。
- [ ] Icon extraction。
- [ ] Intent/action registry。
- [ ] Rollback transaction。

### 6.5 PermissionManager

- [ ] Permission schema。
- [ ] Runtime grant。
- [ ] One-time grant。
- [ ] While-in-use grant。
- [ ] Background grant。
- [ ] System permission。
- [ ] Enterprise permission。
- [ ] Revoke。
- [ ] Audit。
- [ ] Android permission mapping。
- [ ] Native permission mapping。

### 6.6 AppManager

- [ ] Native app launch。
- [ ] Android app launch。
- [ ] Scene management。
- [ ] Activity mapping。
- [ ] Foreground/background。
- [ ] Suspended state。
- [ ] Process priority。
- [ ] OOM score。
- [ ] Crash handling。
- [ ] Recent tasks。
- [ ] Background task policy。

## 第 7 章：图形系统完整设计

### 7.1 组件

- DisplayServer
- SurfaceManager
- WindowServer
- Compositor
- AnimationEngine
- RenderScheduler
- ScreenshotService
- ScreenRecorder
- ColorManager

### 7.2 Surface 模型

所有应用都通过 Surface 提交内容。系统统一控制 Surface 的可见性、层级、透明度、圆角、阴影、动画、截图权限和安全显示。

TODO：

- [ ] Surface create。
- [ ] Surface destroy。
- [ ] Buffer allocate。
- [ ] Buffer submit。
- [ ] Buffer release。
- [ ] Fence wait。
- [ ] Damage region。
- [ ] Layer ordering。
- [ ] Secure layer。
- [ ] Screenshot policy。
- [ ] Android Surface bridge。

### 7.3 WindowServer TODO

- [ ] Window create。
- [ ] Window destroy。
- [ ] Focus management。
- [ ] Fullscreen。
- [ ] Split screen。
- [ ] Keyboard avoidance。
- [ ] Status bar region。
- [ ] Navigation region。
- [ ] Popup window。
- [ ] Modal window。
- [ ] Android Window mapping。

### 7.4 Compositor TODO

- [ ] Layer tree。
- [ ] GPU composition。
- [ ] Software fallback。
- [ ] VSync。
- [ ] Frame pacing。
- [ ] Animation timeline。
- [ ] Blur。
- [ ] Shadow。
- [ ] Rounded corners。
- [ ] HDR。
- [ ] Color management。
- [ ] External display。

## 第 8 章：原生应用框架

### 8.1 `.bapp` 格式

```text
Example.bapp
├── manifest.toml
├── signature
├── bin/main
├── lib
├── assets
├── resources
├── localization
└── privacy
```

### 8.2 manifest 示例

```toml
id = "com.bndroid.example"
name = "Example"
version = "1.0.0"
entry = "bin/main"
min_os = "0.1.0"

[permissions]
network = true
camera = false
microphone = false
location = "while_in_use"
notifications = true

[capabilities]
background_audio = false
background_location = false
file_picker = true
```

### 8.3 UI SDK TODO

- [ ] Text。
- [ ] Image。
- [ ] Button。
- [ ] List。
- [ ] ScrollView。
- [ ] Navigation。
- [ ] TabView。
- [ ] Sheet。
- [ ] Alert。
- [ ] TextField。
- [ ] Toggle。
- [ ] Slider。
- [ ] Picker。
- [ ] VideoView。
- [ ] WebView。
- [ ] Animation。
- [ ] Gesture。
- [ ] Theme。
- [ ] Accessibility。

### 8.4 原生生命周期

- Launching
- Active
- Inactive
- Background
- Suspended
- Terminated
- MemoryWarning
- PermissionChanged
- NetworkChanged
- ThemeChanged

TODO：

- [ ] 生命周期事件分发。
- [ ] 场景恢复。
- [ ] 状态保存。
- [ ] 后台任务申请。
- [ ] 通知唤醒。
- [ ] 崩溃恢复。

## 第 9 章：AndroidBox 完整设计

### 9.1 AndroidBox 目标

目标 AndroidBox 将让 Android APK 在 Bndroid OS 上运行，向 Android 应用提供熟悉的
Android API，并把实际调用映射到 Bndroid 系统服务。当前 DEX-0、Activity-0 回归、
ActivityLifecycle-1 与 Resources-1 仍只处理固定自有 APK 证据样本：DEX-0 只有两个
纯整数 `code_item`，旧 Activity shim 只有 inline text；ActivityLifecycle-1 只在纯数据
模型中解释 exact public no-argument constructor 的两条指令，再执行受限 `onCreate`；
Resources-1 只解析 default-config `resources.arsc`、一个 binary `TextView` layout 与
`android:text @string`，执行固定 `onCreate→setContentView(int)`。这些仍不构成该目标能力。

### 9.2 AndroidBox 组件

以下是目标组件树，不表示当前已经安装或运行这些组件：

```text
AndroidBox
├── androidboxd
├── app_process
├── zygote compatibility
├── ART
├── Bionic
├── linker
├── binder runtime
├── system_server shim
├── framework API shim
├── package bridge
├── permission bridge
├── window bridge
├── surface bridge
├── input bridge
├── audio bridge
├── media bridge
├── camera bridge
├── network bridge
├── storage bridge
└── notification bridge
```

### 9.3 通用 APK 安装 TODO

当前已完成的 DEX-0、Activity-0、ActivityLifecycle-1 与 Resources-1 precursor 只接受
仓库自有的严格固定 archive shape。bounded parser 已能读取固定 binary Manifest 与
strict default-config `resources.arsc`，基础 profile 不安装或发布 package state。
隔离 Install-0 的原始实证已对仓库自有 APK-v2 单签名 Resources-1 fixture 完成
signature-first admission、单包持久事务、首启安装和两次无源恢复；同一严格 shape 的
本机 `org.bndroid.macdemo` no-probe APK 也完成安装与无源恢复。两者均只走纯数据
constructor→`onCreate` 路径，不代表 ART 或一般 Android。严格受限 Update-0 仍只对
原始 demo 的同包、同证书、递增版本 v2→v3 fixture 完成 generation-2 原子更新、零写
replay/recovery 及 rollback/tamper 拒绝。boot-time Uninstall-0 的存储/kernel path
另以 canonical request 绑定 durable identity，用双同代 tombstone 逻辑撤销 APK；
blob 不擦除、没有 managed package data，其 12-boot QEMU gate 已验证
`1→2→removed 3→reinstall 4`。三者仍不满足下列通用安装、更新与卸载管线。

- [x] 严格受限 APK Install-0（原始 fixture 与同 shape 的 Mac no-probe APK、单包、
  无通用 PackageManager）。
- [x] 严格受限 APK Update-0（同包、同 v2 signer certificate、递增版本，
  generation 1→2；无 update UI 或通用多包更新策略）。
- [x] 严格受限 APK Uninstall-0 存储/kernel path（受信 host boot-time request、
  双同代 tombstone、逻辑不可达、43 项 package-store host tests 与 12-boot QEMU gate）。
- [ ] 通用解析任意合规 APK zip。
- [ ] 通用解析 AndroidManifest.xml。
- [ ] 通用解析 resources.arsc 与配置选择。
- [ ] 通用提取 dex。
- [ ] 通用提取 native libs。
- [ ] 生产 APK 信任策略及 v1/v3/v4、multi-signer、证书链/时间验证。
- [ ] 创建 Android app data。
- [ ] 映射 Android permissions。
- [ ] 注册 Activity。
- [ ] 注册 Service。
- [ ] 注册 Receiver。
- [ ] 注册 Provider。
- [ ] 注册 intent filters。

### 9.4 ART/Bionic TODO

- [ ] Bionic libc。
- [ ] pthread。
- [ ] linker namespace。
- [ ] dlopen。
- [ ] JNI。
- [ ] ART runtime。
- [ ] boot image。
- [ ] dex2oat 策略。
- [ ] JIT。
- [ ] interpreter fallback。
- [ ] GC。
- [ ] classloader。
- [ ] multidex。

### 9.5 Linux ABI 子集

- [ ] openat。
- [ ] read。
- [ ] write。
- [ ] close。
- [ ] mmap。
- [ ] munmap。
- [ ] mprotect。
- [ ] futex。
- [ ] epoll。
- [ ] eventfd（M9 历史里程碑的内核 Event object 不等于 Linux `eventfd` ABI）。
- [ ] timerfd。
- [ ] socket。
- [ ] connect。
- [ ] bind。
- [ ] listen。
- [ ] accept。
- [ ] ioctl subset。
- [ ] prctl subset。
- [ ] clone subset。
- [ ] sigaction。
- [ ] clock_gettime。
- [ ] getrandom。

### 9.6 Binder TODO

- [ ] binder_open。
- [ ] binder_mmap。
- [ ] binder_ioctl。
- [ ] transaction。
- [ ] reply。
- [ ] oneway。
- [ ] binder object。
- [ ] binder ref。
- [ ] death recipient。
- [ ] Android Binder service manager（M9 历史里程碑的 BSM1 是 Bndroid 内部原型，不等于此 Binder 兼容项）。
- [ ] Java Binder。
- [ ] Native Binder。
- [ ] AIDL。
- [ ] fd passing。
- [ ] permission hook。

### 9.7 Framework Shim TODO

第一批：

- [ ] Activity。
- [ ] Context。
- [ ] Intent。
- [ ] Bundle。
- [ ] Handler。
- [ ] Looper。
- [ ] View。
- [ ] TextView。
- [ ] Button。
- [ ] ImageView。
- [ ] SharedPreferences。
- [ ] SQLite。
- [ ] HttpURLConnection。

第二批：

- [ ] Service。
- [ ] BroadcastReceiver。
- [ ] ContentProvider。
- [ ] Notification。
- [ ] JobScheduler。
- [ ] AlarmManager。
- [ ] Clipboard。
- [ ] DownloadManager。
- [ ] WebView。

第三批：

- [ ] Camera2。
- [ ] MediaCodec。
- [ ] OpenGL ES。
- [ ] Vulkan。
- [ ] SensorManager。
- [ ] Bluetooth。
- [ ] LocationManager。
- [ ] BiometricPrompt。

## 第 10 章：安全体系

### 10.1 安全设计目标

- 应用默认隔离。
- 服务最小权限。
- IPC 可鉴权。
- 权限可撤销。
- 安装包可验证。
- 系统镜像可验证。
- 用户数据可加密。
- 崩溃可审计。

### 10.2 Code Signing TODO

- [ ] Root certificate。
- [ ] Developer certificate。
- [ ] Package signature。
- [ ] Binary signature。
- [ ] Dynamic library signature。
- [ ] Install verification。
- [ ] Launch verification。
- [ ] Update verification。
- [ ] Revocation list。

### 10.3 Sandbox TODO

- [ ] App identity。
- [ ] App data directory。
- [ ] Private cache。
- [ ] Temporary files。
- [ ] Network capability。
- [ ] Camera capability。
- [ ] Microphone capability。
- [ ] Location capability。
- [ ] Contacts capability。
- [ ] Photos capability。
- [ ] Clipboard policy。
- [ ] Background policy。

### 10.4 Keychain TODO

- [ ] Key generation。
- [ ] Key storage。
- [ ] Hardware binding。
- [ ] App access control。
- [ ] Biometric gate。
- [ ] Android Keystore bridge。
- [ ] Native Keychain API。
- [ ] Backup exclusion。
- [ ] Key destruction。

## 第 11 章：多媒体

### 11.1 AudioServer TODO

- [ ] Audio output。
- [ ] Audio input。
- [ ] Mixer。
- [ ] Volume。
- [ ] Audio focus。
- [ ] Low latency path。
- [ ] Bluetooth audio。
- [ ] Call audio。
- [ ] Android AudioTrack。
- [ ] Android AudioRecord。
- [ ] Native Audio API。

### 11.2 MediaServer TODO

- [ ] MediaExtractor。
- [ ] Decoder registry。
- [ ] Software decode。
- [ ] Hardware decode。
- [ ] Video rendering。
- [ ] Subtitle。
- [ ] DRM-protected content strategy。
- [ ] Android MediaPlayer。
- [ ] Android MediaCodec。
- [ ] ExoPlayer compatibility tests。

### 11.3 CameraServer TODO

- [ ] Camera device discovery。
- [ ] Preview stream。
- [ ] Capture stream。
- [ ] Video recording。
- [ ] Focus。
- [ ] Exposure。
- [ ] Zoom。
- [ ] Flash。
- [ ] Camera2 bridge。
- [ ] Native Camera API。

## 第 12 章：网络和通信

TODO：

- [ ] TCP/IP stack。
- [ ] DNS resolver。
- [ ] Wi-Fi manager。
- [ ] Cellular manager。
- [ ] VPN。
- [ ] Proxy。
- [ ] Per-app firewall。
- [ ] Traffic stats。
- [ ] Captive portal。
- [ ] Hotspot。
- [ ] Bluetooth networking。
- [ ] Android ConnectivityManager。
- [ ] Native Network API。

## 第 13 章：存储系统

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
/runtime/android/data
/runtime/android/cache
```

TODO：

- [x] M21 QEMU modern virtio-mmio v2 只读 polling block boundary：4096-sector fixture、queue 8、两帧 coherent DMA、sector 0/1 双读与三项存储负测。
- [x] M22 GICv2 IRQ completion、两个同时 outstanding request 与 timeout/reset/late-event 恢复。
- [x] M23 512-byte block abstraction、主备 GPT/CRC、单一只读 FAT16 分区与 `/system` VFS 启动期验证。
- [x] M24 两文件 immutable boot catalog、capability-scoped EL0 `FileOpenAt`/`VmoRead` 与 zero-runtime-disk-read 证明。
- [x] M25 private DATA 分区内的双槽 CRC record、GUID-bound epoch、有序 WRITE/FLUSH/readback、跨 QEMU boot-count persistence 与损坏最新槽回退；仍不是通用 block cache 或文件系统写入。
- [x] M26 fw_cfg DMA/ramfb static splash 与 virtio-input keyboard event path；283 host tests，
  screenshot/input QMP acceptance 与完整 test matrix 均通过。
- [x] M27 双层 compositor、dirty redraw 与 virtio-tablet interaction；297 host tests，
  前后全帧 pixel diff 与完整 test matrix 均通过。
- [x] M28（历史）kernel-owned clickable shell 与 Settings screenshot；323 host tests，
  `check-ui.sh` pixel diff 与完整 test matrix 均通过。
- [x] M29 ABI v14 EL0-owned single Surface 与 ramfb/headless UI；337 host tests，完整套件为 `userspace_surface=1 userspace_ui=1`。
- [x] M30 ABI v15 独立 SurfaceServer/Launcher、单独 UI Channel pair；七进程、19 handle、349 host tests。
- [x] M31 ABI v16 独立 SurfaceServer/Launcher/App、双 UI Channel、focus/input/capture 与 Present v2；八进程、21 handle、默认 359 host tests。
- [x] M32 ABI v17 transferable GraphicsBuffer/buffer-present、真实 App raster；八进程、23 handle、默认 377 host tests 与完整矩阵。
- [x] M33 ABI v18 专用 lifecycle profile；七事务、ALC1/USC1=`35/14`、最终八进程/29 handle/26 endpoint，并以同 slot 下一 PID generation 保留 App2。
- [x] M34 ABI v18 专用 crash-recovery profile；十事务、ALC message/transaction/state/crash=`46/10/19/1`、USC command/ack/owner-death=`9/9/1`，同 slot PID generation `1→2→3`，最终保留 App3。
- [ ] Read-only system image。
- [ ] User data partition。
- [ ] App sandbox directory。
- [ ] Media library。
- [ ] Document provider。
- [ ] Android scoped storage。
- [ ] File encryption。
- [ ] Backup。
- [ ] Restore。
- [ ] Quota。
- [ ] Storage cleanup。
- [ ] OTA snapshot。

## 第 14 章：OTA 和恢复

TODO：

- [ ] A/B slots。
- [ ] Update package format。
- [ ] Delta update。
- [ ] Signature verification。
- [ ] Download verification。
- [ ] Install transaction。
- [ ] Boot success marker。
- [ ] Failure rollback。
- [ ] Recovery mode。
- [ ] Factory reset。
- [ ] Data preserve update。

## 第 15 章：开发工具链

### 15.1 CLI 工具

- [ ] bndroid new。
- [ ] bndroid build。
- [ ] bndroid run。
- [ ] bndroid install。
- [ ] bndroid uninstall。
- [ ] bndroid log。
- [ ] bndroid debug。
- [ ] bndroid profile。
- [ ] bndroid sign。
- [ ] bndroid package。
- [ ] bndroid emulator。
- [ ] bndroid android-check。

### 15.2 模拟器

- [ ] QEMU ARM64 integration。
- [ ] Boot image build。
- [ ] System image build。
- [ ] Data image build。
- [ ] Snapshot。
- [ ] Log streaming。
- [ ] App install。
- [ ] Screenshot。
- [ ] Input injection。

### 15.3 调试工具

- [ ] Kernel debug log。
- [ ] Service trace。
- [ ] App crash dump。
- [ ] Android logcat bridge。
- [ ] Symbolication。
- [ ] Performance profiler。
- [ ] Frame profiler。
- [ ] Memory profiler。
- [ ] IPC inspector。
- [ ] Permission inspector。

## 第 16 章：系统应用

优先系统应用：

- Launcher
- Settings
- Files
- Installer
- Log Viewer
- Browser
- Gallery
- Camera
- Music
- Video
- Clock
- Calculator
- Contacts
- Phone
- Messages

每个系统应用 TODO：

- [ ] Rust 原生 UI。
- [ ] manifest。
- [ ] 签名。
- [ ] 权限声明。
- [ ] 沙箱目录。
- [ ] 系统服务调用。
- [ ] 暗色模式。
- [ ] 多语言。
- [ ] 无障碍。
- [ ] 崩溃恢复。

## 第 17 章：测试体系

### 17.1 Kernel Tests

- [ ] Boot test。
- [ ] Allocator test。
- [ ] Page table test。
- [ ] Thread test。
- [ ] Scheduler test。
- [ ] IPC test。
- [ ] Handle test。
- [ ] Syscall test。
- [ ] Fuzz syscall。

### 17.2 Service Tests

- [ ] Service registration。
- [ ] Permission check。
- [ ] Package install。
- [ ] App launch。
- [ ] Window create。
- [ ] Input route。
- [ ] Audio play。
- [ ] Network request。
- [ ] Notification post。

### 17.3 Android Compatibility Tests

- [ ] APK install。
- [ ] Manifest parse。
- [ ] ART boot。
- [ ] JNI load。
- [ ] Binder transaction。
- [ ] Activity lifecycle。
- [ ] View rendering。
- [ ] Touch input。
- [ ] Network。
- [ ] Storage。
- [ ] Notification。
- [ ] WebView。
- [ ] Media。

### 17.4 兼容性数据库

字段：

- app_name
- package_name
- version
- install_status
- launch_status
- login_status
- rendering_status
- network_status
- audio_status
- notification_status
- camera_status
- crash_log
- missing_api
- priority
- fix_status

## 第 18 章：性能目标

目标：

- QEMU 启动到 shell：初期 10 秒内，优化后 5 秒内。
- 真机启动到桌面：目标 5 秒内。
- 原生应用启动：1 秒内。
- Android 简单应用启动：3 秒内。
- UI：60fps，目标 120fps。
- 输入延迟：30ms 内。
- IPC：低延迟、可追踪。
- 空闲内存：可控。
- 后台应用：可冻结。
- 电量：有 power policy。

TODO：

- [ ] Boot profiling。
- [ ] Service lazy loading。
- [ ] Frame profiler。
- [ ] Memory profiler。
- [ ] IPC profiler。
- [ ] Android dex cache。
- [ ] Shader cache。
- [ ] Background freezer。
- [ ] Thermal policy。
- [ ] Power scheduler。

## 第 19 章：真机适配路线

阶段：

1. QEMU ARM64。
2. virtio-gpu 模拟器。
3. ARM64 开发板。
4. 开源硬件手机平台。
5. 指定量产设备。

TODO：

- [ ] Bootloader 适配。
- [ ] Device tree。
- [ ] Display。
- [ ] Touch。
- [ ] GPU。
- [ ] Wi-Fi。
- [ ] Bluetooth。
- [ ] Audio。
- [ ] Camera。
- [ ] Battery。
- [ ] Thermal。
- [ ] USB。
- [ ] Sensors。
- [ ] Modem bridge。

## 第 20 章：里程碑

### M0：Rust Kernel Hello

- [x] workspace。
- [x] no_std kernel。
- [x] QEMU。
- [x] boot log。
- [x] panic handler。

### M1：Kernel Core

- [x] memory。
- [x] page table。
- [x] thread（当前为固定有界 context/每进程一线程）。
- [x] scheduler（当前为单核固定容量实现）。
- [x] syscall/profile（feature-off/default 与 ABI-v23 timeout 为 syscall 0—41；M54/M59 AppData 为 ABI v24/syscall 0—46；M55—M64 StorageServer 为 ABI v25/syscall 0—52；M65 为 ABI v26，并只新增 init-only syscall 53）。
- [x] IPC（Channel/Event 有界子集）。
- [x] M21 QEMU modern virtio-mmio v2 只读轮询块设备（FDT 32-slot discovery、queue 8、two-frame DMA、双 sector 证据；仍非通用 storage stack）。
- [x] M22 GICv2 INTID 79 中断完成、两个同时 outstanding request，以及 timeout/reset/token invalidation/late-spurious 同两帧恢复（仍非通用 storage stack）。
- [x] M23 启动期只读 block/GPT/FAT16/VFS（8 MiB fixture、主备 CRC 与 FAT mirror、`/system` 两文件证据；仍无 EL0 storage API 或持久化）。
- [x] M24 把两个已验证文件发布为 2-entry/68-byte immutable boot catalog，向 EL0 传递 move-only directory capability 并验证 VMO open/read/transfer；仍无 write、mapping 或持久化。
- [x] M25 在 private DATA 分区实现 bounded raw-sector 双槽持久化、WRITE/FLUSH completion 顺序、重读校验、跨 QEMU generation 与损坏槽回退；EL0 仍无 write，且无通用文件写入/cache、exactly-once、认证或 anti-rollback。
- [x] M26 strict fw_cfg DMA/320×480 ramfb 与 virtio-input keyboard IRQ path；仍只有
  历史 static splash/keyboard 边界。
- [x] M27 kernel-owned opaque scene + alpha cursor compositor、dirty redraw 与独立
  virtio-tablet 单点 touch interaction；仍没有 userspace graphics/surface/window/widget/text/gesture。
- [x] M28 kernel-owned clickable shell、三个 app target + home、Settings damage commit；
  该历史阶段为 `owner=kernel userspace_surface=0`，ramfb 仍无 vblank/double buffer。
- [x] M29 ABI v14 EL0-owned single Surface；`owner=userspace userspace_surface=1 userspace_ui=1`，handles by image `4/6/3/4`（总计 17），manager wait items 为 5、Surface 位于 index 0。
- [x] M30 ABI v15 独立 SurfaceServer/Launcher；handles by image `4/5/3/4/2/1`（总计 19），core/UI Channel 为 16/2 endpoint，Surface 只归 server，manager wait 不含 Surface。
- [x] M31 ABI v16 增加独立 App；handles by image `4/5/3/4/3/1/1`（总计 21），core/UI Channel 为 16/4 endpoint，Surface 只归 server，双 client 均无 Surface。

### M2：Userland

- [x] init（构建期嵌入的严格 ELF64 原型）。
- [x] 独立、init-only 监督的 BSM1/BSA1 ServiceManager 原型（Registry 容量 4、两代 bootstrap、M14/M15 基线、M16 post-cleanup 有界复用、M17 sender identity/固定 PID ACL、M18 历史双 Client 并发，以及 M20 两条独立常驻 session/一次 revoke-reattach 隔离；精确 16-endpoint graph/四 token 持续稳定，`multi_session_round=1 general_runtime=0`，仍非产品级 resilient daemon）。
- [ ] LogServer。
- [ ] shell。
- [ ] file service。

### M3：Graphics

- [x] M26--M28 历史 kernel-owned QEMU ramfb、双层 compositor/clickable shell；M29 完成 userspace-owned 单 Surface，M30 完成独立 SurfaceServer/Launcher，M31 增加独立 App/focus/input routing，M32 增加 transferable GraphicsBuffer/真实像素 copy-present，M33 专用 profile 增加生命周期与 generation-safe endpoint replacement，M34 专用 profile 增加 owner-death 清理与 crash restart。
- [x] M45 bounded InputServer（独立服务、唯一 InputCapability、kernel broker、server-owned route/focus/capture/text-context/IME）。
- [x] M47 bounded InputServer recovery（一次 restart/reacquire、30 ms backoff、route resync、permission denial/audit）。
- [x] M48 single-InputServer `ServiceSupervisor::<1>` health/watchdog、runtime quarantine 与 degraded UI（不等于多服务监督）。
- [ ] 产品级 InputServer（仍缺 dependency-aware SurfaceServer+InputServer 多服务恢复、degraded→recovered、restart-storm 防护、完整 IME/candidate/locale/font shaping、任意 Unicode、多点或物理设备栈）。
- [ ] WindowServer。
- [ ] 完整用户态 Compositor（已封口 persistent capacity-2 userspace policy、strict boundary 与 generation-safe recreate，但任意窗口、自动 restart、硬件 vblank 与重复 crash policy 仍未完成）。
- [x] M32 transferable GraphicsBuffer/buffer-present。
- [x] M33 通用 app lifecycle 与最小窗口端点管理（专用 profile）。
- [x] M34 unexpected owner-death/crash cleanup/restart（专用 profile；input 路由代码已接通，注入验收待补）。
- [x] M35 mapped/shared GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence。
- [x] M36 consumer owner-death cleanup 与 producer recovery；display 仍为 Degraded。
- [x] M37 SurfaceServer 自动重启/rebind 与 resident mapped App 帧恢复；专用 QEMU 已通过。
- [x] M38 producer-death orphan 与 two-slot scrub/reuse；dedicated QEMU strict validator/checker 已通过。
- [x] M39 software frame clock/单 grant gated present。
- [x] M40 fixed two-buffer ownership/scheduling 与最终 B3 Acquired 证据。
- [x] M41 bounded multi-window z-order/occlusion/damage/input-routing proof。
- [x] M42 persistent window session、phone bounds、peer-close cleanup、generation-safe recreate 与通用事件序列。
- [x] M43 focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。

### M4：Native Apps

- [ ] `.bapp`。
- [ ] PackageManager。
- [ ] PermissionManager。
- [ ] Native SDK。
- [ ] Hello app。

### M5：AndroidBox MVP

- [x] DEX-0 precursor：固定自有 APK、固定两个纯整数 `code_item`、guest QEMU
  authenticated execution；不计为通用 APK parser/installer、ART、Activity 或通用 APK。
- [x] Activity-0 precursor：解析同一 fixture 的 binary Manifest、选择固定 exported
  MAIN/LAUNCHER、执行真实 `MainActivity.onCreate(Bundle)V` 与四调用 TextView shim；
  不计为通用 APK parser/installer、ART、通用 Activity/Framework 或通用 APK。
- [x] Resources-1 precursor：严格解析 resource fixture 的 default-config
  `resources.arsc`、一个 binary `TextView` layout 与 `android:text @string`，执行固定
  `onCreate→setContentView(int)` 并通过 QEMU gate；不计为 ResourceManager、
  qualifier/alias、任意 View/layout、通用资源系统或通用 APK。
- [x] 严格受限 APK Install-0：原始仓库 APK-v2 单签名 Resources-1 fixture 完成
  signature-first admission、单包双 registry/双 blob 持久事务、首启安装、两次无源
  generation-1 恢复、篡改拒绝和 ABI-43/syscall-59 只读安装快照；同一严格 shape 的
  本机 `org.bndroid.macdemo` no-probe APK 也完成安装/恢复，仍不代表任意 APK。
- [x] 严格受限 APK Update-0：同一包和 signer certificate 的 v2→v3 fixture 经完整
  readback 原子发布为 generation 2；同源 replay/无源恢复零写，rollback 与篡改
  source 拒绝且磁盘不变。
- [x] 严格受限 APK Uninstall-0 存储/kernel path：只接受 boot-time
  `BNDUNS01`，双同代 tombstone、逻辑 APK revocation、无 managed package data，
  package-store 共 43 项 host tests，并已留存 12-boot QEMU gate。
- [ ] 通用 APK parser/installer：任意合规 APK、多包、更新/卸载、生产 signer policy
  与 Installer/PackageManager UI/API。
- [ ] Bionic。
- [ ] ART。
- [ ] Binder。
- [ ] Activity。
- [ ] Surface bridge。
- [ ] Input bridge。
- [ ] 首个依赖 ART、ActivityThread 与通用 Framework 的 APK。

### M6：Developer Preview

- [ ] SDK。
- [ ] Emulator。
- [ ] Debugger。
- [ ] Profiler。
- [ ] Sample apps。
- [ ] Compatibility tests。

## 第 21 章：人员和资源

个人路线：

- 3 个月：Rust kernel 启动。
- 6 个月：用户态和简单 GUI。
- 12 个月：原生应用框架原型。
- 18 到 24 个月：AndroidBox 极简 APK。

小团队路线：

- 内核 2 人。
- 系统服务 2 人。
- 图形 1 到 2 人。
- Android 兼容 2 到 4 人。
- SDK 1 人。
- 测试 1 到 2 人。

完整团队路线：

- 内核 3 到 5 人。
- 驱动 3 到 8 人。
- 图形 2 到 5 人。
- Android Framework 5 到 10 人。
- 系统服务 5 到 10 人。
- 安全 2 到 4 人。
- SDK 2 到 5 人。
- 测试 5 到 15 人。

## 第 22 章：最高优先级总表

### P0

- [x] Rust workspace。
- [x] AArch64 boot。
- [x] QEMU。
- [x] UART log。
- [x] page table。
- [x] allocator。
- [x] scheduler（单核固定容量基础）。
- [x] syscall（feature-off/default 与 timeout 保持 ABI v23/syscall 0—41；M54/M59 AppData 为 ABI v24/syscall 0—46；M55—M64 StorageServer 为 ABI v25/syscall 0—52；M65 为 ABI v26/syscall 0—53；M66 为 ABI v27/syscall 0—54；M67/M68/M69/M70/M71 分别为 ABI v28/v29/v30/v31/v32 且均不新增 syscall；M72 为 ABI v33/init-only syscall 55 `ServiceManifestOpen`；M73 为 ABI v34/init-only syscall 56 `ServiceSupervisorReport`；M74/M75/M76 分别为 ABI v35/v36/v37 且不新增 syscall；M77 为 ABI v38，并新增 init-only syscall 57 `MaintenanceSessionOpen`；M78/M79/M80 分别为 ABI v39/v40/v41 且不新增 syscall。54 仍是 child-only `ServiceShutdown`；M80 raw 58 未知）。
- [x] IPC（Channel/Event 有界子集）。
- [x] init（构建期嵌入 ELF 原型）。
- [x] 独立、init-only 监督的 BSM1/BSA1 ServiceManager 原型（Registry 容量 4、一次 restart、当前六独立镜像/process capacity 7；保留 M15/M16 历史生命周期、M17 kernel sender identity/固定 PID ACL/delegated-endpoint 拒绝、M18 历史双 Client 并发，以及 M20 两个常驻 Client 的独立 session/一次 revoke-reattach 隔离；任意接入、重复 churn、crash 隔离的产品级 resilient runtime 与产品 ACL 仍属后续）。
- [x] M22 modern virtio-mmio v2 只读 IRQ 双请求 boundary；M23—M31 依次加入 storage、ramfb/input、Surface 与独立 App；M32 增加 transferable GraphicsBuffer/真实像素 copy-present；M33 专用 profile 增加有界 App lifecycle 与窗口 endpoint replacement；M34 专用 profile 增加一次 owner-death 清理和 generation-safe crash restart。产品范围仍为 `general_runtime=0`，没有 EL0 通用文件 write、通用 mapping/shared memory 或零拷贝图形，且 `crash_consistency=0`。
- [x] M26--M28 历史 kernel-owned framebuffer/compositor/clickable shell 与 headless screenshot acceptance；M29 userspace Surface/UI acceptance；M30 独立 UI 进程边界；M31 双 client focus/input/capture 边界。
- [ ] PackageManager。
- [ ] PermissionManager。
- [ ] `.bapp`。
- [ ] Native Hello World。

### P1

- [ ] WindowServer。
- [ ] Compositor。
- [ ] InputServer。
- [x] 严格受限 APK Install-0（原始仓库 fixture 与同 shape 的 Mac no-probe APK）。
- [x] 严格受限 APK Update-0（同包同证书 v2→v3、generation 2、零写稳定恢复）。
- [x] 严格受限 APK Uninstall-0 存储/kernel path（boot-time request、双墓碑、
  43 项 host tests 与 12-boot QEMU gate）。
- [ ] 通用 Android APK parser/installer、多包更新/卸载及 Installer/PackageManager UI。
- [ ] Bionic。
- [ ] ART。
- [ ] Binder。
- [ ] Android Activity。
- [ ] Surface bridge。
- [ ] Network bridge。
- [ ] Storage bridge。

### P2

- [ ] AudioServer。
- [ ] NotificationServer。
- [ ] WebView。
- [ ] Camera2。
- [ ] MediaCodec。
- [ ] Vulkan。
- [ ] OpenGL ES。
- [ ] OTA。
- [ ] Secure boot。
- [ ] 真机适配。

## 第 23 章：最终执行路线

最好的执行路线是从可启动、可测试、可演示的最小系统开始：

1. 建立 Rust workspace。
2. 建立 no_std kernel。
3. 在 QEMU ARM64 启动。
4. 实现串口日志和 panic。
5. 实现页表和内存分配。
6. 实现线程和调度器。
7. 实现 IPC 和 handle。
8. 启动用户态 init。
9. M13—M20 历史阶段已实现四独立镜像、一次监督重启、M15 十阶段生命周期、M16 post-cleanup 有界复用、M17 authenticated ACL、M18 历史双 Client、M19 wait-array，以及 M20 两个常驻 Client 的独立 session 与一次 stalled-secondary revoke/reattach 隔离；M21 加入 modern virtio-mmio v2 只读轮询，M22 完成 GICv2 IRQ、双 outstanding 与 timeout/reset/late-event 恢复，M23 加入启动期 block/GPT/FAT16/只读 VFS，M24 加入两文件 immutable boot catalog，M25 加入 private DATA durable record，M26 再加入 strict fw_cfg DMA/ramfb 与 virtio-input keyboard，均未改变服务账本。
10. M27 已实现最小 software compositor、mutable dirty redraw、virtio-tablet pointer/touch 与 interaction test。
11. M28（历史）已实现 kernel-owned clickable shell、Settings damage commit 与 screenshot；当时为 `userspace_surface=0`。
12. M29 已完成 canonical 64-byte `bndr-ui`、严格先全验证后 raster、ABI v14 syscall/EL0 唯一 capability/session，以及运行时 `KernelFallback→UserspaceBound` 单向切换；337 项 host tests，完整套件为 `userspace_surface=1 userspace_ui=1`。
13. M30 已拆出独立 EL0 SurfaceServer 与 Launcher；ABI v15 只增加镜像身份、syscall `0..27` 不变，并以单独 UI Channel 完成认证单 Surface IPC。
14. M31 已增加独立 App、第二对 UI Channel、显式 focus、按焦点 input 与 generation-qualified Present v2。
15. M32—M54 已建立 UI/AppData 历史基线；M55—M66 完成 standalone StorageServer、恢复、Offline/owner/quarantine、durable hint/close、resident shutdown 与 QEMU-only exit；M67 实证真实 UI/InputServer→AppData→M66 closure 的统一路径；M68/M69 再证明单服务恢复与固定双服务 hard dependency；M70 证明 strict FDT `/psci`、PSCI_VERSION 1.1 与 QEMU SYSTEM_OFF；历史 M71 证明事务式五服务/四依赖目录、批量 probe、16 个额外健康轮、同窗两服务瞬态漏报恢复与一次升级 StorageServer replacement，但仍不是 general runtime、运行时任意服务发现、非注入长期 watchdog、PMIC 或 hardware poweroff。下一硬件 P0 需用户指定并授权目标后再实现 BSP/启动链/真实控制器/PMIC；背靠背升级故障、任意时长 soak、真实 power-cut、SMP/IOMMU 与真机 recovery 仍未证明。
15. 实现 PackageManager 和 PermissionManager。
16. 跑通原生 Hello World。
17. 严格受限 APK Install-0 已完成原始仓库 fixture 与同 shape 的 Mac no-probe APK。
18. 严格受限 APK Update-0 已完成同包同证书 v2→v3、generation-2 原子更新。
19. 严格受限 APK Uninstall-0 已完成 12-boot QEMU gate 与 reinstall 恢复。
20. 实现通用 Android APK parser/installer、多包更新/卸载及更新 UI。
21. 适配 Bionic。
22. 启动 ART。
23. 实现 Binder 兼容。
24. 实现 Activity 和 Surface bridge。
25. 跑通首个依赖 ART、ActivityThread 与通用 Framework 的 Android APK。

这套路线既保留了非 Linux 内核和 Rust-first 的系统理想，又让 Android 应用兼容有清晰落点。系统核心用 Rust，兼容层承接 AOSP，图形和多媒体逐步补齐，最终形成一个自研移动 OS。


## 第 24 章：Rust 内核工程展开

### 24.1 最小内核启动链

Bndroid OS 的第一条可执行路线是 Rust no_std kernel。启动链可以按 bootloader、AArch64 汇编入口、Rust kernel_main、内存初始化、异常初始化、调度器初始化、用户态 init 加载展开。每一步都要有可观测日志和验收标准。

TODO：

- [x] `boot.S` 设置栈并跳转 `kernel_main`。
- [x] `kernel_main` 初始化 UART。
- [x] `memory::init` 接收设备树内存范围。
- [x] `paging::init` 创建内核页表。
- [x] `heap::init` 建立内核堆。
- [x] `exceptions::init` 设置异常向量。
- [x] `timer::init` 启动系统 tick。
- [ ] `scheduler::init` 创建 idle thread。
- [x] `syscall::init` 建立按 feature 区分的入口：default/M53 与 timeout 为 ABI v23/syscall 0—41，M54/M59 AppData 为 ABI v24/syscall 0—46，M55—M64 StorageServer 为 ABI v25/syscall 0—52，M65 为 ABI v26/syscall 0—53，M66 为 ABI v27/syscall 0—54，M67/M68/M69/M70/M71 分别为 ABI v28/v29/v30/v31/v32 且均不新增 syscall；M72 为 ABI v33/syscall 0—55，其中 55 `ServiceManifestOpen` 仅允许 init 且 reserved/flags 必须为零；M73 为 ABI v34/syscall 0—56，其中 56 `ServiceSupervisorReport` 仅允许 init，UI query 只读且转换报告由 kernel process ledger 校验；M74 为 ABI v35/syscall 0—56，在相同 init-only VMO 发布前增加 BMS1 RSA 签名与静态 rollback floor 门禁；M75 为 ABI v36/syscall 0—56，在任何 EL0 启动前增加双槽持久 floor transaction；M76 为 ABI v37/syscall 0—56，增加有序 fixture keyring、持久 key epoch/anchor/policy binding 与 pre-EL0 retired-key rejection；M77 为 ABI v38/syscall 0—57，其中 57 `MaintenanceSessionOpen` 仅允许 init，并以准备完成的 BMA1/`BNDRMAU1` 状态门禁 mutating supervisor report；M78/M79 分别为 ABI v39/v40/syscall 0—57，加入独立 `BNDRMEX1` completion 与 `BNDRMST1` step head；当前 M80 为 ABI v41/syscall 0—57，继续保持 UI query 只读并加入独立 `BNDRMPL1` plan head。
- [x] M77/ABI-v38 maintenance authorization：signature-first BMA1 精确绑定、双槽 `BNDRMAU1` exact-sequence SHA-256 hash chain、write/flush/readback、boot-local session gate，以及坏签名/错误 binding/replay 的 pre-EL0 零槽改写拒绝已由六正三负 release QEMU 门封口；不声称 trusted monotonic backend、生产 HSM/RPMB/eFuse、硬件 power-cut 或真机。
- [x] M78/ABI-v39 maintenance execution：独立双槽 `BNDRMEX1` completion ledger、前序完成门禁、`audit2/execution1` 精确同授权零 audit 写恢复、kernel-validated `audit2/execution2` completion 与 completed-replay pre-EL0 零槽改写拒绝，已由两次 PSCI 正启动、一次宿主中断和三次负启动封口；不声称 exactly-once、arbitrary resume、trusted monotonic backend、硬件 power-cut 或真机。
- [x] M79/ABI-v40 maintenance step journal：相对 sector 9/10 的双槽 376-byte `BNDRMST1` 固定记录 rotation1/rotation2/drain，精确绑定 authorization/effect/hash chain，支持 M78 migration anchor、已持久步骤只读 reconciliation、三个 post-marker cut、损坏最新槽回退/修复，并把 terminal chain 绑定进 aggregate completion；五次 PSCI 正启动、三次宿主中断和三次负启动通过；不声称外部副作用 exactly-once、arbitrary resume、trusted monotonic backend、硬件 power-cut 或真机。
- [x] M80/ABI-v41 maintenance plan：相对 sector 11/12 的双槽 424-byte `BNDRMPL1` 为三个固定 operation 记录 plan/operation-instance/idempotency identity、九次 `PREPARED→APPLYING→CONFIRMED` transition 和 apply 前 compensation；prepared/applying/effect/cancel 四次宿主中断、result-unknown/effect-observed reconciliation、损坏最新槽回退/修复、terminal plan chain binding 与五份终态磁盘收敛均通过；不声称外部副作用 exactly-once、arbitrary resume、trusted monotonic backend、硬件 power-cut 或真机。
- [x] `userboot` 从 profile-sized all-or-none catalog 选择镜像；M45—M53 与 M54/M59 为 capacity 9/12 contexts；M55—M65 StorageServer profiles 为八镜像 catalog、capacity 8/11 contexts；M66 复用九镜像 catalog并同时保留 9 个 dynamic child/13 contexts。feature-off default 保持七镜像、capacity 8/11 contexts。
- [x] M16（历史）静态验证已通过：166 host tests、fmt、workspace check、default/all-feature AArch64 Clippy 与四个 userspace bin Clippy；当时的完整 QEMU matrix、immutable Debug SHA256 与 16 路压力也已通过。该历史 hash/stress 结果不能继承给 M21/M22/M23/M24/M25。
- [x] M20 服务基线完成两条独立常驻 Client session、session-routed reply、一次 stalled-secondary revoke/reattach/stale-lease 隔离及五进程精确空闲图，`general_runtime=0`。
- [x] M21 完成 FDT 32-slot virtio-mmio 发现、modern v2 `VERSION_1 | RO`、queue 8/two-frame coherent DMA、只读双 sector polling 证据与 legacy/corrupt/no-device 负测。完整 `./scripts/test.sh` 从头通过 199 项 host tests、双 Clippy、五种 normal QEMU 及全部负测；本轮没有 immutable kernel image 或 16 路压力证据。
- [x] M22 完成 GICv2 interrupt-parent/SPI 解析、INTID 79 路由、两个 generation-qualified request slot、IRQ-only normal completion 与 timeout/reset/late-event 恢复。完整 `./scripts/test.sh` 从头通过 208 项 host tests、双 AArch64 Clippy、五种 normal QEMU 及全部负测；本轮仍没有 immutable kernel image 或 16 路压力证据。
- [x] M23 完成 512-byte block reader、主备 GPT CRC/entry mirror、只读 FAT16 与 `/system` VFS；确定性 8 MiB fixture、226 项 host tests、双 AArch64 Clippy、五种 normal QEMU、六项存储负测、IRQ race 与全部既有负测从头通过。本轮仍没有 immutable kernel image 或 16 路压力证据。
- [x] M24 完成 `BootfsCatalog<2>`/immutable VMO、ABI v13 `FileOpenAt`/`VmoRead` 与 EL0 内容、EOF、fault、rights、stale、transfer 证明；248 项 host tests、双 AArch64 Clippy、五种 normal QEMU 与完整 matrix 从头 exit 0。本轮仍没有 immutable kernel image 或 16 路压力证据。
- [x] M25 完成 private DATA 双槽 CRC record、16-byte GUID-bound epoch、相邻 generation、IRQ WRITE completion 后 FLUSH completion 与双槽 readback；同一 raw image 跨 QEMU 为 `0→1→2`，损坏最新槽可回退并重新提交。完整 `./scripts/test.sh` 从头 exit 0：269 项 host tests（`16/19/31/0/203`）、双 AArch64 Clippy、五种 normal QEMU、11 类 storage negative、M22 race 与全部既有负测通过。本轮仍没有 immutable kernel image 或 16 路压力证据。
- [x] M26 完成 strict `qemu,fw-cfg-mmio`/DMA、320×480 XRGB8888 ramfb 与
  modern virtio-input keyboard；host suite 283（`16/19/31/0/217`），完整测试最终输出
  `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 input_qmp=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M27 完成 opaque scene + alpha cursor compositor、dirty redraw、独立 virtio-tablet
  7-event/3-sample touch interaction；host suite 297（`16/19/31/0/231`），完整测试最终输出
  `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M28（历史）完成 kernel-owned clickable shell、Settings generation-1 commit 与 14-event/
  6-sample UI screenshot；host suite 323（`16/19/31/11/0/246`），完整测试最终输出 `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 clickable_ui=1 ui_screenshot=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M29 完成 ABI v14 EL0-owned single Surface、input `1..21`、frame `1..13`、ramfb/headless UI；host suite 337（`16/19/31/17/0/254`），完整测试最终输出 `BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M30 完成 ABI v15 六镜像与独立 SurfaceServer/Launcher UI IPC；七进程、18 endpoint/9 pair、19 handle，host suite 349（`16/19/31/29/0/254`），完整 `./scripts/test.sh` 从头 exit 0。
- [x] M31 完成 ABI v16 七镜像与独立 SurfaceServer/Launcher/App UI IPC；八进程、20 endpoint/10 pair、21 handle，默认 host suite 359（`16/19/31/39/0/254`）。默认/全 feature check 与 Clippy、五种 normal QEMU、全部负测、storage/persistence/IRQ race、24-input/13-commit capture baseline 与连续两次 36-input/19-commit Present cancellation/retry stress 均通过，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M32 完成 ABI v17 transferable GraphicsBuffer/buffer-present；八进程、20 endpoint/10 pair、23 handle，默认 host suite 377（`19/19/31/45/0/263`），24-input/13-commit buffer baseline、36-input/18-commit stress 与完整 `./scripts/test.sh` 均通过。
- [x] M33 完成 ABI v18 专用 `app-lifecycle-runtime`：canonical 64-byte `ALC1`/`UBP1`/`USC1`，七事务、ALC/USC message=`35/14`、USC operations=`2/3/1/1`；App1 正常退出后 App2 同 slot 下一 generation，created/exited/reaped/live=`10/2/2/8`，最终 29 handle、26 endpoint/13 pair、五对窗口通道与 wait=`8`（many/array=`2/6`）。workspace host suite 406（`20/19/31/73/0/263`），完整 `./scripts/test.sh` 从头 exit 0并固定 `process_terminate=1 app_lifecycle=1`。
- [x] M34 完成 ABI v18 专用 `app-crash-recovery-runtime`：十事务，ALC message/transaction/state/crash=`46/10/19/1`，USC command/ack/owner-death=`9/9/1`、operations=`3/4/1/1`；App2 unexpected Killed 后 Surface 原子清理并由 App3 同 slot generation `1→2→3` 重启，created/exited/reaped/live=`11/3/3/8`、reasons=`2/0/1`、terminate=`1/1`、graphics generation=3，最终 29 handle、26 endpoint/13 pair、wait=`8`（many/array=`2/6`，abandoned=`0/0/1`）。host suite 411（`20/19/31/78/0/263`）、专用 QEMU checker和含 `app_crash_recovery=1` 的完整 `./scripts/test.sh` 均通过；crash-window input 仍无注入验收。
- [x] M35 ABI-v19 mapped/shared GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；host suite 416（`20/19/31/78/0/268`），专用 QEMU 与含 `mapped_graphics=1` 的完整套件通过。
- [x] M36 ABI-v19 graphics consumer owner-death cleanup；mapping pin 权威、consumer teardown-before-producer restore、Acquired QEMU、Queued/Acquired host tests 与显式 App cleanup 已通过；其 M37 full-suite 基线为 417（`20/19/31/78/0/269`）。
- [x] M37 SurfaceServer 自动重启/rebind + resident mapped App frame recovery；`scripts/check-graphics-surface-restart.sh` 与完整从头矩阵均已通过。
- [x] M38 producer-death orphan + two-slot scrub/reuse；host suite 418（`20/19/31/78/0/270`）、专用 QEMU/checker 与完整矩阵均已通过。
- [x] M39 ABI-v20 software frame clock、single grant、三次 gated mapped commit 与最终 Ready opportunity。
- [x] M40 resident two-buffer software-paced swapchain proof。
- [x] M41 bounded multi-window compositor：z-order、occlusion、damage、raise、focus 与 input routing。
- [x] M42 持久事件驱动窗口会话、phone bounds、peer-close cleanup、generation-safe recreate 与通用事件序列。
- [x] M43 focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44 SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45 独立 bounded InputServer、唯一 capability、kernel broker 与 zero Surface FIFO fallback。
- [x] M46 SurfaceServer restart/rebind、route epoch `1→2`、gap release、App capture/contact cancel；dedicated QEMU、M45 regression、650 host/335 kernel 与完整总套件均已通过。
- [x] M47 InputServer 一次 restart/reacquire、fixed 30 ms backoff、Surface route resync、InputAcquire permission denial；674 host/338 kernel、pre/gap/post QEMU 与完整总套件均已通过，quarantine 仅 host-verified。
- [x] M48 single-InputServer `ServiceSupervisor::<1>`、strict `BSH1`、process-exit restart、health-timeout、budget 1 runtime quarantine、degraded UI 与严格 trace/validator/topology；694 host/338 kernel、dedicated QEMU、静态矩阵与最终全量套件均已通过。
- [x] M49 dependency-aware `ServiceSupervisor::<2>`：SurfaceServer+InputServer、`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。

### 24.2 内核 crate 设计

推荐内核只暴露极少稳定接口，其余模块内部演进。核心模块之间通过明确类型传递资源，避免裸整数、裸指针和魔法常量扩散。

关键类型：

- `KernelResult<T>`
- `KernelError`
- `PhysAddr`
- `VirtAddr`
- `PageFrame`
- `AddressSpace`
- `ThreadRef`
- `ProcessRef`
- `HandleValue`
- `Rights`
- `ChannelEndpoint`
- `VmObject`

TODO：

- [ ] 为地址类型建立 newtype。
- [ ] 为 handle 权限建立 bitflags。
- [x] 为首批 bounded byte syscall 建立安全 copy-in/copy-out：PAN-on/UAO-off、只读精确 PC exception table、严格 ESR/FAR/ASID/range 校验、失败消息不 push/pop。
- [ ] 为用户指针建立 `UserPtr<T>`。
- [ ] 为中断上下文和线程上下文建立不同 API。

## 第 25 章：AndroidBox 深度实施计划

### 25.1 三层兼容策略

AndroidBox 分为三层：

第一层是 ABI 兼容层，让 Bionic 和 native library 能获得必要系统调用语义。第二层是 Binder 和 Framework 兼容层，让 Java/Kotlin 应用能找到 Android 服务。第三层是系统服务桥接层，把 Android 调用映射到 Bndroid 的 AppManager、WindowServer、PackageManager、PermissionManager 等服务。

TODO：

- [ ] ABI 层先支持 pthread、mmap、futex、epoll、socket。
- [ ] Binder 层支持 Android service manager、transaction、reply、death recipient（M9 历史里程碑的 BSM1 不代表 Binder 兼容完成）。
- [ ] Framework 层先支持 Activity、View、Handler、Looper、Intent。
- [ ] Bridge 层先支持 window、input、network、storage。
- [ ] 每补一个 Android API，都加入兼容性测试数据库。

### 25.2 首个通用 ART/Framework APK 验收应用

这里验收的是尚未完成的通用 ART/ActivityThread/Framework 闭环，不是严格受限的
Install-0 双证据样本、单 fixture Update-0 与 boot-time Uninstall-0。首个目标 APK
应选择依赖最少、
功能最简单、无 GMS 依赖、
无复杂 native library 的应用。

验收流程：

- [ ] 由通用 Installer/PackageManager 安装成功。
- [ ] Manifest 解析成功。
- [ ] PackageManager 注册 Activity。
- [ ] ART 启动应用 main。
- [ ] ActivityThread 建立。
- [ ] Window 创建。
- [ ] Surface 分配。
- [ ] View 首帧绘制。
- [ ] 点击事件可到达应用。
- [ ] 应用日志进入 LogServer。
- [ ] 退出时资源释放。

## 第 26 章：开发节奏和验收制度

Bndroid OS 是长期系统工程，更好的管理方式是每个阶段都可启动、可运行、可演示、可回归。每个里程碑必须有明确验收项。

每周固定产物：

- [ ] 一份运行日志。
- [ ] 一份新增功能清单。
- [ ] 一份失败项记录。
- [ ] 一份下周任务表。
- [ ] 一次 QEMU 镜像构建。
- [ ] 一次最小回归测试。

每月固定产物：

- [ ] 月度系统镜像。
- [ ] 月度架构变更记录。
- [ ] 月度兼容性报告。
- [ ] 月度性能报告。
- [ ] 月度安全审查。
- [ ] 月度路线图调整。
