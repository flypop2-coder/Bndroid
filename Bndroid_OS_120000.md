# Bndroid OS 120000 字级完整工程蓝图

> 本文件保留详细设计背景与历史里程碑，不再是未来任务的权威来源。精炼后的模块
> 边界、合并/复用/删除规则和按依赖排序的产品待办统一见
> [`TODO.md`](TODO.md)。若两者存在未来计划冲突，以 `TODO.md` 为准；已经验证的
> 事实仍以 `IMPLEMENTATION_STATUS.md` 和对应里程碑证据为准。

> ABI 68 最新 AndroidBox 覆盖（2026-07-31）：
> `androidbox-layout-size18` 已把两份真实 SDK/AAPT2/D8 APK 的正整数
> `layout_width=Ndp` / `layout_height=Ndp` 从二进制 XML 贯通到严格解析器、
> 独立 AndroidApp、`BNDAPC14` v14/v5 24-byte descriptor、720×1600 光栅与
> 精确点击命中。fixture 使用 title width `240dp`、row height `120dp`、
> Button height `64/56dp`，physical rectangle 为
> `84/618/270/128` 与 `374/626/270/112`。三启动、worker 重绑、完整 scene
> 重放、双回调、双包共存和无 source 零写恢复以
> `ANDROIDBOX_LAYOUT_SIZE18_QEMU_OK` 通过，证据在
> `target/layout-size18/coexist.SF5cRf/`；ABI67/ABI66 父门以最终源码重新通过
> 于 `target/layout-directional17/coexist.xi7Dnb/` 与
> `target/layout-spacing16/coexist.v9KMXK/`。本轮没有下载 AOSP，也没有操作
> 持续运行的 ABI48 预览。当前阶段不需要 AOSP；完整 ART/Framework 或 Android
> runtime/container 必须另行评估并先取得明确授权。完整边界见
> `ANDROIDBOX_LAYOUT_SIZE_18.md`。
>
> ABI 67 最新 AndroidBox 覆盖（2026-07-31）：
> `androidbox-layout-directional17` 已把两份真实 SDK/AAPT2/D8 APK 的八个
> left/top/right/bottom padding/margin 属性从二进制 XML 贯通到严格解析器、
> 独立 AndroidApp、`BNDAPC13` v13/v4 24-byte descriptor、720×1600 光栅和
> 精确点击命中。fixture row padding 为 `6/4/2/8dp`，两个 Button margin 为
> `2/1/4/3dp` 与 `6/5/2/1dp`，physical rectangle 为
> `84/650/270/168` 与 `374/658/270/164`。三启动、worker 重绑、完整 scene
> 重放、双回调、双包共存与无 source 零写恢复以
> `ANDROIDBOX_LAYOUT_DIRECTIONAL17_QEMU_OK` 通过，证据在
> `target/layout-directional17/coexist.3bHWU6/`；ABI66 父门以最终源码重新
> 通过于 `target/layout-spacing16/coexist.Mb4eRn/`。本轮没有下载 AOSP，也
> 没有操作持续运行的 ABI48 预览。当前阶段不需要 AOSP；完整 ART/Framework
> 或 Android runtime/container 必须另行评估并先取得明确授权。完整边界见
> `ANDROIDBOX_LAYOUT_DIRECTIONAL_17.md`。
>
> ABI 66 最新 AndroidBox 覆盖（2026-07-31）：
> `androidbox-layout-spacing16` 已把两份真实 SDK/AAPT2/D8 APK 的 uniform
> `padding=4dp`、`layout_margin=2dp` 从二进制 XML 贯通到严格解析器、独立
> AndroidApp、`BNDAPC12` v12/v3 descriptor、720×1600 光栅和精确点击命中。
> 三启动、worker 重绑、完整 scene 重放、双回调、双包共存与无 source 零写恢复
> 以 `ANDROIDBOX_LAYOUT_SPACING16_QEMU_OK` 通过，证据在
> `target/layout-spacing16/coexist.Mb4eRn/`；ABI65 父门重新通过于
> `target/layout-weight15/coexist.lHJezZ/`。本轮没有下载 AOSP，也没有操作
> 持续运行的 ABI48 预览。当前阶段不需要 AOSP；完整 ART/Framework 或 Android
> runtime/container 必须另行评估并先取得明确授权。完整边界见
> `ANDROIDBOX_LAYOUT_SPACING_16.md`。
>
> ABI 65 最新 AndroidBox 覆盖（2026-07-31）：
> `androidbox-layout-weight15` 已把真实 AAPT2 二进制 XML 的
> `layout_width=0dp`、`layout_weight=1` 从两份 SDK/D8 APK 贯通到解析器、
> 独立 AndroidApp、`BNDAPC11` v11/v2 descriptor、720×1600 比例布局和点击
> 命中。三启动、worker 重绑、完整 scene 重放、双回调、双包共存与无 source
> 零写恢复以 `ANDROIDBOX_LAYOUT_WEIGHT15_QEMU_OK` 通过，证据在
> `target/layout-weight15/coexist.xC7DAf/`；ABI64 父门重新通过于
> `target/layout-row14/coexist.Jsu7Eq/`。本轮没有下载 AOSP，也没有操作持续
> 运行的 ABI48 预览。它仍不是 ART、完整 Framework、一般 APK 兼容或真机证明，
> 完整边界见 `ANDROIDBOX_LAYOUT_WEIGHT_15.md`。
>
> ABI 64 最新 AndroidBox 覆盖（2026-07-31）：
> `androidbox-layout-row14` 已让两份真实 SDK/D8 APK 从 AAPT2 二进制 XML
> 解析一个嵌套横向 `LinearLayout`，通过 `BNDAPC10` v10 传输六节点 scene，
> 并在 720×1600、20:9 UI 中并排绘制和点击两个按钮。三启动、worker
> guard-page fault 后 generation 重绑定、完整 scene 重放、双回调、关闭、无 source
> 恢复和双包共存门以 `ANDROIDBOX_LAYOUT_ROW14_QEMU_OK` 通过，证据在
> `target/layout-row14/coexist.ozW81O/`。ABI63 父门也重新通过于
> `target/string-builder13/coexist.IsZM9k/`。本轮没有下载 AOSP；它仍不是
> ART、完整 Android Framework、一般 APK 兼容或真机证明，完整边界见
> `ANDROIDBOX_LAYOUT_ROW_14.md`。
>
> ABI 47 最新覆盖（2026-07-30）：隔离的 `androidbox-process0` 已把固定
> InteractiveActivity-1 的 APK VMO、解释器和会话移入独立 `AndroidApp` EL0
> image；App 仅保留可信输入与光栅。两者通过私有 `BNDAPC01` Channel 完成
> Open→Click→Close，AndroidApp 稳态仅有一个 `READ|WRITE|WAIT` Channel，无
> graphics/input/storage/duplicate/transfer。离线双 boot 门
> `scripts/check-androidbox-process0.sh` 已以 `ANDROIDBOX_PROCESS0_QEMU_OK`
> 通过，证据在 `target/androidbox-process0/check.sFpefj/`。这只证明一个固定
> 受限 APK 子集的独立 Bndroid 进程，不是 ART、通用 Android、崩溃恢复或真机。
>
> 并行 UI/AndroidBox Resources-1 状态（2026-07-30）：隔离的 `mobile-ui-runtime` 输出精确
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
> Manifest-driven component admission。无源恢复 boot 中，同一已安装图标又通过 QEMU
> 触控连续启动两次；request sequence 1/2 均重新 readback、复验并执行，运行时各为
> `reads=389 writes=0 flushes=0`，整盘 SHA-256 不变，显示
> `Hello from a Mac-built APK`。这不是任意 APK 或一般 Android 兼容。
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
> 该隔离 profile 使用 ABI 44。syscall 59 是 boot catalog read，向 EL0 复制
> canonical 640-byte `BNDAPS01`，不暴露 APK bytes、handle 或 package-store 权限。
> Launcher-only syscall 60 接受 canonical 640-byte `BNDARQ01`，把 monotonic request
> sequence 与 exact generation/version/length/profile、APK/signer digest、package 和
> Activity 绑定；提交后 `ShouldWait`，Launcher 逐字节 exact retry。IRQ-enabled service
> 每次重新 `recover`/`read_blob`，复核 APK SHA-256、APK v2 signer、Manifest、
> Resources-1，并执行 constructor(2)→`onCreate`(4)；成功才返回 fresh `BNDAPS01`。
> 该运行时路径结构上只读。Settings 显示 boot catalog；All Apps 保留 generation 到
> release，pending/failed 不显示旧 Activity，fresh response 完全匹配后才导航。没有
> 运行时 install/update/uninstall UI 或通用 PackageManager API；Uninstall-0 只有
> boot-time host request，不增加 mutation syscall。旧 kernel 不理解 `BNDPRM01`，
> 因此该磁盘没有 downgrade-safe 声明。
>
> 点击执行仍是 kernel EL1 纯数据 Resources-1，不是独立 Android App 进程。
> `CompatibleActivitySession-0` 已完成 capacity-one、boot-local、identity-only
> Home/Back/Overview lifecycle/recent 语义：Home 不授予后台 task，Overview 不保存
> Activity pixels/thumbnail/live preview，recent activation 重新执行 syscall 60，
> Back 清除；`BUC1` v8/`BUE1` v6 的 Reserve/Commit/Abort 与 `BUP1` v2
> System UI revision binding 均 fail closed。边界仍为
> `art=0 dalvik=0 activitythread=0 binder=0 bionic=0 jni=0 native_lib=0
> permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled
> standalone_android_process=0 real_phone_claim=0`。它不是任意 APK installer、
> 一般 Android 兼容、网络 App 运行或真机
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

本文档是 Bndroid OS 的 12 万字级工程蓝图。它采用正向执行风格，重点描述能做什么、什么路线更好、如何落地、如何验证。系统主语言选择 Rust，主内核采用非 Linux 路线；长期目标由 AndroidBox 承接 Android 应用，当前已有不计入一般兼容性的 DEX-0、Activity-0 回归与 Resources-1 precursor，完成了严格受限 Install-0/Update-0、12-boot boot-time Uninstall-0/reinstall，以及 ABI-44 generation-bound read-only durable relaunch；通用包管理、独立 Android App 进程与一般 Android 兼容仍未完成。

> 执行状态（2026-07-27）：本文是长期蓝图，不是完成度清单；当前事实以 `IMPLEMENTATION_STATUS.md` 为准。历史 M71 里程碑为 ABI-v32/M71 `unified-product-continuous-supervision-runtime`。它严格扩展匹配的 M70 kernel/userspace closure，字面 kernel chain 为 M71→M70 `unified-product-psci-shutdown-runtime`→M69 `unified-product-multiservice-liveness-runtime`→M68 `unified-product-liveness-runtime`→M67 `unified-product-runtime`→M66 `resident-platform-shutdown-runtime`→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55。M71 不新增 syscall、镜像或 capability right，syscall 上限仍为 54；ABI v32 只更新证据契约。M71 保留历史 M70 的 strict FDT/PSCI 1.1、无 semihosting QEMU `SYSTEM_OFF`，并新增事务式五服务目录（ServiceManager、SurfaceServer、InputServer、StorageServer、App）、4 条依赖（3 hard、1 soft）、事务式批量 probe、每服务 missed-probe tolerance=1、16 轮额外健康 soak，以及 SurfaceServer/InputServer 同轮各漏一次后独立恢复；StorageServer 连续漏报仍走历史真实 100 ms timeout、30 ms backoff、同槽下一代 replacement 与 App block/resume。每次启动精确为 21 轮、107 probes、104 Healthy、3 missed，20 轮全健康、18 个 batch/90 个 batched probes、2 个瞬态恢复和 1 个升级故障；两次同盘启动都通过真实 UI/AppData closure 与 PSCI self-exit。它仍是 bounded、故障注入、single-core QEMU 研究原型，`arbitrary_soak_claim=0 emulator_only=1 general_runtime=0 real_phone_claim=0`，不是 PMIC、硬件 poweroff 或真机证明。feature-off/default 仍为 ABI v23，历史 M55—M64 为 ABI v25；M65/M66/M67/M68/M69/M70/M71 分别为 ABI v26/v27/v28/v29/v30/v31/v32。M33—M71 为三十九个 opt-in leaf，加 default M32 为四十份独立账本；历史 M59 双账继续隔离。

> 历史 M56 是 kernel-only fail-stop recovery：`OutcomeUnknown`/`RequiresReset` 都是 session-fatal。旧 StorageServer 退出、session/capability 清理和 volume unbind 完成后，kernel 才执行 virtio status 0 reset，复核 device identity、相同 features/capacity，以仍独占的 DMA pages 重建 queue，并用 prepare/commit 两阶段重新 arm IRQ。build wrapper 现闭合 M66→M65→M64→M63→M62→M61→M60→M58→M57→M56→M55 kernel chain；M65/M66 要求匹配的 kernel/userspace feature，M63/M64 userspace 继续被拒绝，AppData/StorageServer/timeout profile 仍互斥。

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

> 历史 M60 在 M58 cooperative StorageServer 上增加 ticketed `RecoveryPolicy`。一次 `WRFWRFR` campaign 包含六个历史瞬态 W/R/F 控制和一个模拟永久 read fault；attempt cap=3，退避基数为 2 ticks、倍率 2，恢复成功先进入 Probation，达到健康窗后才回到 Healthy，失败上限后进入本次 boot 内粘滞 Offline。永久 fault 不是 EL0 授权：六个瞬态 EL0 控制仍保留，但 epoch 7 的 EL0 只选择普通 read workload；kernel 在 ownerless 提交窗中 prearm，因此 `el0_permanent_fault_arm_controls=0`、`permanent_authority=kernel-prearmed`。StorageConnect 先完成 principal/image/PID/rights 鉴权，再报告 recovery/Offline 状态。

> 动态 campaign 得到 recovery attempts/commits/failures/rollbacks=`9/6/3/1`、physical successes/failures=`7/2`、policy attempt failures=`4`（含一次 Probation I/O failure）、三次完成的 backoff 共 8 ticks、Probation/Healthy=`7/5/2` 与 5 次 Healthy transition。终态路径在进入 DAIF-masked 窗前已关闭 admission；masked 窗内完成 IRQ rollback，复核 admission 仍关闭、coordinator inactive、driver status=0/in-flight=0 与 terminal DMA ownership，随后发布 broker Offline。历史 M60 动态运行走 direct terminal proof；fallback cooperative quarantine 只有静态封印，计数为 `0/0/0`。Offline 后 attempt/reset/submission delta 均为 0，下一 owner epoch 为 8。精确实测为：

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

> 历史 M65 使用 ABI v26/syscall 53 实现 init-only `Prepare/Commit`。两个固定 workload client 完成并被回收、StorageServer idle、broker/物理 I/O terminal 后，Prepare 关闭新进程与 StorageAcquire/Connect/Accept admission；client 权限探针与 Prepare 后 spawn 均被拒绝。StorageServer 随后 final flush、同 generation recover/readback、ACK、正常退出并被 init 回收；精确 InitReady 后 Commit。kernel monitor 重验全部账本，调用 M64 durable close，封闭 storage/IRQ/shutdown gate 后 halt。

> 同一 writable image 双启动证明 AppData generation 5→6、health `1→2`/`3→4` clean close、两次 server flush/readback/exit，DATA/APPDATA 外字节不变。当前 log 为 91 行、18869 字节，SHA-256 `2c338c42a97a869375ccca50ec33cd8d84556b71cbcf7f56bffe8c44be58c645`：

```text
STORAGE_SERVER_SHUTDOWN_ORCHESTRATION_REBOOT_OK boots=2 userspace_shutdowns=2 clients_drained=4 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 offline_persisted=0 offline_from_record=0 el0_started=1 el0_controls=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M65 userspace StorageServer shutdown orchestration and durable close verified
```

> 这仍是 single-core storage-profile directed proof，没有完整 resident UI/service graph、PSCI/硬件 poweroff、真实断电、SMP 或 general runtime。

> 历史 M66 在 M65 上加入 ABI v27/syscall 54 `ServiceShutdown`。init 同时启动 StorageServer 与八个认证 resident 节点；Prepare 前 kernel 核验 10 个 live process、9 对 init control、10 对 dependency Channel、38 个 endpoint、39 个 handle、rights/空队列、唯一 StorageVolume 与八个 PID/image/node 绑定。提前 SurfaceServer quiesce 必须被拒，随后按 `Primary/Secondary/Launcher/App→Provider/InputServer→ServiceManager/SurfaceServer` 三波逆拓扑关闭；Launcher/App 还执行真实 AppData workload。

> 九个 child 全部 exit/reap、StorageServer final flush/readback、M65 Commit、M64 durable close、storage/shutdown seal、block IRQ disable 与本地 IRQ mask 完成后，kernel 才创建 opaque `ValidatedShutdown` token。M66 唯一 backend 是 AArch64 QEMU semihosting `SYS_EXIT_EXTENDED`；同一 writable image 两次启动都必须由 QEMU 自身 status 0 退出，host kill 不算成功。该历史 89 行/19200 字节日志 SHA-256 为 `656186fb9e4275bed2f64d95484160e97cdbe7960a74f90e209f566b7aff81de`：

```text
RESIDENT_PLATFORM_SHUTDOWN_REBOOT_OK boots=2 qemu_self_exits=2 emulator_poweroffs=2 resident_shutdowns=2 resident_nodes=8 dependency_edges=10 quiesce_waves=3 registrations=16 quiesces=16 order_rejections=2 storage_server_flushes=2 storage_server_readbacks=2 storage_server_exits=2 prepare_calls=8 prepares=2 commit_calls=2 commits=2 spawn_rejections=2 connect_rejections=4 final_appdata_generation=6 final_health_generation=4 final_health_slot=0 prior_health_generation=3 prior_health_slot=1 final_boot_open=0 prior_boot_open=1 qemu_backend_persistence=1 outside_data_appdata_unchanged=1 unused_data_unchanged=1 appdata_changed=1 changed_data_bytes=153 changed_appdata_bytes=2755 emulator_only=1 full_userspace_shutdown_claim=0 hardware_poweroff_claim=0 psci_claim=0 powercut_claim=0 smp_claim=0 general_runtime=0
BOOT_OK: M66 complete resident graph quiesced and QEMU platform exit armed
```

> 这只是 fixed-graph、single-core、emulator-only shutdown proof；不是完整产品 UI runtime，也不是 PSCI/PMIC/硬件 poweroff，不证明真实掉电、SMP、general runtime 或真机。

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

> 历史 M71 阶段的 83 个顶层 shell scripts 内共有 52/52 个 QEMU launch，均恰有一个 `-nic none`。M67—M70 的隔离 source/feature/parser/双启动 gate 当时全部保留；M71 新增 continuous-supervision source/feature/parser/双启动 gate。该阶段 suite 包含 `unified_product_continuous_supervision_static=1 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6`；M70/M71 的 PSCI 双启动合计 `qemu_psci_self_exits=4`，M66—M71 六份双启动 self-exit 账本合计 `qemu_self_exits=12`。

> 2026-07-27 历史 M71 当时最终源码上的完整离线 `CARGO_NET_OFFLINE=true ./scripts/test.sh` 已 exit 0，精确终态为：

```text
BNDROID_TEST_SUITE_OK qemu_boot_profiles=5 framebuffer=1 keyboard_qmp=1 tablet_touch_qmp=1 compositor_interaction=1 userspace_surface=1 userspace_ui=1 ui_screenshot=1 process_terminate=1 app_lifecycle=1 app_crash_recovery=1 mapped_graphics=1 graphics_owner_death=1 graphics_surface_restart=1 graphics_producer_orphan=1 graphics_frame_clock=1 graphics_swapchain=1 multi_window=1 persistent_window=1 text_input=1 soft_keyboard=1 input_server=1 input_server_surface_restart=1 input_server_restart=1 service_supervisor=1 service_dependency=1 post_recovery_interaction=1 post_recovery_focus=1 post_recovery_focus_roundtrip=1 post_recovery_lifecycle_focus=1 app_data_runtime=1 app_data_async_recovery=1 storage_server_static=1 storage_server_recovery_static=1 storage_server_repeated_recovery_static=1 storage_server_async_recovery_static=1 storage_server_fault_policy_static=1 storage_server_owner_liveness_static=1 storage_server_terminal_quarantine_static=1 storage_server_persistent_health_static=1 storage_server_clean_shutdown_static=1 storage_server_shutdown_orchestration_static=1 resident_platform_shutdown_static=1 unified_product_static=1 unified_product_liveness_static=1 unified_product_multiservice_liveness_static=1 unified_product_psci_shutdown_static=1 unified_product_continuous_supervision_static=1 storage_recovery_unification_static=1 storage_server_runtime_boots=3 storage_server_recovery=1 storage_server_repeated_recovery=1 storage_server_async_recovery=1 storage_server_fault_policy=1 storage_server_owner_liveness=1 storage_server_terminal_quarantine=1 storage_server_persistent_health_reboot=1 persistent_health_boots=2 storage_server_clean_shutdown_reboot=1 clean_shutdown_boots=2 storage_server_shutdown_orchestration_reboot=1 shutdown_orchestration_boots=2 resident_platform_shutdown_reboot=1 resident_platform_shutdown_boots=2 unified_product_reboot=1 unified_product_boots=2 unified_product_ui_interactions=2 unified_product_liveness_reboot=1 unified_product_liveness_boots=2 unified_product_liveness_recoveries=2 unified_product_multiservice_liveness_reboot=1 unified_product_multiservice_liveness_boots=2 unified_product_multiservice_liveness_recoveries=2 unified_product_psci_shutdown_reboot=1 unified_product_psci_shutdown_boots=2 unified_product_continuous_supervision_reboot=1 unified_product_continuous_supervision_boots=2 unified_product_continuous_supervision_recoveries=2 continuous_supervised_services=10 continuous_health_probes=214 continuous_healthy=208 continuous_missed=6 qemu_psci_self_exits=4 qemu_self_exits=12 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1 storage_irq_cooperative_recovery=1
```

> 同一次完整回归还允许 M51 `GraphPrepared` 后 Launcher/App 两个已认证 rebind ACK 以任一合法串行顺序到达；每角色 exact-once、PID/image/token 与 payload 校验没有放宽。

> 历史 M55 authority split 保持封印：canonical StorageServer 独占 `StorageAcquire`，volume/session 是 `READ|WRITE|WAIT` 且无 `DUPLICATE|TRANSFER`，client 只可请求 READ/WRITE 非空子集；kernel broker 只管理 raw-sector owner/epoch/session/token/bounds/completion/cleanup，EL0 StorageServer 执行 path namespace 与 AppData COW volume policy。精确边界为 `kernel_namespace_ops=0 namespace_policy=0 block_driver=kernel`。

> raw block request 是 strict `SBRQ` v2、exact 4160 bytes（64-byte header + 4096 data），offset 24 为 little-endian `u16 sector_count`。read/write count 1—8，所有 reserved、read data、write unused tail 必须为零；flush 只能是 LBA/count/data 全零，并以 checked add 保证 `lba + count <= 1920`。server read-ahead/cache 与 contiguous-write coalescing 上限均为 batch 8；batch 不是原子 multi-sector transaction，底层仍逐扇区执行。

> 历史 M55 logical timer 保持 100 Hz；storage completion 只把 physical compare 临时拉近为 one-shot immediate IRQ，并在正常 exception-return boundary 给 newly-woken context 一次 preference。真实 timer IRQ、TrapFrame、TTBR/translation 与 stack switch 已覆盖（`trapframe_switch=1`）。精确账式是 `completion_scans=candidate_scans+no_candidate_scans`、`candidate_scans=preferred_requests+preferred_coalesced`、`preferred_requests=preferred_dispatches+stale+preferred_pending`；ready 时 `stale=0 preferred_pending=0`，并满足 `0<immediate_interrupts<=immediate_requests<=I/O completions`。completion 可能早于 client 进入 `StorageTake` 阻塞态，此时没有 candidate waiter；旧的 `preferred_requests==all I/O completions` 是错误不变量。不能把 one-shot 写成提高 logical tick。M55 的 idle restart 不等于 M56 的 session-fatal in-flight reset。

> 历史 M55 三次同镜像启动形成 AppData generation `0→4→5→5`。boot 1 的 read batches/write batches/flushes/completions=`1425/289/12/1726`，boot 2=`765/45/2/812`，boot 3=`606/0/0/606`；对应 read/write sectors 为 `11400/2121`、`6120/290`、`4848/0`，三次 `max_batch_sectors=8 errors=0 bounds_rejections=0`。boot 3 零 AppData write/flush；boot 2/3 AppData partition SHA-256 同为 `d105d3eecbeee5e77774c1d37f83e11406e50e020d6abd48c0b4a56f9980b089`。whole disk 每次因 `BNDROID_DATA` boot counter 变化，稳定证明范围只能是 `BNDROID_APPDATA` partition。

> M55—M70 与 M59 双账本是历史前缀，历史 M71 由隔离 checker 封口。M70 继承固定双服务 dependency-liveness，并把最终 QEMU exit 绑定到 strict FDT `/psci`、PSCI_VERSION 1.1 与 HVC SYSTEM_OFF；M71 再加入事务式五服务/四边目录、批量 probe、16 个额外健康轮、同窗两服务瞬态漏报恢复和一次升级 StorageServer replacement。它仍是 bounded single-core QEMU `arbitrary_soak_claim=0 general_runtime=0 real_phone_claim=0` research prototype，不是真手机、不可刷机、不可日用，也不证明 PMIC/hardware poweroff。下一硬件 P0 必须先由用户指定并授权目标，再做 BSP、启动链、真实控制器/PMIC 与真机电源路径；目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak 仍待推进。

> 紧随其后的 `BNDROID_TEST_SUITE_OK` 整行是历史 M55 完整套件的唯一终态 seal；M56—M66 与 M59 各自证据不能彼此替代。

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

> `bndr-appdata` 的 22 项历史 host tests 覆盖正常操作、292 个 mutation 切点和 965 个首次格式化切点。该段只描述 M54 kernel-monitor service；M55—M69 历史 StorageServer/shutdown/unified-product/liveness 与历史 M70 QEMU PSCI 证据见文首，不能回写成 M54 架构。它们都不是 POSIX 或产品文件系统，也都缺真实 UFS/eMMC/NVMe/手机控制器与真机断电恢复；`general_runtime=0`。

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

> Present protocol 为 v2/64-byte。Launcher/App frame 携带 focus generation；server 只提交当前焦点且 generation 匹配的 client frame，并把两条本地连续 frame 序列映射成一条全局连续 commit 序列。后台或 stale-generation present 返回 `PresentCancelled`，不推进序列并允许同一 frame id 重试。默认关闭的 `ui-stale-present-evidence` 提供确定性 directed QEMU 证据：24 input/13 commit baseline 后，frame 10 在 SurfaceServer 已读取 generation 8 时挂起，Home generation 9 取消它，generation 10 以同一 local frame id 重试，最终 Home generation 11 收敛为 36 input/18 个连续 commit；无需尝试多个调度相位。输入在 pointer down 时选择 Home→Launcher 或当前 App 并 capture 到 release；QEMU trace 也证明 Settings 内按下、拖到 Home 区再抬起的 App 本地 sequence `1..3` 全部只到 App，Launcher 无泄漏且不触发 focus/frame commit。Launcher 独立绘制 Home，App 独立绘制 Phone/Messages/Settings。

> M32 增加两槽、page-aligned 的静态 XRGB8888 GraphicsBuffer pool：每槽固定 `208×368`、logical/backing `306176/307200` bytes。App 以 `READ|WRITE|DUPLICATE|TRANSFER=0x0f` 创建，用 syscall 29 进行非空、4-byte 对齐且单次最多 4096-byte 的规范化写入，再把 `READ|TRANSFER=0x09` 衰减副本随首个 64-byte `BUP1` `BufferPresent` 原子转移给 SurfaceServer。M32 历史 wire 为 v1；当前 v2 保留 Full geometry 和 client/global frame id、focus generation、buffer generation，并新增 byte 40..48 的 System UI revision；mobile frame 必须绑定当前非零 revision。最后一个引用释放时完整擦除 backing 并推进 slot generation。

> M33 专用 profile 在 ABI v18 上固定 canonical 64-byte v1 `ALC1` app lifecycle、`UBP1` startup endpoint bootstrap 与 `USC1` Init↔SurfaceServer supervisor control。Init 驱动 `Launch1→Activate1→Suspend1→Resume1→Terminate1→Launch2→Activate2` 七个严格事务；kernel committed-write trace 接受 ALC1 35 条、USC1 14 条消息，USC1 Install/Activate/ShowLauncher/Retire command=`2/3/1/1`，decode/authentication/tracker error 全为 0。

> M33 QEMU 收敛时 App1 在 terminate ack 与 Surface 退休旧 endpoint/buffer/capture 后正常 Exited，并由 `ProcessWait` 回收；App2 复用同一 PID slot、PID generation 严格加一，最终 active/resident。created/exited/reaped/live=`10/2/2/8`，最终 29 handles、26 endpoint/13 pair；M20 core 8 pair 之外，Init↔Surface supervisor、Init↔Launcher lifecycle、Init↔App lifecycle、Surface↔Launcher UI、Surface↔App UI 构成五对常驻窗口通道。最终等待账本为 8 项 object token，wait-many 2、wait-array 6。

> M34 专用 profile 把脚本扩展为十个严格事务：在 M33 七步之后，Init 以 ABI 18 `ProcessTerminate` 杀死正阻塞于 `WaitArray` 的 App2；SurfaceServer 从 UI peer-close 原子撤销旧 App endpoint/buffer、焦点、input capture 与 pointer state，切回 Launcher，并向 Init 发送 generation-qualified USC1 `OwnerDied`。Init 接受该自发事务后发布 ALC1 `Crashed/ProcessExited`，再启动并激活 App3。App PID 在同一 slot 上严格 `generation 1→2→3`，最终 App3 active/resident，GraphicsBuffer generation 为 3。专用 QEMU 精确得到 ALC message/transaction/state/crash=`46/10/19/1`，USC command/ack/owner-death=`9/9/1`，USC transaction=`10`，operation Install/Activate/ShowLauncher/Retire=`3/4/1/1`；process created/exited/reaped/live=`11/3/3/8`，exit reasons Exited/Faulted/Killed=`2/0/1`，terminate accepted/completed=`1/1`。最终仍为 29 handle、26 endpoint/13 pair 和 8 个 object wait token（many/array=`2/6`），终止遗弃 object/many/array token=`0/0/1`。

> buffer decode、producer/rights、focus/sequence、generation、全部像素和 counter 在首像素前验证，失败/取消不改变 scene、序列与计数。启动期以一次 4-byte seed 和 pre-focus cancelled present 验证句柄握手，所以启动账本 `created/write_calls/write_bytes/presents=1/1/4/0`；交互路径由 App raster 76544 个真实像素并由 SurfaceServer copy-present。early per-page W^X window 为此从 RAM 前 4 MiB 扩为 6 MiB，新增 L3 覆盖但不放松权限。这是有界 object transfer+copy，不是 `mmap`、共享 EL0 mapping 或零拷贝。

> M22 的无分配、两遍 FDT API 以固定容量 32 收集 enabled、direct-root `virtio,mmio` 节点，并解析根或节点级 `interrupt-parent`、GICv2 controller phandle、`#interrupt-cells = 3` 与 SPI specifier。当前 QEMU 树恰有 32 个 coherent transport，其中 1 个 active block device；其 raw specifier `[0,47,1]` 被严格解释为 edge-rising SPI INTID 79。GICv2 在开放本地 IRQ 前完成 disable/configure/target CPU0/priority/clear/enable。M25 正常设备必须物理可写，严格要求 `VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_FLUSH` 并拒绝 RO；独立的 M22 race build 仍使用物理只读设备。两者都拒绝 legacy MMIO 与 non-coherent DMA。

> queue 0 固定为 8 项 split ring，216-byte ring 独占一个物理 frame；第二个 frame 以 536-byte stride 打包两个 header/data/status request slot，总占用 1072 bytes，descriptor head 为 0/3。请求追踪器给每个槽分配 generation-qualified token，支持两个同时 outstanding、乱序 used completion、重复/未知/过期 completion 拒绝。M25 在同一所有权模型内加入 type 1 WRITE 与 type 4 FLUSH，二者都严格验证 `used.len=1`，且只有 WRITE 的 IRQ completion 完成后才发布 FLUSH。正常前台只发布请求并等待 IRQ 侧递增的完成代数，不轮询 used ring；IRQ 顺序为读取并 ACK virtio source、drain used entries、更新完成状态，再由公共 dispatcher EOI GIC。单一静态 storage owner 永久持有 driver 和两帧，并通过屏蔽本地 IRQ 的短临界区保证单核所有权。

> M23 在同一 IRQ-only owner 上增加 512-byte sector block abstraction、严格 GPT 与只读 FAT16/VFS；M23/M24 历史 fixture 的 SHA-256 为 `36f39e23da09401dc2f686217e2bb06d146a65cd14aa4b9b8b2f0fd4aa7b809b`。M24 只在系统文件大小和摘要全部验证后，才把固定 28+40=68 bytes 一次性复制到 immutable `BootfsCatalog<2>`/VMO；catalog 发布后的用户态 open/read 不再访问磁盘。M25 确定性镜像为 8 MiB/16384 sectors，SHA-256 改为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`：protective MBR、主 header LBA 1/entry array 2—33、备份 array 16351—16382/header 16383 均被严格验证；index 0 `BNDROID_SYS` 仍为 LBA 2048—16350 且只读，新 private index 1 `BNDROID_DATA` 为 LBA 64—127。DATA superblock 绑定其 GPT unique GUID 派生的 16-byte format epoch；两个 512-byte CRC record slot 若都有效，generation 必须相邻。提交顺序固定为 inactive-slot WRITE completion、FLUSH completion、重读双槽，旧已选槽始终保留；fresh boot 从 generation 0 提交到 1。下列 M25—M30 marker 只作为对应阶段历史证据；其中 ABI v14/v15 与旧五/七进程账本不能冒充 M31。

> M26 严格唯一发现 direct-root `qemu,fw-cfg-mmio`，验证 fw_cfg signature/features 并通过 big-endian DMA 配置 `etc/ramfb`。kernel-owned framebuffer 为 320×480 XRGB8888、stride 1280、614400 bytes/150 pages；九色 static splash 的 kernel digest 为 `0x6ef9c2b7d15fde25`。headless `check-framebuffer.sh` 经 QMP screendump 验证 153600 pixels、9 colors/9 samples，pixel SHA-256 为 `0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac`。M26 还绑定 coherent modern virtio-input keyboard：device ID 18、MMIO `0x0a003c00`、raw IRQ `0/46/1`/SPI 78、queue 8/one DMA frame，配置必须含 A+Enter；block 保持 `0x0a003e00`/SPI 79。QMP A down/up 精确形成 A-down/SYN/A-up/SYN，completions/delivered/recycled=`4/4/4`、used/avail=`4/12`，drop/invalid/config IRQ/spurious 均为 0。这些是保留的 M26 历史证据。

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

> M20 服务 transcript 与 M29—M31 旧账本只保留为历史基线。M32 default normal checker 继续固定其历史账本。经完整矩阵验证的 ABI-v20/M42 封口 default workspace 为 500 tests（`20/41/19/31/99/0/290`），加 M41 feature 20 与 M42 feature 3 项为 523 unique/313 unique kernel；hardened persistent-window QEMU 3/3、M41 regression、all-feature Clippy 与完整 `./scripts/test.sh` 均已从头通过，最终 marker 与上方 exact suite marker 相同。

> 产品范围仍是 `general_runtime=0 crash_consistency=0`。M34—M48 分别只证明固定 lifecycle/death/restart/orphan/frame-clock/swapchain/multi-window/persistent-window、输入与单服务监督场景；M49 已完成固定 SurfaceServer+InputServer、单条 soft dependency、degraded→recovered 与 restart-storm 防护 witness，后续 M50—M53 继续复用该固定两服务前缀。它仍不是任意服务依赖图或产品 supervisor。系统也没有硬件 vblank/pageflip、DMA-BUF/IOMMU、任意 App/window 管理、产品级 InputServer、完整 IME/candidate/locale/font shaping/Unicode、多点或真实硬件触屏；网络、电话/蜂窝、Wi-Fi、音频、电源管理、产品级驱动、安全启动、应用沙箱、安全更新与可用产品 UX 也均未完成，当前绝不是现实可用的手机系统。

> M32 ABI-v17、M33/M34 ABI-v18 与 M35—M41 作为历史与专项基线继续保留；ABI-v20/M42、ABI-v21/M43/M44、ABI-v22/M45/M46 及 ABI-v23/M47 的完整历史封口保持不变。ABI-v23/M48 的 dedicated QEMU、四截图、host、隔离回归、静态/Clippy matrix 与最终全量 `scripts/test.sh` 均已通过。这仍不是多服务 supervisor、完整 compositor、产品级 IME/InputServer 或手机产品。

## 第 1 章：项目总纲

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 1.1 愿景

项目总纲 - 愿景 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 愿景 的最小可运行目标。
- [ ] 实现 愿景 的 Rust 数据结构和错误模型。
- [ ] 接入 愿景 的日志输出和诊断字段。
- [ ] 为 愿景 编写单元测试或集成测试。
- [ ] 把 愿景 接入构建系统和 QEMU 镜像。
- [ ] 建立 愿景 的验收标准和失败回滚策略。
- [ ] 记录 愿景 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 愿景 增加性能指标和安全审计点。

#### 验收标准

- 愿景 可以独立构建。
- 愿景 可以在 QEMU 或测试环境中运行。
- 愿景 的错误路径有日志。
- 愿景 的权限检查可被测试覆盖。
- 愿景 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 1.2 能力目标

项目总纲 - 能力目标 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 能力目标 的最小可运行目标。
- [ ] 实现 能力目标 的 Rust 数据结构和错误模型。
- [ ] 接入 能力目标 的日志输出和诊断字段。
- [ ] 为 能力目标 编写单元测试或集成测试。
- [ ] 把 能力目标 接入构建系统和 QEMU 镜像。
- [ ] 建立 能力目标 的验收标准和失败回滚策略。
- [ ] 记录 能力目标 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 能力目标 增加性能指标和安全审计点。

#### 验收标准

- 能力目标 可以独立构建。
- 能力目标 可以在 QEMU 或测试环境中运行。
- 能力目标 的错误路径有日志。
- 能力目标 的权限检查可被测试覆盖。
- 能力目标 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 1.3 阶段目标

项目总纲 - 阶段目标 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 阶段目标 的最小可运行目标。
- [ ] 实现 阶段目标 的 Rust 数据结构和错误模型。
- [ ] 接入 阶段目标 的日志输出和诊断字段。
- [ ] 为 阶段目标 编写单元测试或集成测试。
- [ ] 把 阶段目标 接入构建系统和 QEMU 镜像。
- [ ] 建立 阶段目标 的验收标准和失败回滚策略。
- [ ] 记录 阶段目标 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 阶段目标 增加性能指标和安全审计点。

#### 验收标准

- 阶段目标 可以独立构建。
- 阶段目标 可以在 QEMU 或测试环境中运行。
- 阶段目标 的错误路径有日志。
- 阶段目标 的权限检查可被测试覆盖。
- 阶段目标 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 1.4 成功标准

项目总纲 - 成功标准 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 成功标准 的最小可运行目标。
- [ ] 实现 成功标准 的 Rust 数据结构和错误模型。
- [ ] 接入 成功标准 的日志输出和诊断字段。
- [ ] 为 成功标准 编写单元测试或集成测试。
- [ ] 把 成功标准 接入构建系统和 QEMU 镜像。
- [ ] 建立 成功标准 的验收标准和失败回滚策略。
- [ ] 记录 成功标准 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 成功标准 增加性能指标和安全审计点。

#### 验收标准

- 成功标准 可以独立构建。
- 成功标准 可以在 QEMU 或测试环境中运行。
- 成功标准 的错误路径有日志。
- 成功标准 的权限检查可被测试覆盖。
- 成功标准 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 1.5 产品边界

项目总纲 - 产品边界 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 产品边界 的最小可运行目标。
- [ ] 实现 产品边界 的 Rust 数据结构和错误模型。
- [ ] 接入 产品边界 的日志输出和诊断字段。
- [ ] 为 产品边界 编写单元测试或集成测试。
- [ ] 把 产品边界 接入构建系统和 QEMU 镜像。
- [ ] 建立 产品边界 的验收标准和失败回滚策略。
- [ ] 记录 产品边界 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 产品边界 增加性能指标和安全审计点。

#### 验收标准

- 产品边界 可以独立构建。
- 产品边界 可以在 QEMU 或测试环境中运行。
- 产品边界 的错误路径有日志。
- 产品边界 的权限检查可被测试覆盖。
- 产品边界 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 1.6 工程原则

项目总纲 - 工程原则 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 工程原则 的最小可运行目标。
- [ ] 实现 工程原则 的 Rust 数据结构和错误模型。
- [ ] 接入 工程原则 的日志输出和诊断字段。
- [ ] 为 工程原则 编写单元测试或集成测试。
- [ ] 把 工程原则 接入构建系统和 QEMU 镜像。
- [ ] 建立 工程原则 的验收标准和失败回滚策略。
- [ ] 记录 工程原则 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 工程原则 增加性能指标和安全审计点。

#### 验收标准

- 工程原则 可以独立构建。
- 工程原则 可以在 QEMU 或测试环境中运行。
- 工程原则 的错误路径有日志。
- 工程原则 的权限检查可被测试覆盖。
- 工程原则 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 2 章：Rust-first 战略

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 2.1 no_std 内核

Rust-first 战略 - no_std 内核 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 no_std 内核 的最小可运行目标。
- [ ] 实现 no_std 内核 的 Rust 数据结构和错误模型。
- [ ] 接入 no_std 内核 的日志输出和诊断字段。
- [ ] 为 no_std 内核 编写单元测试或集成测试。
- [ ] 把 no_std 内核 接入构建系统和 QEMU 镜像。
- [ ] 建立 no_std 内核 的验收标准和失败回滚策略。
- [ ] 记录 no_std 内核 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 no_std 内核 增加性能指标和安全审计点。

#### 验收标准

- no_std 内核 可以独立构建。
- no_std 内核 可以在 QEMU 或测试环境中运行。
- no_std 内核 的错误路径有日志。
- no_std 内核 的权限检查可被测试覆盖。
- no_std 内核 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 2.2 用户态服务

Rust-first 战略 - 用户态服务 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 用户态服务 的最小可运行目标。
- [ ] 实现 用户态服务 的 Rust 数据结构和错误模型。
- [ ] 接入 用户态服务 的日志输出和诊断字段。
- [ ] 为 用户态服务 编写单元测试或集成测试。
- [ ] 把 用户态服务 接入构建系统和 QEMU 镜像。
- [ ] 建立 用户态服务 的验收标准和失败回滚策略。
- [ ] 记录 用户态服务 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 用户态服务 增加性能指标和安全审计点。

#### 验收标准

- 用户态服务 可以独立构建。
- 用户态服务 可以在 QEMU 或测试环境中运行。
- 用户态服务 的错误路径有日志。
- 用户态服务 的权限检查可被测试覆盖。
- 用户态服务 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 2.3 FFI 边界

Rust-first 战略 - FFI 边界 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 FFI 边界 的最小可运行目标。
- [ ] 实现 FFI 边界 的 Rust 数据结构和错误模型。
- [ ] 接入 FFI 边界 的日志输出和诊断字段。
- [ ] 为 FFI 边界 编写单元测试或集成测试。
- [ ] 把 FFI 边界 接入构建系统和 QEMU 镜像。
- [ ] 建立 FFI 边界 的验收标准和失败回滚策略。
- [ ] 记录 FFI 边界 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 FFI 边界 增加性能指标和安全审计点。

#### 验收标准

- FFI 边界 可以独立构建。
- FFI 边界 可以在 QEMU 或测试环境中运行。
- FFI 边界 的错误路径有日志。
- FFI 边界 的权限检查可被测试覆盖。
- FFI 边界 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 2.4 错误模型

Rust-first 战略 - 错误模型 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 错误模型 的最小可运行目标。
- [ ] 实现 错误模型 的 Rust 数据结构和错误模型。
- [ ] 接入 错误模型 的日志输出和诊断字段。
- [ ] 为 错误模型 编写单元测试或集成测试。
- [ ] 把 错误模型 接入构建系统和 QEMU 镜像。
- [ ] 建立 错误模型 的验收标准和失败回滚策略。
- [ ] 记录 错误模型 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 错误模型 增加性能指标和安全审计点。

#### 验收标准

- 错误模型 可以独立构建。
- 错误模型 可以在 QEMU 或测试环境中运行。
- 错误模型 的错误路径有日志。
- 错误模型 的权限检查可被测试覆盖。
- 错误模型 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 2.5 并发模型

Rust-first 战略 - 并发模型 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 并发模型 的最小可运行目标。
- [ ] 实现 并发模型 的 Rust 数据结构和错误模型。
- [ ] 接入 并发模型 的日志输出和诊断字段。
- [ ] 为 并发模型 编写单元测试或集成测试。
- [ ] 把 并发模型 接入构建系统和 QEMU 镜像。
- [ ] 建立 并发模型 的验收标准和失败回滚策略。
- [ ] 记录 并发模型 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 并发模型 增加性能指标和安全审计点。

#### 验收标准

- 并发模型 可以独立构建。
- 并发模型 可以在 QEMU 或测试环境中运行。
- 并发模型 的错误路径有日志。
- 并发模型 的权限检查可被测试覆盖。
- 并发模型 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 2.6 workspace 组织

Rust-first 战略 - workspace 组织 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 workspace 组织 的最小可运行目标。
- [ ] 实现 workspace 组织 的 Rust 数据结构和错误模型。
- [ ] 接入 workspace 组织 的日志输出和诊断字段。
- [ ] 为 workspace 组织 编写单元测试或集成测试。
- [ ] 把 workspace 组织 接入构建系统和 QEMU 镜像。
- [ ] 建立 workspace 组织 的验收标准和失败回滚策略。
- [ ] 记录 workspace 组织 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 workspace 组织 增加性能指标和安全审计点。

#### 验收标准

- workspace 组织 可以独立构建。
- workspace 组织 可以在 QEMU 或测试环境中运行。
- workspace 组织 的错误路径有日志。
- workspace 组织 的权限检查可被测试覆盖。
- workspace 组织 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 3 章：内核架构

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 3.1 AArch64 启动

内核架构 - AArch64 启动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 AArch64 启动 的最小可运行目标。
- [ ] 实现 AArch64 启动 的 Rust 数据结构和错误模型。
- [ ] 接入 AArch64 启动 的日志输出和诊断字段。
- [ ] 为 AArch64 启动 编写单元测试或集成测试。
- [ ] 把 AArch64 启动 接入构建系统和 QEMU 镜像。
- [ ] 建立 AArch64 启动 的验收标准和失败回滚策略。
- [ ] 记录 AArch64 启动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 AArch64 启动 增加性能指标和安全审计点。

#### 验收标准

- AArch64 启动 可以独立构建。
- AArch64 启动 可以在 QEMU 或测试环境中运行。
- AArch64 启动 的错误路径有日志。
- AArch64 启动 的权限检查可被测试覆盖。
- AArch64 启动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.2 内存管理

内核架构 - 内存管理 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 内存管理 的最小可运行目标。
- [ ] 实现 内存管理 的 Rust 数据结构和错误模型。
- [ ] 接入 内存管理 的日志输出和诊断字段。
- [ ] 为 内存管理 编写单元测试或集成测试。
- [ ] 把 内存管理 接入构建系统和 QEMU 镜像。
- [ ] 建立 内存管理 的验收标准和失败回滚策略。
- [ ] 记录 内存管理 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 内存管理 增加性能指标和安全审计点。

#### 验收标准

- 内存管理 可以独立构建。
- 内存管理 可以在 QEMU 或测试环境中运行。
- 内存管理 的错误路径有日志。
- 内存管理 的权限检查可被测试覆盖。
- 内存管理 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.3 调度器

内核架构 - 调度器 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 调度器 的最小可运行目标。
- [ ] 实现 调度器 的 Rust 数据结构和错误模型。
- [ ] 接入 调度器 的日志输出和诊断字段。
- [ ] 为 调度器 编写单元测试或集成测试。
- [ ] 把 调度器 接入构建系统和 QEMU 镜像。
- [ ] 建立 调度器 的验收标准和失败回滚策略。
- [ ] 记录 调度器 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 调度器 增加性能指标和安全审计点。

#### 验收标准

- 调度器 可以独立构建。
- 调度器 可以在 QEMU 或测试环境中运行。
- 调度器 的错误路径有日志。
- 调度器 的权限检查可被测试覆盖。
- 调度器 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.4 进程模型

内核架构 - 进程模型 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 进程模型 的最小可运行目标。
- [ ] 实现 进程模型 的 Rust 数据结构和错误模型。
- [ ] 接入 进程模型 的日志输出和诊断字段。
- [ ] 为 进程模型 编写单元测试或集成测试。
- [ ] 把 进程模型 接入构建系统和 QEMU 镜像。
- [ ] 建立 进程模型 的验收标准和失败回滚策略。
- [ ] 记录 进程模型 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 进程模型 增加性能指标和安全审计点。

#### 验收标准

- 进程模型 可以独立构建。
- 进程模型 可以在 QEMU 或测试环境中运行。
- 进程模型 的错误路径有日志。
- 进程模型 的权限检查可被测试覆盖。
- 进程模型 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.5 线程模型

内核架构 - 线程模型 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 线程模型 的最小可运行目标。
- [ ] 实现 线程模型 的 Rust 数据结构和错误模型。
- [ ] 接入 线程模型 的日志输出和诊断字段。
- [ ] 为 线程模型 编写单元测试或集成测试。
- [ ] 把 线程模型 接入构建系统和 QEMU 镜像。
- [ ] 建立 线程模型 的验收标准和失败回滚策略。
- [ ] 记录 线程模型 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 线程模型 增加性能指标和安全审计点。

#### 验收标准

- 线程模型 可以独立构建。
- 线程模型 可以在 QEMU 或测试环境中运行。
- 线程模型 的错误路径有日志。
- 线程模型 的权限检查可被测试覆盖。
- 线程模型 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.6 系统调用

内核架构 - 系统调用 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 系统调用 的最小可运行目标。
- [ ] 实现 系统调用 的 Rust 数据结构和错误模型。
- [ ] 接入 系统调用 的日志输出和诊断字段。
- [ ] 为 系统调用 编写单元测试或集成测试。
- [ ] 把 系统调用 接入构建系统和 QEMU 镜像。
- [ ] 建立 系统调用 的验收标准和失败回滚策略。
- [ ] 记录 系统调用 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 系统调用 增加性能指标和安全审计点。

#### 验收标准

- 系统调用 可以独立构建。
- 系统调用 可以在 QEMU 或测试环境中运行。
- 系统调用 的错误路径有日志。
- 系统调用 的权限检查可被测试覆盖。
- 系统调用 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.7 IPC

内核架构 - IPC 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [x] 实现按 feature 区分的 ABI/profile：feature-off/default 与历史 M59 `storage-irq-timeout-self-test` 为 ABI v23/syscall 0—41；历史 M54 `app-data-runtime` 及其 M59 `app-data-async-recovery-runtime` child 为 ABI v24/syscall 0—46；M55—M64 StorageServer branch 为 ABI v25/syscall 0—52，并使 old namespace syscall 42—46 返回 `Unsupported`；M65/M66 分别为 ABI v26/v27 与 syscall 53/54；M67—M71 分别为 ABI v28—v32 且不新增 syscall；M72 为 ABI v33/init-only syscall 55 `ServiceManifestOpen`；M73 为 ABI v34/init-only syscall 56 `ServiceSupervisorReport`；M74/M75/M76 分别为 ABI v35/v36/v37 且仍为 syscall 0—56；M77 为 ABI v38，并新增 init-only syscall 57 `MaintenanceSessionOpen`；M78/M79/M80 分别为 ABI v39/v40/v41 且仍只开放 syscall 0—57。M80 raw 58 严格 unknown。
- [x] 实现 syscall 23 `FileOpenAt` 与 syscall 24 `VmoRead`：move-only directory root、canonical root-relative UTF-8 path、`x2` 高/低 32 bit offset/requested length、有界 EOF read、immutable VMO rights 和 fault/attenuation/stale/transfer 验证。
- [x] 实现固定两项 wait-any 与相对 timeout：poll/infinite/finite、全参数验证、最低 index、单次 counter 采样的 deadline-first exact-token 仲裁。
- [x] 实现有界 1—8 项 wait-any 数组：8-byte canonical LE item、最多 64-byte usercopy、copy+全项 validation-before-ready、最低 index、真实八项 block/wake、显式 completion kind 与 exact token。
- [ ] 泛化为超过 8 项/任意长度 wait-any、wait-all、显式 cancel、高精度 timer 与 SMP-safe 实现。

以下其余 TODO 指向产品级通用 IPC 与 AndroidBox/系统服务接口，不因上述有界 M16 子集而自动完成：

- [ ] 定义 IPC 的最小可运行目标。
- [ ] 实现 IPC 的 Rust 数据结构和错误模型。
- [ ] 接入 IPC 的日志输出和诊断字段。
- [ ] 为 IPC 编写单元测试或集成测试。
- [ ] 把 IPC 接入构建系统和 QEMU 镜像。
- [ ] 建立 IPC 的验收标准和失败回滚策略。
- [ ] 记录 IPC 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 IPC 增加性能指标和安全审计点。

#### 验收标准

- IPC 可以独立构建。
- IPC 可以在 QEMU 或测试环境中运行。
- IPC 的错误路径有日志。
- IPC 的权限检查可被测试覆盖。
- IPC 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.8 Handle

内核架构 - Handle 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Handle 的最小可运行目标。
- [ ] 实现 Handle 的 Rust 数据结构和错误模型。
- [ ] 接入 Handle 的日志输出和诊断字段。
- [ ] 为 Handle 编写单元测试或集成测试。
- [ ] 把 Handle 接入构建系统和 QEMU 镜像。
- [ ] 建立 Handle 的验收标准和失败回滚策略。
- [ ] 记录 Handle 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Handle 增加性能指标和安全审计点。

#### 验收标准

- Handle 可以独立构建。
- Handle 可以在 QEMU 或测试环境中运行。
- Handle 的错误路径有日志。
- Handle 的权限检查可被测试覆盖。
- Handle 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 3.9 Capability

内核架构 - Capability 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Capability 的最小可运行目标。
- [ ] 实现 Capability 的 Rust 数据结构和错误模型。
- [ ] 接入 Capability 的日志输出和诊断字段。
- [ ] 为 Capability 编写单元测试或集成测试。
- [ ] 把 Capability 接入构建系统和 QEMU 镜像。
- [ ] 建立 Capability 的验收标准和失败回滚策略。
- [ ] 记录 Capability 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Capability 增加性能指标和安全审计点。

#### 验收标准

- Capability 可以独立构建。
- Capability 可以在 QEMU 或测试环境中运行。
- Capability 的错误路径有日志。
- Capability 的权限检查可被测试覆盖。
- Capability 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 4 章：驱动框架

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 4.0 M25 块设备、只读文件与持久化数据下层边界

M21 建立了第一个真实但刻意收窄的轮询驱动边界；M22 在同一两帧 DMA 所有权上完成 direct-root GICv2 IRQ 与双请求，而仍不是通用 DriverManager。FDT 以 32 槽固定容量枚举 enabled `virtio,mmio` 节点，并以两遍解析绑定 interrupt-parent、GICv2 phandle 与三 cell SPI；当前 32 个 transport 全部 coherent，唯一 active block 的 `[0,47,1]` 映射到 edge-rising INTID 79。

已实证的 transport/queue 契约如下：

- [x] 仅接受 modern virtio-mmio version 2；status reset 确认后按 `ACKNOWLEDGE`、`DRIVER`、`FEATURES_OK`、`DRIVER_OK` 迁移。
- [x] M25 normal 严格要求物理可写设备、`VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_FLUSH` 并拒绝 RO；M22 race build 保留物理 RO。legacy transport、non-coherent DMA、多个或零 active block 均拒绝。
- [x] queue 0 固定 size 8，216-byte split ring 独占一个 frame；第二个 frame 含两个 536-byte request slot，descriptor head 为 0/3。
- [x] GICv2 在本地 IRQ 开放前完成 SPI trigger/target/priority/clear/enable；virtio source ACK 与 used drain 发生在 GIC EOI 前，正常前台没有 used-ring polling fallback。
- [x] 单一静态 storage owner 永久持有两帧；generation token 支持两个同时 outstanding 与乱序完成，并拒绝 duplicate/unknown/stale completion。
- [x] physical-counter deadline、一次屏蔽 IRQ race drain、reset/re-negotiate、token epoch 作废和同两帧恢复进入自测状态机。
- [x] M25 的 512-byte block path 仍通过同一 IRQ-only owner；normal parser/read/write/flush=`273/280/1/1`，request/completion/IRQ completion=`282/282/282`，read/write bytes=`143360/512`，仍只有两帧 DMA。
- [x] protective MBR、主备 GPT header/array CRC、128-entry mirror 与唯一 `BNDROID_SYS` 分区被严格验证；双 FAT 镜像一致的 FAT16 由 `/system` VFS 只读访问。
- [x] M24 将验证后的两个文件发布为 2-entry/68-byte immutable boot catalog；`init` 以 move-only root capability 通过 `FileOpenAt`/`VmoRead` 完成内容、EOF、fault、rights、stale 与 transfer 证明，runtime disk read 为 0。
- [x] M25 新增 private `BNDROID_DATA` index 1/LBA 64—127、16-byte GUID-bound format epoch 与两个 512-byte CRC record slot；双槽同时有效时 generation 必须相邻。
- [x] type 1 WRITE/type 4 FLUSH 均验证 `used.len=1`；inactive-slot WRITE completion 后才 FLUSH，随后重读双槽、保留旧已选槽，fresh generation `0→1`。system 与 DATA 外写入会在请求账本不变时被拒绝。
- [x] 跨 QEMU 同一 raw image 实证 `0→1→2`、DATA 外不变、`changed_data_bytes=98`；破坏最新槽后回退 generation 0 并重新提交 1。
- [x] 完整 M25 `./scripts/test.sh` 从头 exit 0：269 项 host tests（ABI/ELF/SM/shared/kernel=`16/19/31/0/203`）、双 AArch64 Clippy、五个 normal QEMU、11 类存储负测、IRQ timeout/reset/late-event 自测及既有全部 negative/rollback/fault checks。

这不是用户态驱动框架、通用异步 block scheduler 或通用文件系统。M25 只有内核启动期 DATA 范围内的 bounded raw record write/flush/persistence，没有通用文件写入或 cache、exactly-once、认证、anti-rollback 或产品级 crash consistency（`crash_consistency=0`）；EL0 当时只有两个固定 boot 文件的 immutable VMO open/read。M26—M40 依次加入 ramfb/input、Surface/App、GraphicsBuffer、lifecycle、bounded recovery、software frame grant 与 two-buffer swapchain；M41 增加 fixed two-window userspace z-order/occlusion/damage/input proof。在截至 M41 的这段历史基线中，仍没有通用 watchdog/backoff/restart budget、重复 crash、硬件 vblank/pageflip、任意 App/window manager、持久 compositor event loop 或 text/IME；后续 M43—M49 已增加有界 text/input 与固定两服务监督 witness，但仍未产品泛化。`ACCESS_PLATFORM`、non-coherent DMA、IOMMU、GICv3/SMP 和真实设备也仍未支持。

### 4.1 用户态驱动

驱动框架 - 用户态驱动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 用户态驱动 的最小可运行目标。
- [ ] 实现 用户态驱动 的 Rust 数据结构和错误模型。
- [ ] 接入 用户态驱动 的日志输出和诊断字段。
- [ ] 为 用户态驱动 编写单元测试或集成测试。
- [ ] 把 用户态驱动 接入构建系统和 QEMU 镜像。
- [ ] 建立 用户态驱动 的验收标准和失败回滚策略。
- [ ] 记录 用户态驱动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 用户态驱动 增加性能指标和安全审计点。

#### 验收标准

- 用户态驱动 可以独立构建。
- 用户态驱动 可以在 QEMU 或测试环境中运行。
- 用户态驱动 的错误路径有日志。
- 用户态驱动 的权限检查可被测试覆盖。
- 用户态驱动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.2 DriverManager

驱动框架 - DriverManager 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 DriverManager 的最小可运行目标。
- [ ] 实现 DriverManager 的 Rust 数据结构和错误模型。
- [ ] 接入 DriverManager 的日志输出和诊断字段。
- [ ] 为 DriverManager 编写单元测试或集成测试。
- [ ] 把 DriverManager 接入构建系统和 QEMU 镜像。
- [ ] 建立 DriverManager 的验收标准和失败回滚策略。
- [ ] 记录 DriverManager 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 DriverManager 增加性能指标和安全审计点。

#### 验收标准

- DriverManager 可以独立构建。
- DriverManager 可以在 QEMU 或测试环境中运行。
- DriverManager 的错误路径有日志。
- DriverManager 的权限检查可被测试覆盖。
- DriverManager 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.3 显示驱动

驱动框架 - 显示驱动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 显示驱动 的最小可运行目标。
- [ ] 实现 显示驱动 的 Rust 数据结构和错误模型。
- [ ] 接入 显示驱动 的日志输出和诊断字段。
- [ ] 为 显示驱动 编写单元测试或集成测试。
- [ ] 把 显示驱动 接入构建系统和 QEMU 镜像。
- [ ] 建立 显示驱动 的验收标准和失败回滚策略。
- [ ] 记录 显示驱动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 显示驱动 增加性能指标和安全审计点。

#### 验收标准

- 显示驱动 可以独立构建。
- 显示驱动 可以在 QEMU 或测试环境中运行。
- 显示驱动 的错误路径有日志。
- 显示驱动 的权限检查可被测试覆盖。
- 显示驱动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.4 触控驱动

驱动框架 - 触控驱动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 触控驱动 的最小可运行目标。
- [ ] 实现 触控驱动 的 Rust 数据结构和错误模型。
- [ ] 接入 触控驱动 的日志输出和诊断字段。
- [ ] 为 触控驱动 编写单元测试或集成测试。
- [ ] 把 触控驱动 接入构建系统和 QEMU 镜像。
- [ ] 建立 触控驱动 的验收标准和失败回滚策略。
- [ ] 记录 触控驱动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 触控驱动 增加性能指标和安全审计点。

#### 验收标准

- 触控驱动 可以独立构建。
- 触控驱动 可以在 QEMU 或测试环境中运行。
- 触控驱动 的错误路径有日志。
- 触控驱动 的权限检查可被测试覆盖。
- 触控驱动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.5 GPU 路线

驱动框架 - GPU 路线 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 GPU 路线 的最小可运行目标。
- [ ] 实现 GPU 路线 的 Rust 数据结构和错误模型。
- [ ] 接入 GPU 路线 的日志输出和诊断字段。
- [ ] 为 GPU 路线 编写单元测试或集成测试。
- [ ] 把 GPU 路线 接入构建系统和 QEMU 镜像。
- [ ] 建立 GPU 路线 的验收标准和失败回滚策略。
- [ ] 记录 GPU 路线 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 GPU 路线 增加性能指标和安全审计点。

#### 验收标准

- GPU 路线 可以独立构建。
- GPU 路线 可以在 QEMU 或测试环境中运行。
- GPU 路线 的错误路径有日志。
- GPU 路线 的权限检查可被测试覆盖。
- GPU 路线 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.6 音频驱动

驱动框架 - 音频驱动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 音频驱动 的最小可运行目标。
- [ ] 实现 音频驱动 的 Rust 数据结构和错误模型。
- [ ] 接入 音频驱动 的日志输出和诊断字段。
- [ ] 为 音频驱动 编写单元测试或集成测试。
- [ ] 把 音频驱动 接入构建系统和 QEMU 镜像。
- [ ] 建立 音频驱动 的验收标准和失败回滚策略。
- [ ] 记录 音频驱动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 音频驱动 增加性能指标和安全审计点。

#### 验收标准

- 音频驱动 可以独立构建。
- 音频驱动 可以在 QEMU 或测试环境中运行。
- 音频驱动 的错误路径有日志。
- 音频驱动 的权限检查可被测试覆盖。
- 音频驱动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.7 相机驱动

驱动框架 - 相机驱动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 相机驱动 的最小可运行目标。
- [ ] 实现 相机驱动 的 Rust 数据结构和错误模型。
- [ ] 接入 相机驱动 的日志输出和诊断字段。
- [ ] 为 相机驱动 编写单元测试或集成测试。
- [ ] 把 相机驱动 接入构建系统和 QEMU 镜像。
- [ ] 建立 相机驱动 的验收标准和失败回滚策略。
- [ ] 记录 相机驱动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 相机驱动 增加性能指标和安全审计点。

#### 验收标准

- 相机驱动 可以独立构建。
- 相机驱动 可以在 QEMU 或测试环境中运行。
- 相机驱动 的错误路径有日志。
- 相机驱动 的权限检查可被测试覆盖。
- 相机驱动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 4.8 电源管理

驱动框架 - 电源管理 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 电源管理 的最小可运行目标。
- [ ] 实现 电源管理 的 Rust 数据结构和错误模型。
- [ ] 接入 电源管理 的日志输出和诊断字段。
- [ ] 为 电源管理 编写单元测试或集成测试。
- [ ] 把 电源管理 接入构建系统和 QEMU 镜像。
- [ ] 建立 电源管理 的验收标准和失败回滚策略。
- [ ] 记录 电源管理 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 电源管理 增加性能指标和安全审计点。

#### 验收标准

- 电源管理 可以独立构建。
- 电源管理 可以在 QEMU 或测试环境中运行。
- 电源管理 的错误路径有日志。
- 电源管理 的权限检查可被测试覆盖。
- 电源管理 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 5 章：系统服务

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 5.1 Init

系统服务 - Init 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Init 的最小可运行目标。
- [ ] 实现 Init 的 Rust 数据结构和错误模型。
- [ ] 接入 Init 的日志输出和诊断字段。
- [ ] 为 Init 编写单元测试或集成测试。
- [ ] 把 Init 接入构建系统和 QEMU 镜像。
- [ ] 建立 Init 的验收标准和失败回滚策略。
- [ ] 记录 Init 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Init 增加性能指标和安全审计点。

#### 验收标准

- Init 可以独立构建。
- Init 可以在 QEMU 或测试环境中运行。
- Init 的错误路径有日志。
- Init 的权限检查可被测试覆盖。
- Init 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.2 ServiceManager

系统服务 - ServiceManager 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

M9 建立 init-hosted 最小子集，M12 移出 ServiceManager，M13 使用四镜像，M14 加入两次 post-ready echo。M15 增加 BSM1 v1 Unregister/Reply 与一次 provider-driven 十阶段动态生命周期、显式 causal confirmation 和精确空闲图。M16 再加入 kind/opcode 通用分派与一次固定空间 post-cleanup round；M17 加入 kernel-stamped generation sender、manager 固定 PID allowlist、delegated-endpoint 拒绝和 48-byte 对抗性 reducer。M18 历史切片在 primary 常驻时短暂启动 secondary，经共享 manager ingress 和各自 private reply 完成两个 outstanding Lookup，随后 secondary exit/reap；M19 加入 ABI v12 八项 wait-array，但不改变服务图。M20 则让两个 Client 都保持常驻并各有独立、认证的 manager session，Lookup 回复沿 session 路由；固定 transcript 覆盖 secondary 停滞、primary 持续前进、一次 lease revoke/reattach、provider abort 和旧 lease 双端拒绝，最终收敛到五进程、16 endpoint/8 pair。服务子系统当前仍为 `dynamic_lifecycle=1 dynamic_registration_loop=1 idle_wait_graph=1 reusable=1 constant_space=1 bounded_reuse=1 writer_identity=1 bounded_slice=1 multi_session_round=1 general_runtime=0`，只是两个 Client、四个 txid 的 self-test，不是任意客户端产品 runtime 或 Android Binder ServiceManager。M29 在其下方增加 EL0-owned single Surface；M30 把 catalog 扩到六镜像并增加独立 SurfaceServer/Launcher 与一对 UI Channel；M31 再扩到七镜像、独立 App 与第二对 UI Channel，但始终保持这份 M20 core Channel 账本不变。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [x] 定义 ServiceManager 的最小可运行目标（独立、init-only 监督、Registry 容量 4、两代 self-test）。
- [x] 实现 ServiceManager 的 Rust 数据结构和错误模型（BSM1 frame + BSA1 attach + epoch Registry）。
- [x] 接入 ServiceManager 的日志输出和诊断字段（`SERVICE_RESTART_OK`、`SERVICE_MANAGER_OK`、`SERVICE_LOOP_OK`、`SERVICE_MULTISESSION_OK`、`SERVICE_WAIT_TOPOLOGY_OK`、`RESIDENT_STABILITY_OK`）。
- [x] 为 ServiceManager 编写单元测试和 QEMU 集成测试。
- [x] 把 ServiceManager 接入 workspace 与 init 监督；M13 的四角色、M30 的六角色 catalog 是历史基线，M31 全局 all-or-none catalog 已扩为七个 pairwise-distinct 镜像。
- [x] 建立有界 ServiceManager 原型的验收标准和失败回滚/退出回收策略。
- [ ] 记录 ServiceManager 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 ServiceManager 增加性能指标和安全审计点。

#### 验收标准

- `bndr-sm` crate 可以独立构建并通过 host tests。
- 独立 EL0 ServiceManager 原型可由 init 完成一次受监督重启；第二代与 provider、primary、secondary 保持常驻，在 M15/M16 生命周期、M17 authenticated ACL、M18 历史并发及 M19 wait-array 基线上完成 M20 两条独立 session、一次 stalled-secondary revoke/reattach 和 stale-lease 拒绝。
- role denial、重复注册、未知查找、跨 epoch stale instance、旧 PID、dependent peer-close/rebind、generation-handle ACK/DONE 来源，以及 M20 的 16 endpoint/8 pair 图、cross-image `1/1/2/2/2/0` 都有验收契约；M20 原始等待项为 `4/2/2/2`，M29 历史上在 manager index 0 前置 Surface 后为 `5/2/2/2`。M32 默认认证基线的 server/launcher/app 等待为 `3/1/1`，20 endpoint/10 pair、object/many/array=`7/2/3`、23 handle；M33/M34 专用 profile 都收敛为 26 endpoint/13 pair、29 handle、五对常驻窗口通道和 wait=`8`（many/array=`2/6`），M34 另精确验证终止遗弃 token=`0/0/1`。这些稳定向量都不是产品级长期无泄漏证明。
- `dynamic_lifecycle=1 dynamic_registration_loop=1 idle_wait_graph=1 reusable=1 constant_space=1 bounded_reuse=1 writer_identity=1 bounded_slice=1 multi_session_round=1 general_runtime=0` 不代表产品化：M20 只证明两个固定 Client 在四个固定 txid 中完成一次受 supervisor 驱动的停滞隔离和 revoke/reattach。任意接入、公平轮转、重复 churn、自发 crash/peer-close/malformed-session 隔离及产品 identity/ACL/list/persistence/audit 仍属后续。
- BSA1 bootstrap identity 本身不等于通用 caller identity；M17 由 kernel-stamped sender 实现固定角色 ACL，M20 把 full generation PID 认证用于两个固定 Client 的 manager 与 provider 路径，但仍不能宣称产品权限覆盖达标。
- BSM1/BSA1 已有显式 v1，但对外稳定接口与 AndroidBox/Binder/driver compatibility 尚未建立。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.3 LogServer

系统服务 - LogServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 LogServer 的最小可运行目标。
- [ ] 实现 LogServer 的 Rust 数据结构和错误模型。
- [ ] 接入 LogServer 的日志输出和诊断字段。
- [ ] 为 LogServer 编写单元测试或集成测试。
- [ ] 把 LogServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 LogServer 的验收标准和失败回滚策略。
- [ ] 记录 LogServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 LogServer 增加性能指标和安全审计点。

#### 验收标准

- LogServer 可以独立构建。
- LogServer 可以在 QEMU 或测试环境中运行。
- LogServer 的错误路径有日志。
- LogServer 的权限检查可被测试覆盖。
- LogServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.4 PackageManager

系统服务 - PackageManager 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

当前实现不能等同于本节的通用系统服务：隔离 Install-0/Update-0 已有本地 QEMU
证据；Uninstall-0 只接受受信离线 host 的 boot-time `BNDUNS01` `fw_cfg` 请求，
其双同代 tombstone 与 reinstall 约束有 43 项 package-store host tests 和
12-boot QEMU gate。没有 runtime mutation capability、Settings 卸载、managed app data 或
通用 PackageManager API。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 PackageManager 的最小可运行目标。
- [ ] 实现 PackageManager 的 Rust 数据结构和错误模型。
- [ ] 接入 PackageManager 的日志输出和诊断字段。
- [ ] 为 PackageManager 编写单元测试或集成测试。
- [ ] 把 PackageManager 接入构建系统和 QEMU 镜像。
- [ ] 建立 PackageManager 的验收标准和失败回滚策略。
- [ ] 记录 PackageManager 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 PackageManager 增加性能指标和安全审计点。

#### 验收标准

- PackageManager 可以独立构建。
- PackageManager 可以在 QEMU 或测试环境中运行。
- PackageManager 的错误路径有日志。
- PackageManager 的权限检查可被测试覆盖。
- PackageManager 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.5 PermissionManager

系统服务 - PermissionManager 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 PermissionManager 的最小可运行目标。
- [ ] 实现 PermissionManager 的 Rust 数据结构和错误模型。
- [ ] 接入 PermissionManager 的日志输出和诊断字段。
- [ ] 为 PermissionManager 编写单元测试或集成测试。
- [ ] 把 PermissionManager 接入构建系统和 QEMU 镜像。
- [ ] 建立 PermissionManager 的验收标准和失败回滚策略。
- [ ] 记录 PermissionManager 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 PermissionManager 增加性能指标和安全审计点。

#### 验收标准

- PermissionManager 可以独立构建。
- PermissionManager 可以在 QEMU 或测试环境中运行。
- PermissionManager 的错误路径有日志。
- PermissionManager 的权限检查可被测试覆盖。
- PermissionManager 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.6 AppManager

系统服务 - AppManager 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 AppManager 的最小可运行目标。
- [ ] 实现 AppManager 的 Rust 数据结构和错误模型。
- [ ] 接入 AppManager 的日志输出和诊断字段。
- [ ] 为 AppManager 编写单元测试或集成测试。
- [ ] 把 AppManager 接入构建系统和 QEMU 镜像。
- [ ] 建立 AppManager 的验收标准和失败回滚策略。
- [ ] 记录 AppManager 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 AppManager 增加性能指标和安全审计点。

#### 验收标准

- AppManager 可以独立构建。
- AppManager 可以在 QEMU 或测试环境中运行。
- AppManager 的错误路径有日志。
- AppManager 的权限检查可被测试覆盖。
- AppManager 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.7 WindowServer

系统服务 - WindowServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 WindowServer 的最小可运行目标。
- [ ] 实现 WindowServer 的 Rust 数据结构和错误模型。
- [ ] 接入 WindowServer 的日志输出和诊断字段。
- [ ] 为 WindowServer 编写单元测试或集成测试。
- [ ] 把 WindowServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 WindowServer 的验收标准和失败回滚策略。
- [ ] 记录 WindowServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 WindowServer 增加性能指标和安全审计点。

#### 验收标准

- WindowServer 可以独立构建。
- WindowServer 可以在 QEMU 或测试环境中运行。
- WindowServer 的错误路径有日志。
- WindowServer 的权限检查可被测试覆盖。
- WindowServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.8 InputServer

系统服务 - InputServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [x] 定义 M45—M48 bounded InputServer 与单服务监督的最小可运行目标。
- [x] 实现 `bndr-input` 的 no_std Rust 数据结构、strict wire 与错误模型。
- [x] 接入 InputServer 的 kernel/runtime trace、permission audit 与诊断字段。
- [x] 为 InputServer 编写 host、kernel 与 QEMU 集成测试。
- [x] 把 bounded InputServer 接入构建系统和 QEMU 镜像。
- [x] 建立 M47 一次 restart/reacquire、30 ms backoff 与 route resync 的有界验收标准。
- [x] 建立 M48 `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、process-exit restart、health-timeout、budget 1 runtime quarantine、degraded UI 与严格 trace/validator/topology 验收。
- [ ] 记录 InputServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为产品级 InputServer 补齐性能指标与完整安全审计（M47 只覆盖 Surface `InputAcquire` 拒绝）。

> 上述已勾选项只属于固定容量的 M45—M48 QEMU witness；M48 runtime 只监督一个 InputServer。依赖感知的 SurfaceServer+InputServer 多服务监督、degraded→recovered、restart-storm 防护、AndroidBox 集成与产品级输入栈仍未完成。

#### 验收标准

- InputServer 可以独立构建。
- InputServer 可以在 QEMU 或测试环境中运行。
- InputServer 的错误路径有日志。
- InputServer 的权限检查可被测试覆盖。
- InputServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.9 AudioServer

系统服务 - AudioServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 AudioServer 的最小可运行目标。
- [ ] 实现 AudioServer 的 Rust 数据结构和错误模型。
- [ ] 接入 AudioServer 的日志输出和诊断字段。
- [ ] 为 AudioServer 编写单元测试或集成测试。
- [ ] 把 AudioServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 AudioServer 的验收标准和失败回滚策略。
- [ ] 记录 AudioServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 AudioServer 增加性能指标和安全审计点。

#### 验收标准

- AudioServer 可以独立构建。
- AudioServer 可以在 QEMU 或测试环境中运行。
- AudioServer 的错误路径有日志。
- AudioServer 的权限检查可被测试覆盖。
- AudioServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 5.10 NetworkServer

系统服务 - NetworkServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 NetworkServer 的最小可运行目标。
- [ ] 实现 NetworkServer 的 Rust 数据结构和错误模型。
- [ ] 接入 NetworkServer 的日志输出和诊断字段。
- [ ] 为 NetworkServer 编写单元测试或集成测试。
- [ ] 把 NetworkServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 NetworkServer 的验收标准和失败回滚策略。
- [ ] 记录 NetworkServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 NetworkServer 增加性能指标和安全审计点。

#### 验收标准

- NetworkServer 可以独立构建。
- NetworkServer 可以在 QEMU 或测试环境中运行。
- NetworkServer 的错误路径有日志。
- NetworkServer 的权限检查可被测试覆盖。
- NetworkServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 6 章：图形系统

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 6.1 Surface

图形系统 - Surface 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Surface 的最小可运行目标。
- [ ] 实现 Surface 的 Rust 数据结构和错误模型。
- [ ] 接入 Surface 的日志输出和诊断字段。
- [ ] 为 Surface 编写单元测试或集成测试。
- [ ] 把 Surface 接入构建系统和 QEMU 镜像。
- [ ] 建立 Surface 的验收标准和失败回滚策略。
- [ ] 记录 Surface 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Surface 增加性能指标和安全审计点。

#### 验收标准

- Surface 可以独立构建。
- Surface 可以在 QEMU 或测试环境中运行。
- Surface 的错误路径有日志。
- Surface 的权限检查可被测试覆盖。
- Surface 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.2 Window

图形系统 - Window 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Window 的最小可运行目标。
- [ ] 实现 Window 的 Rust 数据结构和错误模型。
- [ ] 接入 Window 的日志输出和诊断字段。
- [ ] 为 Window 编写单元测试或集成测试。
- [ ] 把 Window 接入构建系统和 QEMU 镜像。
- [ ] 建立 Window 的验收标准和失败回滚策略。
- [ ] 记录 Window 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Window 增加性能指标和安全审计点。

#### 验收标准

- Window 可以独立构建。
- Window 可以在 QEMU 或测试环境中运行。
- Window 的错误路径有日志。
- Window 的权限检查可被测试覆盖。
- Window 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.3 Compositor

图形系统 - Compositor 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Compositor 的最小可运行目标。
- [ ] 实现 Compositor 的 Rust 数据结构和错误模型。
- [ ] 接入 Compositor 的日志输出和诊断字段。
- [ ] 为 Compositor 编写单元测试或集成测试。
- [ ] 把 Compositor 接入构建系统和 QEMU 镜像。
- [ ] 建立 Compositor 的验收标准和失败回滚策略。
- [ ] 记录 Compositor 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Compositor 增加性能指标和安全审计点。

#### 验收标准

- Compositor 可以独立构建。
- Compositor 可以在 QEMU 或测试环境中运行。
- Compositor 的错误路径有日志。
- Compositor 的权限检查可被测试覆盖。
- Compositor 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.4 VSync

图形系统 - VSync 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 VSync 的最小可运行目标。
- [ ] 实现 VSync 的 Rust 数据结构和错误模型。
- [ ] 接入 VSync 的日志输出和诊断字段。
- [ ] 为 VSync 编写单元测试或集成测试。
- [ ] 把 VSync 接入构建系统和 QEMU 镜像。
- [ ] 建立 VSync 的验收标准和失败回滚策略。
- [ ] 记录 VSync 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 VSync 增加性能指标和安全审计点。

#### 验收标准

- VSync 可以独立构建。
- VSync 可以在 QEMU 或测试环境中运行。
- VSync 的错误路径有日志。
- VSync 的权限检查可被测试覆盖。
- VSync 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.5 动画

图形系统 - 动画 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 动画 的最小可运行目标。
- [ ] 实现 动画 的 Rust 数据结构和错误模型。
- [ ] 接入 动画 的日志输出和诊断字段。
- [ ] 为 动画 编写单元测试或集成测试。
- [ ] 把 动画 接入构建系统和 QEMU 镜像。
- [ ] 建立 动画 的验收标准和失败回滚策略。
- [ ] 记录 动画 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 动画 增加性能指标和安全审计点。

#### 验收标准

- 动画 可以独立构建。
- 动画 可以在 QEMU 或测试环境中运行。
- 动画 的错误路径有日志。
- 动画 的权限检查可被测试覆盖。
- 动画 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.6 截图权限

图形系统 - 截图权限 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 截图权限 的最小可运行目标。
- [ ] 实现 截图权限 的 Rust 数据结构和错误模型。
- [ ] 接入 截图权限 的日志输出和诊断字段。
- [ ] 为 截图权限 编写单元测试或集成测试。
- [ ] 把 截图权限 接入构建系统和 QEMU 镜像。
- [ ] 建立 截图权限 的验收标准和失败回滚策略。
- [ ] 记录 截图权限 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 截图权限 增加性能指标和安全审计点。

#### 验收标准

- 截图权限 可以独立构建。
- 截图权限 可以在 QEMU 或测试环境中运行。
- 截图权限 的错误路径有日志。
- 截图权限 的权限检查可被测试覆盖。
- 截图权限 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.7 安全显示

图形系统 - 安全显示 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 安全显示 的最小可运行目标。
- [ ] 实现 安全显示 的 Rust 数据结构和错误模型。
- [ ] 接入 安全显示 的日志输出和诊断字段。
- [ ] 为 安全显示 编写单元测试或集成测试。
- [ ] 把 安全显示 接入构建系统和 QEMU 镜像。
- [ ] 建立 安全显示 的验收标准和失败回滚策略。
- [ ] 记录 安全显示 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 安全显示 增加性能指标和安全审计点。

#### 验收标准

- 安全显示 可以独立构建。
- 安全显示 可以在 QEMU 或测试环境中运行。
- 安全显示 的错误路径有日志。
- 安全显示 的权限检查可被测试覆盖。
- 安全显示 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 6.8 Android Surface Bridge

图形系统 - Android Surface Bridge 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Android Surface Bridge 的最小可运行目标。
- [ ] 实现 Android Surface Bridge 的 Rust 数据结构和错误模型。
- [ ] 接入 Android Surface Bridge 的日志输出和诊断字段。
- [ ] 为 Android Surface Bridge 编写单元测试或集成测试。
- [ ] 把 Android Surface Bridge 接入构建系统和 QEMU 镜像。
- [ ] 建立 Android Surface Bridge 的验收标准和失败回滚策略。
- [ ] 记录 Android Surface Bridge 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Android Surface Bridge 增加性能指标和安全审计点。

#### 验收标准

- Android Surface Bridge 可以独立构建。
- Android Surface Bridge 可以在 QEMU 或测试环境中运行。
- Android Surface Bridge 的错误路径有日志。
- Android Surface Bridge 的权限检查可被测试覆盖。
- Android Surface Bridge 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 7 章：原生应用生态

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 7.1 bapp 包格式

原生应用生态 - bapp 包格式 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 bapp 包格式 的最小可运行目标。
- [ ] 实现 bapp 包格式 的 Rust 数据结构和错误模型。
- [ ] 接入 bapp 包格式 的日志输出和诊断字段。
- [ ] 为 bapp 包格式 编写单元测试或集成测试。
- [ ] 把 bapp 包格式 接入构建系统和 QEMU 镜像。
- [ ] 建立 bapp 包格式 的验收标准和失败回滚策略。
- [ ] 记录 bapp 包格式 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 bapp 包格式 增加性能指标和安全审计点。

#### 验收标准

- bapp 包格式 可以独立构建。
- bapp 包格式 可以在 QEMU 或测试环境中运行。
- bapp 包格式 的错误路径有日志。
- bapp 包格式 的权限检查可被测试覆盖。
- bapp 包格式 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.2 manifest

原生应用生态 - manifest 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 manifest 的最小可运行目标。
- [ ] 实现 manifest 的 Rust 数据结构和错误模型。
- [ ] 接入 manifest 的日志输出和诊断字段。
- [ ] 为 manifest 编写单元测试或集成测试。
- [ ] 把 manifest 接入构建系统和 QEMU 镜像。
- [ ] 建立 manifest 的验收标准和失败回滚策略。
- [ ] 记录 manifest 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 manifest 增加性能指标和安全审计点。

#### 验收标准

- manifest 可以独立构建。
- manifest 可以在 QEMU 或测试环境中运行。
- manifest 的错误路径有日志。
- manifest 的权限检查可被测试覆盖。
- manifest 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.3 签名

原生应用生态 - 签名 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 签名 的最小可运行目标。
- [ ] 实现 签名 的 Rust 数据结构和错误模型。
- [ ] 接入 签名 的日志输出和诊断字段。
- [ ] 为 签名 编写单元测试或集成测试。
- [ ] 把 签名 接入构建系统和 QEMU 镜像。
- [ ] 建立 签名 的验收标准和失败回滚策略。
- [ ] 记录 签名 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 签名 增加性能指标和安全审计点。

#### 验收标准

- 签名 可以独立构建。
- 签名 可以在 QEMU 或测试环境中运行。
- 签名 的错误路径有日志。
- 签名 的权限检查可被测试覆盖。
- 签名 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.4 生命周期

原生应用生态 - 生命周期 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 生命周期 的最小可运行目标。
- [ ] 实现 生命周期 的 Rust 数据结构和错误模型。
- [ ] 接入 生命周期 的日志输出和诊断字段。
- [ ] 为 生命周期 编写单元测试或集成测试。
- [ ] 把 生命周期 接入构建系统和 QEMU 镜像。
- [ ] 建立 生命周期 的验收标准和失败回滚策略。
- [ ] 记录 生命周期 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 生命周期 增加性能指标和安全审计点。

#### 验收标准

- 生命周期 可以独立构建。
- 生命周期 可以在 QEMU 或测试环境中运行。
- 生命周期 的错误路径有日志。
- 生命周期 的权限检查可被测试覆盖。
- 生命周期 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.5 Rust UI SDK

原生应用生态 - Rust UI SDK 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Rust UI SDK 的最小可运行目标。
- [ ] 实现 Rust UI SDK 的 Rust 数据结构和错误模型。
- [ ] 接入 Rust UI SDK 的日志输出和诊断字段。
- [ ] 为 Rust UI SDK 编写单元测试或集成测试。
- [ ] 把 Rust UI SDK 接入构建系统和 QEMU 镜像。
- [ ] 建立 Rust UI SDK 的验收标准和失败回滚策略。
- [ ] 记录 Rust UI SDK 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Rust UI SDK 增加性能指标和安全审计点。

#### 验收标准

- Rust UI SDK 可以独立构建。
- Rust UI SDK 可以在 QEMU 或测试环境中运行。
- Rust UI SDK 的错误路径有日志。
- Rust UI SDK 的权限检查可被测试覆盖。
- Rust UI SDK 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.6 系统应用

原生应用生态 - 系统应用 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 系统应用 的最小可运行目标。
- [ ] 实现 系统应用 的 Rust 数据结构和错误模型。
- [ ] 接入 系统应用 的日志输出和诊断字段。
- [ ] 为 系统应用 编写单元测试或集成测试。
- [ ] 把 系统应用 接入构建系统和 QEMU 镜像。
- [ ] 建立 系统应用 的验收标准和失败回滚策略。
- [ ] 记录 系统应用 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 系统应用 增加性能指标和安全审计点。

#### 验收标准

- 系统应用 可以独立构建。
- 系统应用 可以在 QEMU 或测试环境中运行。
- 系统应用 的错误路径有日志。
- 系统应用 的权限检查可被测试覆盖。
- 系统应用 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 7.7 开发者体验

原生应用生态 - 开发者体验 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 开发者体验 的最小可运行目标。
- [ ] 实现 开发者体验 的 Rust 数据结构和错误模型。
- [ ] 接入 开发者体验 的日志输出和诊断字段。
- [ ] 为 开发者体验 编写单元测试或集成测试。
- [ ] 把 开发者体验 接入构建系统和 QEMU 镜像。
- [ ] 建立 开发者体验 的验收标准和失败回滚策略。
- [ ] 记录 开发者体验 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 开发者体验 增加性能指标和安全审计点。

#### 验收标准

- 开发者体验 可以独立构建。
- 开发者体验 可以在 QEMU 或测试环境中运行。
- 开发者体验 的错误路径有日志。
- 开发者体验 的权限检查可被测试覆盖。
- 开发者体验 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 8 章：AndroidBox

> 当前实现与目标架构必须分开：基础 `androidbox-dex0` 保留 DEX-0 两个固定
> pure-static integer `code_item`、旧 inline-text Activity-0 回归及 Resources-1
> pre-compatibility slice，并在纯数据 ActivityLifecycle-1 中先解释两指令 exact
> public no-argument constructor，再执行受限 `onCreate`；它不安装或发布 package
> state。独立 `androidbox-apk-install0` 的原始实证以仓库自有 APK-v2 单签名
> Resources-1 fixture 完成 signature-first admission、单包持久事务、首启安装、
> 两次无源恢复和只读安装快照；同一严格 Resources-1 shape 的本机
> `org.bndroid.macdemo` no-probe APK 也完成安装/恢复。两者都不代表任意 APK、
> ART 或一般 Android 兼容。同一隔离 profile 又以原始 demo 同包、同证书、递增版本的
> v3 fixture 完成 generation-2 Update-0、零写 replay/recovery 与 rollback/tamper 拒绝。另有
> boot-time Uninstall-0 的 canonical request 绑定 durable identity，
> 双同代 tombstone 逻辑撤销 APK；blob 不擦除、没有 managed package data，QEMU gate
> 已验证 `1→2→removed 3→reinstall 4`。这只完成严格受限单包生命周期；仍没有
> 通用安装/更新/卸载、运行时 UI、
> ActivityThread、
> ResourceManager、qualifier/alias、任意 View/layout、ART/Dalvik、Bionic、Binder、
> JNI、通用 Framework、权限/服务映射或一般 APK 兼容；
> `android_compatibility_claim=0`。下列章节仍是完整 AndroidBox 的未来路线。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 8.1 APK 安装

AndroidBox - APK 安装 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [x] 定义严格受限 APK Install-0：原始仓库 APK-v2 单签名 Resources-1 fixture 与
  同一严格 shape 的本机 `org.bndroid.macdemo` no-probe APK 均已完成受限安装/恢复。
- [x] 实现 signature-first admission、Rust 数据/错误模型及单包双 registry/双 blob
  持久事务；当前 43 项 package-store 单测覆盖 install/update 的
  write/torn-write/flush、ack loss、幂等重试与损坏回退，并覆盖 Uninstall-0
  双同代 tombstone、逐写/撕裂/flush/readback、单副本修复、零写 replay 和
  reinstall policy。
- [x] 接入日志、构建和本地 QEMU 门：完成篡改拒绝、首启安装、两次无 APK source
  generation-1 恢复、稳定零写恢复及 ABI-43/syscall-59 只读安装快照。
- [x] 完成严格受限 APK Update-0：仅同包、同 v2 signer certificate 和严格递增
  `versionCode` 的 v2→v3 fixture，经完整持久 readback 原子发布到 generation 2；
  同源 replay/无源恢复零写，rollback 与篡改 source 拒绝且磁盘不变。
- [x] 实现严格受限 APK Uninstall-0 存储/kernel boot path：仅受信 offline host
  `BNDUNS01` request，双同代 tombstone、逻辑 APK revocation、
  `NoManagedPackageData`；APK blob 不擦除，旧 kernel 无 downgrade-safe 声明；
  已留存 12-boot QEMU gate 与整盘证据。
- [ ] 扩展为通用 APK installer：任意合规 APK、多包、配额、稳定 App identity、
  更新/卸载及 per-app storage。
- [ ] 建立生产 signer policy、证书链/时间、v1/v3/v4、multi-signer 与 signer rotation。
- [ ] 实现 Installer/PackageManager UI/API，并接入完整权限、服务与审计体系。
- [ ] 在真实控制器、物理断电和目标硬件上完成性能、安全与恢复验收。

#### 验收标准与范围

- Install-0 可以独立构建，并在本地 QEMU 中按上述两个严格同 shape 证据样本运行。
- Install-0 的 admission、事务和恢复错误路径有日志及自动化测试。
- Update-0 只证明一个已安装包、一个同证书 v3 fixture 与 QEMU package disk；
  不提供通用多包 update policy、update UI 或可信硬件 anti-rollback。
- Uninstall-0 已完成 12-boot QEMU gate，但只有 boot-time host authority，无运行时
  UI/API，不擦除 blob，也没有 managed application data 可删。
- Install-0 只向 UI 发布只读、无权限的安装快照；APK bytes、handle 和 package-store
  mutation authority 均不进入 EL0 应用。
- 通用安装器的权限检查、稳定版本化接口和生产信任策略仍是未完成验收项。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.2 ART

AndroidBox - ART 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 ART 的最小可运行目标。
- [ ] 实现 ART 的 Rust 数据结构和错误模型。
- [ ] 接入 ART 的日志输出和诊断字段。
- [ ] 为 ART 编写单元测试或集成测试。
- [ ] 把 ART 接入构建系统和 QEMU 镜像。
- [ ] 建立 ART 的验收标准和失败回滚策略。
- [ ] 记录 ART 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 ART 增加性能指标和安全审计点。

#### 验收标准

- ART 可以独立构建。
- ART 可以在 QEMU 或测试环境中运行。
- ART 的错误路径有日志。
- ART 的权限检查可被测试覆盖。
- ART 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.3 Bionic

AndroidBox - Bionic 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Bionic 的最小可运行目标。
- [ ] 实现 Bionic 的 Rust 数据结构和错误模型。
- [ ] 接入 Bionic 的日志输出和诊断字段。
- [ ] 为 Bionic 编写单元测试或集成测试。
- [ ] 把 Bionic 接入构建系统和 QEMU 镜像。
- [ ] 建立 Bionic 的验收标准和失败回滚策略。
- [ ] 记录 Bionic 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Bionic 增加性能指标和安全审计点。

#### 验收标准

- Bionic 可以独立构建。
- Bionic 可以在 QEMU 或测试环境中运行。
- Bionic 的错误路径有日志。
- Bionic 的权限检查可被测试覆盖。
- Bionic 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.4 Binder

AndroidBox - Binder 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Binder 的最小可运行目标。
- [ ] 实现 Binder 的 Rust 数据结构和错误模型。
- [ ] 接入 Binder 的日志输出和诊断字段。
- [ ] 为 Binder 编写单元测试或集成测试。
- [ ] 把 Binder 接入构建系统和 QEMU 镜像。
- [ ] 建立 Binder 的验收标准和失败回滚策略。
- [ ] 记录 Binder 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Binder 增加性能指标和安全审计点。

#### 验收标准

- Binder 可以独立构建。
- Binder 可以在 QEMU 或测试环境中运行。
- Binder 的错误路径有日志。
- Binder 的权限检查可被测试覆盖。
- Binder 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.5 Linux ABI 子集

AndroidBox - Linux ABI 子集 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Linux ABI 子集 的最小可运行目标。
- [ ] 实现 Linux ABI 子集 的 Rust 数据结构和错误模型。
- [ ] 接入 Linux ABI 子集 的日志输出和诊断字段。
- [ ] 为 Linux ABI 子集 编写单元测试或集成测试。
- [ ] 把 Linux ABI 子集 接入构建系统和 QEMU 镜像。
- [ ] 建立 Linux ABI 子集 的验收标准和失败回滚策略。
- [ ] 记录 Linux ABI 子集 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Linux ABI 子集 增加性能指标和安全审计点。

#### 验收标准

- Linux ABI 子集 可以独立构建。
- Linux ABI 子集 可以在 QEMU 或测试环境中运行。
- Linux ABI 子集 的错误路径有日志。
- Linux ABI 子集 的权限检查可被测试覆盖。
- Linux ABI 子集 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.6 Framework Shim

AndroidBox - Framework Shim 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Framework Shim 的最小可运行目标。
- [ ] 实现 Framework Shim 的 Rust 数据结构和错误模型。
- [ ] 接入 Framework Shim 的日志输出和诊断字段。
- [ ] 为 Framework Shim 编写单元测试或集成测试。
- [ ] 把 Framework Shim 接入构建系统和 QEMU 镜像。
- [ ] 建立 Framework Shim 的验收标准和失败回滚策略。
- [ ] 记录 Framework Shim 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Framework Shim 增加性能指标和安全审计点。

#### 验收标准

- Framework Shim 可以独立构建。
- Framework Shim 可以在 QEMU 或测试环境中运行。
- Framework Shim 的错误路径有日志。
- Framework Shim 的权限检查可被测试覆盖。
- Framework Shim 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.7 服务映射

AndroidBox - 服务映射 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 服务映射 的最小可运行目标。
- [ ] 实现 服务映射 的 Rust 数据结构和错误模型。
- [ ] 接入 服务映射 的日志输出和诊断字段。
- [ ] 为 服务映射 编写单元测试或集成测试。
- [ ] 把 服务映射 接入构建系统和 QEMU 镜像。
- [ ] 建立 服务映射 的验收标准和失败回滚策略。
- [ ] 记录 服务映射 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 服务映射 增加性能指标和安全审计点。

#### 验收标准

- 服务映射 可以独立构建。
- 服务映射 可以在 QEMU 或测试环境中运行。
- 服务映射 的错误路径有日志。
- 服务映射 的权限检查可被测试覆盖。
- 服务映射 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 8.8 兼容性数据库

AndroidBox - 兼容性数据库 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 兼容性数据库 的最小可运行目标。
- [ ] 实现 兼容性数据库 的 Rust 数据结构和错误模型。
- [ ] 接入 兼容性数据库 的日志输出和诊断字段。
- [ ] 为 兼容性数据库 编写单元测试或集成测试。
- [ ] 把 兼容性数据库 接入构建系统和 QEMU 镜像。
- [ ] 建立 兼容性数据库 的验收标准和失败回滚策略。
- [ ] 记录 兼容性数据库 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 兼容性数据库 增加性能指标和安全审计点。

#### 验收标准

- 兼容性数据库 可以独立构建。
- 兼容性数据库 可以在 QEMU 或测试环境中运行。
- 兼容性数据库 的错误路径有日志。
- 兼容性数据库 的权限检查可被测试覆盖。
- 兼容性数据库 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 9 章：安全体系

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 9.1 代码签名

安全体系 - 代码签名 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 代码签名 的最小可运行目标。
- [ ] 实现 代码签名 的 Rust 数据结构和错误模型。
- [ ] 接入 代码签名 的日志输出和诊断字段。
- [ ] 为 代码签名 编写单元测试或集成测试。
- [ ] 把 代码签名 接入构建系统和 QEMU 镜像。
- [ ] 建立 代码签名 的验收标准和失败回滚策略。
- [ ] 记录 代码签名 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 代码签名 增加性能指标和安全审计点。

#### 验收标准

- 代码签名 可以独立构建。
- 代码签名 可以在 QEMU 或测试环境中运行。
- 代码签名 的错误路径有日志。
- 代码签名 的权限检查可被测试覆盖。
- 代码签名 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.2 沙箱

安全体系 - 沙箱 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 沙箱 的最小可运行目标。
- [ ] 实现 沙箱 的 Rust 数据结构和错误模型。
- [ ] 接入 沙箱 的日志输出和诊断字段。
- [ ] 为 沙箱 编写单元测试或集成测试。
- [ ] 把 沙箱 接入构建系统和 QEMU 镜像。
- [ ] 建立 沙箱 的验收标准和失败回滚策略。
- [ ] 记录 沙箱 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 沙箱 增加性能指标和安全审计点。

#### 验收标准

- 沙箱 可以独立构建。
- 沙箱 可以在 QEMU 或测试环境中运行。
- 沙箱 的错误路径有日志。
- 沙箱 的权限检查可被测试覆盖。
- 沙箱 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.3 权限

安全体系 - 权限 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 权限 的最小可运行目标。
- [ ] 实现 权限 的 Rust 数据结构和错误模型。
- [ ] 接入 权限 的日志输出和诊断字段。
- [ ] 为 权限 编写单元测试或集成测试。
- [ ] 把 权限 接入构建系统和 QEMU 镜像。
- [ ] 建立 权限 的验收标准和失败回滚策略。
- [ ] 记录 权限 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 权限 增加性能指标和安全审计点。

#### 验收标准

- 权限 可以独立构建。
- 权限 可以在 QEMU 或测试环境中运行。
- 权限 的错误路径有日志。
- 权限 的权限检查可被测试覆盖。
- 权限 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.4 Keychain

安全体系 - Keychain 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Keychain 的最小可运行目标。
- [ ] 实现 Keychain 的 Rust 数据结构和错误模型。
- [ ] 接入 Keychain 的日志输出和诊断字段。
- [ ] 为 Keychain 编写单元测试或集成测试。
- [ ] 把 Keychain 接入构建系统和 QEMU 镜像。
- [ ] 建立 Keychain 的验收标准和失败回滚策略。
- [ ] 记录 Keychain 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Keychain 增加性能指标和安全审计点。

#### 验收标准

- Keychain 可以独立构建。
- Keychain 可以在 QEMU 或测试环境中运行。
- Keychain 的错误路径有日志。
- Keychain 的权限检查可被测试覆盖。
- Keychain 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.5 审计

安全体系 - 审计 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 审计 的最小可运行目标。
- [ ] 实现 审计 的 Rust 数据结构和错误模型。
- [ ] 接入 审计 的日志输出和诊断字段。
- [ ] 为 审计 编写单元测试或集成测试。
- [ ] 把 审计 接入构建系统和 QEMU 镜像。
- [ ] 建立 审计 的验收标准和失败回滚策略。
- [ ] 记录 审计 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 审计 增加性能指标和安全审计点。

#### 验收标准

- 审计 可以独立构建。
- 审计 可以在 QEMU 或测试环境中运行。
- 审计 的错误路径有日志。
- 审计 的权限检查可被测试覆盖。
- 审计 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.6 安全启动

安全体系 - 安全启动 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 安全启动 的最小可运行目标。
- [ ] 实现 安全启动 的 Rust 数据结构和错误模型。
- [ ] 接入 安全启动 的日志输出和诊断字段。
- [ ] 为 安全启动 编写单元测试或集成测试。
- [ ] 把 安全启动 接入构建系统和 QEMU 镜像。
- [ ] 建立 安全启动 的验收标准和失败回滚策略。
- [ ] 记录 安全启动 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 安全启动 增加性能指标和安全审计点。

#### 验收标准

- 安全启动 可以独立构建。
- 安全启动 可以在 QEMU 或测试环境中运行。
- 安全启动 的错误路径有日志。
- 安全启动 的权限检查可被测试覆盖。
- 安全启动 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.7 OTA 验证

安全体系 - OTA 验证 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 OTA 验证 的最小可运行目标。
- [ ] 实现 OTA 验证 的 Rust 数据结构和错误模型。
- [ ] 接入 OTA 验证 的日志输出和诊断字段。
- [ ] 为 OTA 验证 编写单元测试或集成测试。
- [ ] 把 OTA 验证 接入构建系统和 QEMU 镜像。
- [ ] 建立 OTA 验证 的验收标准和失败回滚策略。
- [ ] 记录 OTA 验证 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 OTA 验证 增加性能指标和安全审计点。

#### 验收标准

- OTA 验证 可以独立构建。
- OTA 验证 可以在 QEMU 或测试环境中运行。
- OTA 验证 的错误路径有日志。
- OTA 验证 的权限检查可被测试覆盖。
- OTA 验证 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 9.8 攻击面控制

安全体系 - 攻击面控制 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 攻击面控制 的最小可运行目标。
- [ ] 实现 攻击面控制 的 Rust 数据结构和错误模型。
- [ ] 接入 攻击面控制 的日志输出和诊断字段。
- [ ] 为 攻击面控制 编写单元测试或集成测试。
- [ ] 把 攻击面控制 接入构建系统和 QEMU 镜像。
- [ ] 建立 攻击面控制 的验收标准和失败回滚策略。
- [ ] 记录 攻击面控制 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 攻击面控制 增加性能指标和安全审计点。

#### 验收标准

- 攻击面控制 可以独立构建。
- 攻击面控制 可以在 QEMU 或测试环境中运行。
- 攻击面控制 的错误路径有日志。
- 攻击面控制 的权限检查可被测试覆盖。
- 攻击面控制 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 10 章：存储系统

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 10.0 当前已实证存储基线

当前产品 head 是 ABI-v41/M80 `unified-product-maintenance-plan-runtime`；其存储链经 M79→M78→M77→M76→M75→M74→M73→M72→M71→M70→M69→M68→M67→M66→M65→M64→M63→M62→M61→M60→M58→M57→M56 继承 M55。M55 的 EL0 namespace、strict 4160-byte/最多 8-sector transport、无 DUP/TRANSFER capability 与 `0→4→5→5` 三启动继续保留；M56—M64 完成 recovery、Offline policy、owner/quarantine、durable health hint 与 exact-session clean close，M65 再完成 init-only Prepare/Commit、StorageServer final flush/readback/exit/reap。M66—M71 把该存储路径接入固定 resident graph、真实项目 UI/AppData/PSCI closure 与有界注入式监督；M72 通过 kernel-validated immutable BMF1 VMO 和 init 事务解析决定 StorageServer 初次 spawn/replacement image；M73 再加入只读 UI seal query、kernel-authenticated event loop、两次 clean rotation、budget rearm 与 4-response cancel/drain。M74 在不增加 syscall 或 block/filesystem authority 的前提下，把 BMF1 装入外部 BMS1，用 kernel-pinned RSA-2048 PKCS#1 v1.5 SHA-256 trust anchor 和静态 rollback floor 在 VMO 发布前 fail-closed 校验；M75 增加 DATA 相对 sector 3/4 的双槽持久 floor transaction；M76 再增加有序 fixture keyring 和双槽 `BNDRKEY1`。M77 在同一 pre-EL0 boundary 用独立 fixture root signature-first 校验 BMA1，并精确绑定 manifest、ordered key policy、device 与 maintenance policy；接受记录必须以 exact next sequence 写入 DATA 相对 sector 5/6 的双槽 `BNDRMAU1` SHA-256 hash chain，完成 write/flush/readback 后才发布 prepared evidence。init-only syscall 57 打开一次 boot-local session，并门禁 mutating supervisor report；UI query 保持只读。M78 在 DATA 相对 sector 7/8 增加独立 368-byte 双槽 `BNDRMEX1` completion ledger，以 predecessor completion、精确同授权恢复和 completed-replay 拒绝约束 aggregate completion。M79 不新增 syscall，而是在 DATA 相对 sector 9/10 增加独立 376-byte 双槽 `BNDRMST1`。它固定记录 rotation1、rotation2、drain，逐步绑定精确 authorization、确定性 effect digest 与 SHA-256 chain；pre-EL0 在 audit mutation 前预读三套双槽，空 step ledger 只允许从已完成 M78 sequence-1 迁移，已持久步骤只能只读 reconciliation。terminal drain 必须在 aggregate completion 前重新读取并把 chain head 纳入 runtime digest。专项门证明三个 durable-marker 后宿主 cut 与恢复、损坏最新槽回退/修复，以及坏签名、错误 binding、completed replay 的 pre-EL0 零槽改写拒绝。

M80 继续不新增 syscall，并在 DATA 相对 sector 11/12 增加独立 424-byte 双槽 `BNDRMPL1`。三个固定 resident operation 各有精确 plan ID、operation-instance ID、idempotency key、effect digest 与前一 chain，正常路径严格执行九次 `PREPARED→APPLYING→CONFIRMED` transition；只有尚未 apply 的 PREPARED 可进入 COMPENSATED。pre-EL0 admission 在 audit mutation 前同时预读 audit/execution/step/plan 八个槽。APPLYING 重启若未观察到对应 durable step effect，则发布 result-unknown 并按固定幂等 effect 恢复；若精确 effect 已持久，则按 observed-unconfirmed 只推进确认。专项门证明 normal、prepared/applying/effect 三处宿主 cut 后恢复、cancel-prepared、损坏最新 plan 槽回退/修复与五份终态磁盘收敛；terminal plan chain `1dfb9859db551fe7621e274a518edc937c67feccc23275c35e9e99d098f58ed4` 已绑定进 aggregate completion。

M59 另有两份独立账本。ABI-v24 `app-data-async-recovery-runtime` 是 M54 AppData child：它只注入一次 read QueueNotify loss，冻结 read 的 `RequiresReset` outcome，kernel 绝不重放原 operation；userspace 只允许零输出 `FileOpenAt` 的一次 `Unavailable` retry，mutation、`OutcomeUnknown` 和第二次 `Unavailable` 均不 retry。实测 physical steps/pending/yields=`4/3/5`，已认证同 owner waits/不同 dispatch changes=`2/2`，driver/control mask=`53500/86187<625000`。ABI-v23 timeout 自测不是产品 opt-in leaf；它复用 shared engine，steps/yields=`4/3`、timer/worker windows=`3/3`、timer dispatches=`9`、双 worker work=`539135/376383`、mask=`59375/87188<625000`，并明确 `el0_progress_claim=0`。

shared physical engine 现由 M80 继承的 StorageServer 路径、基础 AppData runtime、历史 M59 AppData fault child 与低层 timeout 自测共同消费；各自 coordinator 保留 policy ownership。物理 commit 从不隐式开放 admission：M55/M56/M57 同步 coordinator 为 `rearm→open→broker`，M59 AppData/timeout 为 `rearm→open→finalize/return`，历史 M58 为 `rearm→broker→open`，历史 M60 则在 ownerless DAIF-masked 窗内执行 `rearm→open→kernel-prearm→broker→success-ledgers→restore-DAIF`；M61 在 fatal completion publication 的独立 DAIF 窗内建立 owner ticket。普通 AppData sector I/O 仍是 IRQ-enabled busy-spin，只有 recovery cooperative；早期 probe/fatal cleanup 仍有同步 reset。M75—M80 rollback/key-policy/authorization/execution/step/plan preflights 位于 `process::init()` 后、`userboot::start()` 前的 IRQ-enabled pre-EL0 窗，不在 IRQ-masked SVC 中执行同步块 I/O；M78 aggregate completion 与 M79/M80 runtime step/plan commits 则位于 kernel 已验证的运行阶段，terminal step/plan 在 aggregate completion 前重读。永久 fault 与 M62 proof-unavailable gate 都是模拟故障；M63 record 只是一条 open hint，不是 hardware identity/Offline authority。M64/M65 的 clean close/orchestration 也只在 QEMU 固定路径证明。M80 仍只有固定服务集合、固定三项 operation 与 proof profile；fixture keyring、双槽 policy/audit/execution/step/plan 不是生产 HSM custody 或 RPMB/eFuse，也不抵抗 host replay/erase/tamper。result-unknown/effect-observed reconciliation 仅证明固定 resident 幂等效果，不是外部副作用 exactly-once 或 arbitrary instruction resume；尚无 hardware power-cut、非注入无限期 watchdog、accepted-session 完整 race、任意 soak、hotplug/device replacement、真实控制器、SMP/IOMMU 或真机恢复。

历史下层仍是 M25 QEMU durable bounded DATA record slice，其下保留 M24 capability-scoped EL0 只读 boot-file/VMO 与 M23 GPT/FAT16/VFS 启动证据。确定性 fixture 为 8388608 bytes/16384 sectors，SHA-256 为 `6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838`；该 SHA 只绑定存储 fixture，不代表 immutable kernel image。sector 0/1 的 FNV-1a64 为 `0xbebd264b8c14cd72` / `0x8294de399174037c`。index 0 `BNDROID_SYS` 仍为 LBA 2048—16350 且只读；private index 1 `BNDROID_DATA` 为 LBA 64—127。最初两请求与后续 273 次 parser read、5 次 persistence read、1 次 WRITE、1 次 FLUSH 都由 IRQ 完成，正常 read/write/flush=`280/1/1`，request/completion/IRQ completion=`282/282/282`，read/write bytes=`143360/512`。两个固定系统文件共 68 bytes 仍一次性缓存到 immutable `BootfsCatalog<2>`/VMO，后续 EL0 open/read 的 runtime disk read 为 0。

#### 已完成

- [x] FDT 32-slot direct-root compatible discovery、GICv2 phandle/SPI 解析与唯一 active coherent block device 选择。
- [x] modern MMIO v2 status/feature negotiation、queue 8、two-frame physical DMA、两个 536-byte request slot 与 IRQ completion；normal 严格要求物理可写、`VERSION_1 | FLUSH` 并拒绝 RO，M22 race build 保留物理 RO。
- [x] generation token、乱序/重复/过期 completion 检查、coherent barrier、stable capacity 与永久 owner 所有权纪律。
- [x] 同时读取 sector 0/1，并固定 `FDT_VIRTIO_OK` 至 `STORAGE_LIMITS` 的 M25 正常成功 marker；前台 used-ring polling fallback 为 0。
- [x] protective MBR，主备 GPT header/entry CRC 与数组镜像，唯一 `BNDROID_SYS` 分区（LBA 2048—16350）。
- [x] private `BNDROID_DATA` entry index 1/LBA 64—127，与 system 不重叠；write API 仅接受该范围，system 和 DATA 外写入拒绝且请求账本不变。
- [x] 双 FAT 镜像验证、FAT16 8.3 root/subdirectory 有界 lookup/cross-cluster read，以及 `/system/HELLO.TXT`、`/system/SYSTEM/BUILD.TXT` 内容证据。
- [x] 11 类 storage negative 覆盖历史 legacy/整镜像/GPT/FAT16/VFS/no-device，并增加 RO、missing FLUSH、坏 DATA superblock、坏 slots 与 generation gap；timeout/reset/late-event 恢复路径也保留。
- [x] M24 `BootfsCatalog<2>`/immutable VMO、move-only `READ|TRANSFER` root capability、ABI v13 `FileOpenAt`/`VmoRead`，以及完整内容/FNV、partial、EOF、bounds、bad-address、rights、stale、WAIT denial 与 zero-payload transfer 证明。
- [x] M25 DATA superblock、两个 512-byte CRC record slot 与绑定 DATA GPT unique GUID 的 16-byte format epoch；双槽都有效时 generation 必须相邻。
- [x] inactive-slot type 1 WRITE 的 IRQ completion 后才发布 type 4 FLUSH，二者都要求 `used.len=1`；FLUSH completion 后重读双槽并保留旧已选槽，fresh generation `0→1`。
- [x] 跨 QEMU 复用同一 raw image 实证 `0→1→2`、DATA 外不变、`changed_data_bytes=98`；破坏最新槽后回退 generation 0 并重新提交 1。
- [x] 完整 M25 `./scripts/test.sh` 从头 exit 0：269 项 host tests（ABI/ELF/SM/shared/kernel=`16/19/31/0/203`）、双 AArch64 Clippy、五个 normal QEMU、M22 race 与全部既有 negative/rollback/fault checks。

#### 严格边界与路线

- [x] M23：512-byte block abstraction、严格 GPT、只读 FAT16 与 `/system` VFS 的启动期 profile。
- [x] M24：固定两文件 immutable boot catalog 与 capability-scoped EL0 open/read；`runtime_disk_reads=0 mapped=0 shared_memory=0`。
- [x] M25：private DATA 范围内的 bounded raw-sector 双槽 record、GUID-bound epoch、有序 WRITE/FLUSH/readback、跨 QEMU persistence 与坏槽回退。
- [x] M26：strict fw_cfg DMA/320×480 XRGB8888 ramfb 与 virtio-input keyboard event path；
  headless screenshot/QMP input acceptance 和 283 项 host tests 通过。
- [x] M27：opaque scene + alpha cursor compositor、mutable dirty redraw、virtio-tablet touch 与 interaction test；297 项 host tests。
- [x] M28（历史）：kernel-owned clickable shell、三个 app target + home、Settings generation-1 damage commit、14-event/6-sample UI screenshot；323 项 host tests。
- [x] M29 protocol+raster/session 地基：canonical 64-byte `bndr-ui` 与严格先全验证后 raster 的纯 `SurfaceSession`。
- [x] M29 运行时接入：ABI v14 syscall 25/26/27、EL0 唯一 capability/session、真实 `KernelFallback→UserspaceBound` 单向切换、ServiceManager 内嵌 SurfaceServer、独立 input event 分类与 ramfb/headless 验收；337 项 host tests，完整套件为 `userspace_surface=1 userspace_ui=1`。
- [x] M30：独立 EL0 SurfaceServer + Launcher；ABI v15 只扩充镜像身份、syscall `0..27` 不变，以单独 UI Channel 完成认证单 Surface IPC；349 项 host tests。
- [x] M31：独立 EL0 App、第二对 UI Channel、显式 focus、按焦点 input、press-to-release capture 与 Present v2 generation/cancellation；ABI v16、默认 359 项 host tests。
- [x] M32：ABI v17 两槽 transferable GraphicsBuffer、`BUP1`、App→SurfaceServer 衰减句柄、generation/scrub、失败原子性与真实 raster/copy-present；默认 377 项 host tests和完整矩阵通过。
- [x] ABI v18：新增 init-only syscall 31 `ProcessTerminate`，覆盖 generation-qualified target、终止原因、Waiting token abandonment 与 reap。
- [x] M33：专用 `app-lifecycle-runtime` profile 完成 canonical 64-byte `ALC1`/`UBP1`/`USC1`、七事务与 generation-safe App replacement；406 项 host tests和完整矩阵通过。
- [x] M34：专用 `app-crash-recovery-runtime` profile 完成 canonical USC1 `OwnerDied`、peer-close 资源/焦点/capture 清理、Killed 回收和 App2→App3 generation-safe restart；411 项 host tests、专用 QEMU checker 与完整矩阵通过。清理后的 input routing 已接线，但 crash-window 输入尚无专门注入验收。
- [x] M35：ABI-v19 mapped/shared GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；App RW/SurfaceServer RO 共享 75 页，BBM+TLBI、EL0 volatile reads、416 host tests 与完整矩阵通过。
- [x] M36：consumer owner-death abandon/release + producer recovery；Acquired QEMU、Queued/Acquired host tests、75 页 EL0 重写与零最终 graphics mappings/handles 已通过。
- [x] M37：SurfaceServer generation-2 restart/rebind + resident mapped App frame recovery；专用 QEMU 与完整从头矩阵均已通过。
- [x] M38：producer-death orphan + two-slot scrub/reuse；dedicated QEMU strict validator、checker、host suite 418 与完整矩阵均已通过。
- [x] M39：ABI-v20 software frame clock/single-grant gated present。
- [x] M40：resident two-buffer software-paced swapchain、ABA commit 与 post-copy release。
- [x] M41：bounded userspace multi-window compositor，证明 z-order、occlusion、damage、raise、focus 与 input capture。
- [x] M42：持久事件驱动窗口会话、phone 边界拒绝、peer-close 清理、generation-safe recreate 与通用事件序列。
- [x] M43：focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44：SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45：独立 bounded InputServer、唯一 InputCapability、capacity-64 kernel broker、broker-only route 与 Surface FIFO 零 fallback；dedicated QEMU、三截图、623 host/334 kernel 与完整总套件均已通过。
- [x] M46：SurfaceServer restart/rebind、route epoch `1→2`、gap release、App capture/contact cancel；dedicated QEMU、M45 regression、650 host/335 kernel 与完整总套件均已通过。
- [x] M47：ABI-v23 `InputSessionInfo`、InputServer 一次 restart/reacquire、fixed 30 ms backoff、Surface route resync 与 InputAcquire permission denial；674 host/338 kernel、pre/gap/post QEMU 与完整总套件均已通过，quarantine 仅 host-verified。
- [x] M48：single-InputServer `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、首个 process-exit 重启、第二次 health-timeout、budget 1 runtime quarantine、degraded UI 与严格 trace/validator/topology；694 host/338 kernel、dedicated QEMU、host/static matrix 与最终全量套件均已通过。
- [x] M49：依赖感知 `ServiceSupervisor::<2>` 同时监督 SurfaceServer+InputServer；`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。
- [x] M50：保持 ABI-v23/syscall 0—41/八镜像/capacity 不变，完成恢复后 phone 外拒绝、App physical `6/7`→两条 BWE→Present sequence 5/frame 3→output frame 4/write generation 4→health `2/2` resident 的 QEMU 封口。
- [x] M51：保持 ABI/镜像/capacity/context 不变，继承 M49/M50 并完成 Launcher physical `8/9` capture/release、focus `App/3→Launcher/4`、Present command 6/frame 4→output frame 5/write generation 5、health `2/2` resident 的离线 QEMU 封口。
- [x] M52：保持 ABI/镜像/capacity/context 不变，继承 M49/M50/M51 并完成 App physical `10/11` capture/release、focus `Launcher/4→App/5`、Present command 6/frame 4→output frame 6/write generation 6 的离线 QEMU roundtrip 封口。
- [x] M53：保持 ABI/镜像/capacity/context 不变，作为 M52 feature child 完成 session 2 的 `Ready→App/1→Launcher/2→App/3` 双 client 实时同步与 RFCK/LFCK/AFCK 收敛；channel=`14/14`、M52 APCK boundary=`1/1`，且不新增 QMP input 或 output frame。
- [x] M54：仅 `app-data-runtime` 为 ABI v24；32-entry/path-64/depth-4/file-4096/live-128-KiB AppData 卷、capability authority、292+965 切点、三次持久启动与坏 checkpoint 回退均由专用四-QEMU checker及 22 项 host tests封口。
- [x] M55：独立 StorageServer ELF、ABI v25 syscall 47—52、strict v2 4160-byte/最多 8-sector request、userspace namespace、kernel raw-sector broker、100 Hz logical + one-shot wake、idle restart/rebind 与三启动 `0→4→5→5` 已封口；第三次 AppData 零 write/flush。
- [x] M56：完成三类受控 session-fatal in-flight failure、kernel-only device reset/re-negotiate/queue rebuild、两阶段 IRQ rearm 与 replacement durable remount。
- [x] M57：完成两轮串行 `WRFWRF`、driver suppression `2/2/2`、IRQ-safe broker access、Running-owner `ServiceAbandoned`、physical submission gate，以及一次 IRQ commit abort 后真正成功 retry。
- [x] M58：把 StorageServer 的七次 physical recovery 拆成四相位 cooperative executor，跨相位释放 driver borrow，以单 attempt id 封闭 IRQ rearm/broker/admission，并证明七个 timer/双 worker/已认证 EL0 progress window、零 masked polling，以及被计量的 driver/control 区间均短于 timer period。
- [x] M59：建立 ABI-v24 AppData fault child 与 ABI-v23 timeout self-test 两份独立账本；共享四相位 physical engine，冻结 AppData outcome 且 kernel 零重放，只允许零输出 `FileOpenAt` 一次 `Unavailable` retry，timeout 明确不声称 EL0 progress。物理 commit 不隐式开门，各 coordinator 显式 rearm/open。
- [x] M60：完成六次瞬态加一次模拟永久 read 的 `WRFWRFR`、ticketed RecoveryPolicy、cap 3、`2×2` backoff、Probation/Healthy、kernel-prearmed 永久故障权限、boot-local sticky Offline、终态 direct IRQ/DMA proof 与 Offline 后零新增 attempt/reset/submission。
- [x] M61：完成 fatal-latched、不可续期 250 ms owner grace，精确 generation-qualified PID/broker epoch/lease-generation monitor retirement，真实 ObjectWait abandonment、ordinary reaper barrier 与 epoch-7 replacement recovery；明确不是通用健康 heartbeat。
- [x] M62：kernel-only terminal quarantine proof deferral、三步/two-Pending fallback 与 reverified IRQ/DMA Offline boundary 已封口。
- [x] M63：双槽 persistent health、legacy upgrade、unclosed hint 与 fresh current-device reprobe 已封口；record 不恢复 Offline。
- [x] M64：exact-session durable close、prior-closed 第二启动与 pre-EL0 no-later-storage seal 已封口。
- [x] M65：ABI-v26 init-only Prepare/Commit、两个 client drain、StorageServer final flush/readback/exit/reap 与 durable close/seal 已封口。
- [x] M66：ABI-v27 八节点/10-edge fixed resident graph、kernel topology proof、三波逆拓扑 quiesce、opaque token 与两次 QEMU-only self-exit 已封口。
- [x] 历史 M71 ABI-v32 五服务连续监督：事务式 5-service/4-edge 目录与批量 probe、21 个 cadence 轮/16 个额外健康轮、同窗两服务瞬态 miss 容忍与独立恢复、一次超限 StorageServer replacement，以及两次 PSCI QEMU self-exit 均已封口；不新增 syscall，明确 `arbitrary_soak_claim=0 real_phone_claim=0`。
- [ ] 下一 P0：推进目录驱动的任意有界服务发现/启动、非注入长期健康循环、背靠背升级故障与任意时长 soak；用户指定并授权目标后，再扩展 BSP/启动链/PMIC、hotplug/device replacement、真实 power-cut、SMP/IOMMU 与真机恢复。

M25/M24 只有有界 record 与两个固定只读启动文件；历史 M54—M66 增加 fixed AppData、EL0 StorageServer、恢复、durable hint/close、resident shutdown 与 QEMU-only exit；M67—M69 再统一真实 UI/InputServer、单服务恢复和固定 App hard dependency；M70 增加 strict FDT+PSCI QEMU self-exit；历史 M71 增加事务式五服务/四依赖目录、批量 probe、16 个额外健康轮、同窗两服务瞬态漏报恢复与一次升级 StorageServer replacement。它仍只有固定 AppData namespace、编译期给定的五服务/四边 witness 和注入式 bounded soak，没有真实 BSP/PMIC/hardware poweroff、通用 POSIX/fd/cache、认证/anti-rollback、多 App policy、真机控制器或掉电恢复；不能描述成可用手机存储系统。

### 10.1 分区布局

存储系统 - 分区布局 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 分区布局 的最小可运行目标。
- [ ] 实现 分区布局 的 Rust 数据结构和错误模型。
- [ ] 接入 分区布局 的日志输出和诊断字段。
- [ ] 为 分区布局 编写单元测试或集成测试。
- [ ] 把 分区布局 接入构建系统和 QEMU 镜像。
- [ ] 建立 分区布局 的验收标准和失败回滚策略。
- [ ] 记录 分区布局 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 分区布局 增加性能指标和安全审计点。

#### 验收标准

- 分区布局 可以独立构建。
- 分区布局 可以在 QEMU 或测试环境中运行。
- 分区布局 的错误路径有日志。
- 分区布局 的权限检查可被测试覆盖。
- 分区布局 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.2 应用目录

存储系统 - 应用目录 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 应用目录 的最小可运行目标。
- [ ] 实现 应用目录 的 Rust 数据结构和错误模型。
- [ ] 接入 应用目录 的日志输出和诊断字段。
- [ ] 为 应用目录 编写单元测试或集成测试。
- [ ] 把 应用目录 接入构建系统和 QEMU 镜像。
- [ ] 建立 应用目录 的验收标准和失败回滚策略。
- [ ] 记录 应用目录 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 应用目录 增加性能指标和安全审计点。

#### 验收标准

- 应用目录 可以独立构建。
- 应用目录 可以在 QEMU 或测试环境中运行。
- 应用目录 的错误路径有日志。
- 应用目录 的权限检查可被测试覆盖。
- 应用目录 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.3 Scoped Storage

存储系统 - Scoped Storage 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Scoped Storage 的最小可运行目标。
- [ ] 实现 Scoped Storage 的 Rust 数据结构和错误模型。
- [ ] 接入 Scoped Storage 的日志输出和诊断字段。
- [ ] 为 Scoped Storage 编写单元测试或集成测试。
- [ ] 把 Scoped Storage 接入构建系统和 QEMU 镜像。
- [ ] 建立 Scoped Storage 的验收标准和失败回滚策略。
- [ ] 记录 Scoped Storage 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Scoped Storage 增加性能指标和安全审计点。

#### 验收标准

- Scoped Storage 可以独立构建。
- Scoped Storage 可以在 QEMU 或测试环境中运行。
- Scoped Storage 的错误路径有日志。
- Scoped Storage 的权限检查可被测试覆盖。
- Scoped Storage 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.4 媒体库

存储系统 - 媒体库 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 媒体库 的最小可运行目标。
- [ ] 实现 媒体库 的 Rust 数据结构和错误模型。
- [ ] 接入 媒体库 的日志输出和诊断字段。
- [ ] 为 媒体库 编写单元测试或集成测试。
- [ ] 把 媒体库 接入构建系统和 QEMU 镜像。
- [ ] 建立 媒体库 的验收标准和失败回滚策略。
- [ ] 记录 媒体库 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 媒体库 增加性能指标和安全审计点。

#### 验收标准

- 媒体库 可以独立构建。
- 媒体库 可以在 QEMU 或测试环境中运行。
- 媒体库 的错误路径有日志。
- 媒体库 的权限检查可被测试覆盖。
- 媒体库 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.5 备份恢复

存储系统 - 备份恢复 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 备份恢复 的最小可运行目标。
- [ ] 实现 备份恢复 的 Rust 数据结构和错误模型。
- [ ] 接入 备份恢复 的日志输出和诊断字段。
- [ ] 为 备份恢复 编写单元测试或集成测试。
- [ ] 把 备份恢复 接入构建系统和 QEMU 镜像。
- [ ] 建立 备份恢复 的验收标准和失败回滚策略。
- [ ] 记录 备份恢复 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 备份恢复 增加性能指标和安全审计点。

#### 验收标准

- 备份恢复 可以独立构建。
- 备份恢复 可以在 QEMU 或测试环境中运行。
- 备份恢复 的错误路径有日志。
- 备份恢复 的权限检查可被测试覆盖。
- 备份恢复 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.6 加密

存储系统 - 加密 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 加密 的最小可运行目标。
- [ ] 实现 加密 的 Rust 数据结构和错误模型。
- [ ] 接入 加密 的日志输出和诊断字段。
- [ ] 为 加密 编写单元测试或集成测试。
- [ ] 把 加密 接入构建系统和 QEMU 镜像。
- [ ] 建立 加密 的验收标准和失败回滚策略。
- [ ] 记录 加密 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 加密 增加性能指标和安全审计点。

#### 验收标准

- 加密 可以独立构建。
- 加密 可以在 QEMU 或测试环境中运行。
- 加密 的错误路径有日志。
- 加密 的权限检查可被测试覆盖。
- 加密 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.7 配额

存储系统 - 配额 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 配额 的最小可运行目标。
- [ ] 实现 配额 的 Rust 数据结构和错误模型。
- [ ] 接入 配额 的日志输出和诊断字段。
- [ ] 为 配额 编写单元测试或集成测试。
- [ ] 把 配额 接入构建系统和 QEMU 镜像。
- [ ] 建立 配额 的验收标准和失败回滚策略。
- [ ] 记录 配额 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 配额 增加性能指标和安全审计点。

#### 验收标准

- 配额 可以独立构建。
- 配额 可以在 QEMU 或测试环境中运行。
- 配额 的错误路径有日志。
- 配额 的权限检查可被测试覆盖。
- 配额 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 10.8 清理策略

存储系统 - 清理策略 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 清理策略 的最小可运行目标。
- [ ] 实现 清理策略 的 Rust 数据结构和错误模型。
- [ ] 接入 清理策略 的日志输出和诊断字段。
- [ ] 为 清理策略 编写单元测试或集成测试。
- [ ] 把 清理策略 接入构建系统和 QEMU 镜像。
- [ ] 建立 清理策略 的验收标准和失败回滚策略。
- [ ] 记录 清理策略 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 清理策略 增加性能指标和安全审计点。

#### 验收标准

- 清理策略 可以独立构建。
- 清理策略 可以在 QEMU 或测试环境中运行。
- 清理策略 的错误路径有日志。
- 清理策略 的权限检查可被测试覆盖。
- 清理策略 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 11 章：网络通信

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 11.1 TCP/IP

网络通信 - TCP/IP 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 TCP/IP 的最小可运行目标。
- [ ] 实现 TCP/IP 的 Rust 数据结构和错误模型。
- [ ] 接入 TCP/IP 的日志输出和诊断字段。
- [ ] 为 TCP/IP 编写单元测试或集成测试。
- [ ] 把 TCP/IP 接入构建系统和 QEMU 镜像。
- [ ] 建立 TCP/IP 的验收标准和失败回滚策略。
- [ ] 记录 TCP/IP 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 TCP/IP 增加性能指标和安全审计点。

#### 验收标准

- TCP/IP 可以独立构建。
- TCP/IP 可以在 QEMU 或测试环境中运行。
- TCP/IP 的错误路径有日志。
- TCP/IP 的权限检查可被测试覆盖。
- TCP/IP 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.2 DNS

网络通信 - DNS 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 DNS 的最小可运行目标。
- [ ] 实现 DNS 的 Rust 数据结构和错误模型。
- [ ] 接入 DNS 的日志输出和诊断字段。
- [ ] 为 DNS 编写单元测试或集成测试。
- [ ] 把 DNS 接入构建系统和 QEMU 镜像。
- [ ] 建立 DNS 的验收标准和失败回滚策略。
- [ ] 记录 DNS 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 DNS 增加性能指标和安全审计点。

#### 验收标准

- DNS 可以独立构建。
- DNS 可以在 QEMU 或测试环境中运行。
- DNS 的错误路径有日志。
- DNS 的权限检查可被测试覆盖。
- DNS 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.3 Wi-Fi

网络通信 - Wi-Fi 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Wi-Fi 的最小可运行目标。
- [ ] 实现 Wi-Fi 的 Rust 数据结构和错误模型。
- [ ] 接入 Wi-Fi 的日志输出和诊断字段。
- [ ] 为 Wi-Fi 编写单元测试或集成测试。
- [ ] 把 Wi-Fi 接入构建系统和 QEMU 镜像。
- [ ] 建立 Wi-Fi 的验收标准和失败回滚策略。
- [ ] 记录 Wi-Fi 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Wi-Fi 增加性能指标和安全审计点。

#### 验收标准

- Wi-Fi 可以独立构建。
- Wi-Fi 可以在 QEMU 或测试环境中运行。
- Wi-Fi 的错误路径有日志。
- Wi-Fi 的权限检查可被测试覆盖。
- Wi-Fi 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.4 蜂窝

网络通信 - 蜂窝 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 蜂窝 的最小可运行目标。
- [ ] 实现 蜂窝 的 Rust 数据结构和错误模型。
- [ ] 接入 蜂窝 的日志输出和诊断字段。
- [ ] 为 蜂窝 编写单元测试或集成测试。
- [ ] 把 蜂窝 接入构建系统和 QEMU 镜像。
- [ ] 建立 蜂窝 的验收标准和失败回滚策略。
- [ ] 记录 蜂窝 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 蜂窝 增加性能指标和安全审计点。

#### 验收标准

- 蜂窝 可以独立构建。
- 蜂窝 可以在 QEMU 或测试环境中运行。
- 蜂窝 的错误路径有日志。
- 蜂窝 的权限检查可被测试覆盖。
- 蜂窝 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.5 VPN

网络通信 - VPN 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 VPN 的最小可运行目标。
- [ ] 实现 VPN 的 Rust 数据结构和错误模型。
- [ ] 接入 VPN 的日志输出和诊断字段。
- [ ] 为 VPN 编写单元测试或集成测试。
- [ ] 把 VPN 接入构建系统和 QEMU 镜像。
- [ ] 建立 VPN 的验收标准和失败回滚策略。
- [ ] 记录 VPN 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 VPN 增加性能指标和安全审计点。

#### 验收标准

- VPN 可以独立构建。
- VPN 可以在 QEMU 或测试环境中运行。
- VPN 的错误路径有日志。
- VPN 的权限检查可被测试覆盖。
- VPN 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.6 代理

网络通信 - 代理 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 代理 的最小可运行目标。
- [ ] 实现 代理 的 Rust 数据结构和错误模型。
- [ ] 接入 代理 的日志输出和诊断字段。
- [ ] 为 代理 编写单元测试或集成测试。
- [ ] 把 代理 接入构建系统和 QEMU 镜像。
- [ ] 建立 代理 的验收标准和失败回滚策略。
- [ ] 记录 代理 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 代理 增加性能指标和安全审计点。

#### 验收标准

- 代理 可以独立构建。
- 代理 可以在 QEMU 或测试环境中运行。
- 代理 的错误路径有日志。
- 代理 的权限检查可被测试覆盖。
- 代理 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.7 防火墙

网络通信 - 防火墙 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 防火墙 的最小可运行目标。
- [ ] 实现 防火墙 的 Rust 数据结构和错误模型。
- [ ] 接入 防火墙 的日志输出和诊断字段。
- [ ] 为 防火墙 编写单元测试或集成测试。
- [ ] 把 防火墙 接入构建系统和 QEMU 镜像。
- [ ] 建立 防火墙 的验收标准和失败回滚策略。
- [ ] 记录 防火墙 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 防火墙 增加性能指标和安全审计点。

#### 验收标准

- 防火墙 可以独立构建。
- 防火墙 可以在 QEMU 或测试环境中运行。
- 防火墙 的错误路径有日志。
- 防火墙 的权限检查可被测试覆盖。
- 防火墙 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 11.8 Android Connectivity Bridge

网络通信 - Android Connectivity Bridge 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Android Connectivity Bridge 的最小可运行目标。
- [ ] 实现 Android Connectivity Bridge 的 Rust 数据结构和错误模型。
- [ ] 接入 Android Connectivity Bridge 的日志输出和诊断字段。
- [ ] 为 Android Connectivity Bridge 编写单元测试或集成测试。
- [ ] 把 Android Connectivity Bridge 接入构建系统和 QEMU 镜像。
- [ ] 建立 Android Connectivity Bridge 的验收标准和失败回滚策略。
- [ ] 记录 Android Connectivity Bridge 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Android Connectivity Bridge 增加性能指标和安全审计点。

#### 验收标准

- Android Connectivity Bridge 可以独立构建。
- Android Connectivity Bridge 可以在 QEMU 或测试环境中运行。
- Android Connectivity Bridge 的错误路径有日志。
- Android Connectivity Bridge 的权限检查可被测试覆盖。
- Android Connectivity Bridge 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 12 章：多媒体

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 12.1 AudioServer

多媒体 - AudioServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 AudioServer 的最小可运行目标。
- [ ] 实现 AudioServer 的 Rust 数据结构和错误模型。
- [ ] 接入 AudioServer 的日志输出和诊断字段。
- [ ] 为 AudioServer 编写单元测试或集成测试。
- [ ] 把 AudioServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 AudioServer 的验收标准和失败回滚策略。
- [ ] 记录 AudioServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 AudioServer 增加性能指标和安全审计点。

#### 验收标准

- AudioServer 可以独立构建。
- AudioServer 可以在 QEMU 或测试环境中运行。
- AudioServer 的错误路径有日志。
- AudioServer 的权限检查可被测试覆盖。
- AudioServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.2 MediaCodec

多媒体 - MediaCodec 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 MediaCodec 的最小可运行目标。
- [ ] 实现 MediaCodec 的 Rust 数据结构和错误模型。
- [ ] 接入 MediaCodec 的日志输出和诊断字段。
- [ ] 为 MediaCodec 编写单元测试或集成测试。
- [ ] 把 MediaCodec 接入构建系统和 QEMU 镜像。
- [ ] 建立 MediaCodec 的验收标准和失败回滚策略。
- [ ] 记录 MediaCodec 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 MediaCodec 增加性能指标和安全审计点。

#### 验收标准

- MediaCodec 可以独立构建。
- MediaCodec 可以在 QEMU 或测试环境中运行。
- MediaCodec 的错误路径有日志。
- MediaCodec 的权限检查可被测试覆盖。
- MediaCodec 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.3 CameraServer

多媒体 - CameraServer 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 CameraServer 的最小可运行目标。
- [ ] 实现 CameraServer 的 Rust 数据结构和错误模型。
- [ ] 接入 CameraServer 的日志输出和诊断字段。
- [ ] 为 CameraServer 编写单元测试或集成测试。
- [ ] 把 CameraServer 接入构建系统和 QEMU 镜像。
- [ ] 建立 CameraServer 的验收标准和失败回滚策略。
- [ ] 记录 CameraServer 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 CameraServer 增加性能指标和安全审计点。

#### 验收标准

- CameraServer 可以独立构建。
- CameraServer 可以在 QEMU 或测试环境中运行。
- CameraServer 的错误路径有日志。
- CameraServer 的权限检查可被测试覆盖。
- CameraServer 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.4 录音

多媒体 - 录音 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 录音 的最小可运行目标。
- [ ] 实现 录音 的 Rust 数据结构和错误模型。
- [ ] 接入 录音 的日志输出和诊断字段。
- [ ] 为 录音 编写单元测试或集成测试。
- [ ] 把 录音 接入构建系统和 QEMU 镜像。
- [ ] 建立 录音 的验收标准和失败回滚策略。
- [ ] 记录 录音 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 录音 增加性能指标和安全审计点。

#### 验收标准

- 录音 可以独立构建。
- 录音 可以在 QEMU 或测试环境中运行。
- 录音 的错误路径有日志。
- 录音 的权限检查可被测试覆盖。
- 录音 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.5 视频播放

多媒体 - 视频播放 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 视频播放 的最小可运行目标。
- [ ] 实现 视频播放 的 Rust 数据结构和错误模型。
- [ ] 接入 视频播放 的日志输出和诊断字段。
- [ ] 为 视频播放 编写单元测试或集成测试。
- [ ] 把 视频播放 接入构建系统和 QEMU 镜像。
- [ ] 建立 视频播放 的验收标准和失败回滚策略。
- [ ] 记录 视频播放 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 视频播放 增加性能指标和安全审计点。

#### 验收标准

- 视频播放 可以独立构建。
- 视频播放 可以在 QEMU 或测试环境中运行。
- 视频播放 的错误路径有日志。
- 视频播放 的权限检查可被测试覆盖。
- 视频播放 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.6 硬解

多媒体 - 硬解 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 硬解 的最小可运行目标。
- [ ] 实现 硬解 的 Rust 数据结构和错误模型。
- [ ] 接入 硬解 的日志输出和诊断字段。
- [ ] 为 硬解 编写单元测试或集成测试。
- [ ] 把 硬解 接入构建系统和 QEMU 镜像。
- [ ] 建立 硬解 的验收标准和失败回滚策略。
- [ ] 记录 硬解 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 硬解 增加性能指标和安全审计点。

#### 验收标准

- 硬解 可以独立构建。
- 硬解 可以在 QEMU 或测试环境中运行。
- 硬解 的错误路径有日志。
- 硬解 的权限检查可被测试覆盖。
- 硬解 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.7 蓝牙音频

多媒体 - 蓝牙音频 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 蓝牙音频 的最小可运行目标。
- [ ] 实现 蓝牙音频 的 Rust 数据结构和错误模型。
- [ ] 接入 蓝牙音频 的日志输出和诊断字段。
- [ ] 为 蓝牙音频 编写单元测试或集成测试。
- [ ] 把 蓝牙音频 接入构建系统和 QEMU 镜像。
- [ ] 建立 蓝牙音频 的验收标准和失败回滚策略。
- [ ] 记录 蓝牙音频 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 蓝牙音频 增加性能指标和安全审计点。

#### 验收标准

- 蓝牙音频 可以独立构建。
- 蓝牙音频 可以在 QEMU 或测试环境中运行。
- 蓝牙音频 的错误路径有日志。
- 蓝牙音频 的权限检查可被测试覆盖。
- 蓝牙音频 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 12.8 Android 多媒体兼容

多媒体 - Android 多媒体兼容 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Android 多媒体兼容 的最小可运行目标。
- [ ] 实现 Android 多媒体兼容 的 Rust 数据结构和错误模型。
- [ ] 接入 Android 多媒体兼容 的日志输出和诊断字段。
- [ ] 为 Android 多媒体兼容 编写单元测试或集成测试。
- [ ] 把 Android 多媒体兼容 接入构建系统和 QEMU 镜像。
- [ ] 建立 Android 多媒体兼容 的验收标准和失败回滚策略。
- [ ] 记录 Android 多媒体兼容 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Android 多媒体兼容 增加性能指标和安全审计点。

#### 验收标准

- Android 多媒体兼容 可以独立构建。
- Android 多媒体兼容 可以在 QEMU 或测试环境中运行。
- Android 多媒体兼容 的错误路径有日志。
- Android 多媒体兼容 的权限检查可被测试覆盖。
- Android 多媒体兼容 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 13 章：开发工具链

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 13.1 CLI

开发工具链 - CLI 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 CLI 的最小可运行目标。
- [ ] 实现 CLI 的 Rust 数据结构和错误模型。
- [ ] 接入 CLI 的日志输出和诊断字段。
- [ ] 为 CLI 编写单元测试或集成测试。
- [ ] 把 CLI 接入构建系统和 QEMU 镜像。
- [ ] 建立 CLI 的验收标准和失败回滚策略。
- [ ] 记录 CLI 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 CLI 增加性能指标和安全审计点。

#### 验收标准

- CLI 可以独立构建。
- CLI 可以在 QEMU 或测试环境中运行。
- CLI 的错误路径有日志。
- CLI 的权限检查可被测试覆盖。
- CLI 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.2 SDK

开发工具链 - SDK 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 SDK 的最小可运行目标。
- [ ] 实现 SDK 的 Rust 数据结构和错误模型。
- [ ] 接入 SDK 的日志输出和诊断字段。
- [ ] 为 SDK 编写单元测试或集成测试。
- [ ] 把 SDK 接入构建系统和 QEMU 镜像。
- [ ] 建立 SDK 的验收标准和失败回滚策略。
- [ ] 记录 SDK 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 SDK 增加性能指标和安全审计点。

#### 验收标准

- SDK 可以独立构建。
- SDK 可以在 QEMU 或测试环境中运行。
- SDK 的错误路径有日志。
- SDK 的权限检查可被测试覆盖。
- SDK 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.3 模拟器

开发工具链 - 模拟器 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 模拟器 的最小可运行目标。
- [ ] 实现 模拟器 的 Rust 数据结构和错误模型。
- [ ] 接入 模拟器 的日志输出和诊断字段。
- [ ] 为 模拟器 编写单元测试或集成测试。
- [ ] 把 模拟器 接入构建系统和 QEMU 镜像。
- [ ] 建立 模拟器 的验收标准和失败回滚策略。
- [ ] 记录 模拟器 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 模拟器 增加性能指标和安全审计点。

#### 验收标准

- 模拟器 可以独立构建。
- 模拟器 可以在 QEMU 或测试环境中运行。
- 模拟器 的错误路径有日志。
- 模拟器 的权限检查可被测试覆盖。
- 模拟器 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.4 调试器

开发工具链 - 调试器 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 调试器 的最小可运行目标。
- [ ] 实现 调试器 的 Rust 数据结构和错误模型。
- [ ] 接入 调试器 的日志输出和诊断字段。
- [ ] 为 调试器 编写单元测试或集成测试。
- [ ] 把 调试器 接入构建系统和 QEMU 镜像。
- [ ] 建立 调试器 的验收标准和失败回滚策略。
- [ ] 记录 调试器 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 调试器 增加性能指标和安全审计点。

#### 验收标准

- 调试器 可以独立构建。
- 调试器 可以在 QEMU 或测试环境中运行。
- 调试器 的错误路径有日志。
- 调试器 的权限检查可被测试覆盖。
- 调试器 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.5 日志

开发工具链 - 日志 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 日志 的最小可运行目标。
- [ ] 实现 日志 的 Rust 数据结构和错误模型。
- [ ] 接入 日志 的日志输出和诊断字段。
- [ ] 为 日志 编写单元测试或集成测试。
- [ ] 把 日志 接入构建系统和 QEMU 镜像。
- [ ] 建立 日志 的验收标准和失败回滚策略。
- [ ] 记录 日志 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 日志 增加性能指标和安全审计点。

#### 验收标准

- 日志 可以独立构建。
- 日志 可以在 QEMU 或测试环境中运行。
- 日志 的错误路径有日志。
- 日志 的权限检查可被测试覆盖。
- 日志 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.6 性能分析

开发工具链 - 性能分析 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 性能分析 的最小可运行目标。
- [ ] 实现 性能分析 的 Rust 数据结构和错误模型。
- [ ] 接入 性能分析 的日志输出和诊断字段。
- [ ] 为 性能分析 编写单元测试或集成测试。
- [ ] 把 性能分析 接入构建系统和 QEMU 镜像。
- [ ] 建立 性能分析 的验收标准和失败回滚策略。
- [ ] 记录 性能分析 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 性能分析 增加性能指标和安全审计点。

#### 验收标准

- 性能分析 可以独立构建。
- 性能分析 可以在 QEMU 或测试环境中运行。
- 性能分析 的错误路径有日志。
- 性能分析 的权限检查可被测试覆盖。
- 性能分析 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.7 UI Inspector

开发工具链 - UI Inspector 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 UI Inspector 的最小可运行目标。
- [ ] 实现 UI Inspector 的 Rust 数据结构和错误模型。
- [ ] 接入 UI Inspector 的日志输出和诊断字段。
- [ ] 为 UI Inspector 编写单元测试或集成测试。
- [ ] 把 UI Inspector 接入构建系统和 QEMU 镜像。
- [ ] 建立 UI Inspector 的验收标准和失败回滚策略。
- [ ] 记录 UI Inspector 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 UI Inspector 增加性能指标和安全审计点。

#### 验收标准

- UI Inspector 可以独立构建。
- UI Inspector 可以在 QEMU 或测试环境中运行。
- UI Inspector 的错误路径有日志。
- UI Inspector 的权限检查可被测试覆盖。
- UI Inspector 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 13.8 兼容性测试

开发工具链 - 兼容性测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 兼容性测试 的最小可运行目标。
- [ ] 实现 兼容性测试 的 Rust 数据结构和错误模型。
- [ ] 接入 兼容性测试 的日志输出和诊断字段。
- [ ] 为 兼容性测试 编写单元测试或集成测试。
- [ ] 把 兼容性测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 兼容性测试 的验收标准和失败回滚策略。
- [ ] 记录 兼容性测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 兼容性测试 增加性能指标和安全审计点。

#### 验收标准

- 兼容性测试 可以独立构建。
- 兼容性测试 可以在 QEMU 或测试环境中运行。
- 兼容性测试 的错误路径有日志。
- 兼容性测试 的权限检查可被测试覆盖。
- 兼容性测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 14 章：测试体系

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 14.1 内核测试

测试体系 - 内核测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 内核测试 的最小可运行目标。
- [ ] 实现 内核测试 的 Rust 数据结构和错误模型。
- [ ] 接入 内核测试 的日志输出和诊断字段。
- [ ] 为 内核测试 编写单元测试或集成测试。
- [ ] 把 内核测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 内核测试 的验收标准和失败回滚策略。
- [ ] 记录 内核测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 内核测试 增加性能指标和安全审计点。

#### 验收标准

- 内核测试 可以独立构建。
- 内核测试 可以在 QEMU 或测试环境中运行。
- 内核测试 的错误路径有日志。
- 内核测试 的权限检查可被测试覆盖。
- 内核测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.2 服务测试

测试体系 - 服务测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 服务测试 的最小可运行目标。
- [ ] 实现 服务测试 的 Rust 数据结构和错误模型。
- [ ] 接入 服务测试 的日志输出和诊断字段。
- [ ] 为 服务测试 编写单元测试或集成测试。
- [ ] 把 服务测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 服务测试 的验收标准和失败回滚策略。
- [ ] 记录 服务测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 服务测试 增加性能指标和安全审计点。

#### 验收标准

- 服务测试 可以独立构建。
- 服务测试 可以在 QEMU 或测试环境中运行。
- 服务测试 的错误路径有日志。
- 服务测试 的权限检查可被测试覆盖。
- 服务测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.3 图形测试

测试体系 - 图形测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 图形测试 的最小可运行目标。
- [ ] 实现 图形测试 的 Rust 数据结构和错误模型。
- [ ] 接入 图形测试 的日志输出和诊断字段。
- [ ] 为 图形测试 编写单元测试或集成测试。
- [ ] 把 图形测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 图形测试 的验收标准和失败回滚策略。
- [ ] 记录 图形测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 图形测试 增加性能指标和安全审计点。

#### 验收标准

- 图形测试 可以独立构建。
- 图形测试 可以在 QEMU 或测试环境中运行。
- 图形测试 的错误路径有日志。
- 图形测试 的权限检查可被测试覆盖。
- 图形测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.4 Android 兼容测试

测试体系 - Android 兼容测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Android 兼容测试 的最小可运行目标。
- [ ] 实现 Android 兼容测试 的 Rust 数据结构和错误模型。
- [ ] 接入 Android 兼容测试 的日志输出和诊断字段。
- [ ] 为 Android 兼容测试 编写单元测试或集成测试。
- [ ] 把 Android 兼容测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 Android 兼容测试 的验收标准和失败回滚策略。
- [ ] 记录 Android 兼容测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Android 兼容测试 增加性能指标和安全审计点。

#### 验收标准

- Android 兼容测试 可以独立构建。
- Android 兼容测试 可以在 QEMU 或测试环境中运行。
- Android 兼容测试 的错误路径有日志。
- Android 兼容测试 的权限检查可被测试覆盖。
- Android 兼容测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.5 压力测试

测试体系 - 压力测试 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 压力测试 的最小可运行目标。
- [ ] 实现 压力测试 的 Rust 数据结构和错误模型。
- [ ] 接入 压力测试 的日志输出和诊断字段。
- [ ] 为 压力测试 编写单元测试或集成测试。
- [ ] 把 压力测试 接入构建系统和 QEMU 镜像。
- [ ] 建立 压力测试 的验收标准和失败回滚策略。
- [ ] 记录 压力测试 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 压力测试 增加性能指标和安全审计点。

#### 验收标准

- 压力测试 可以独立构建。
- 压力测试 可以在 QEMU 或测试环境中运行。
- 压力测试 的错误路径有日志。
- 压力测试 的权限检查可被测试覆盖。
- 压力测试 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.6 Fuzz

测试体系 - Fuzz 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 Fuzz 的最小可运行目标。
- [ ] 实现 Fuzz 的 Rust 数据结构和错误模型。
- [ ] 接入 Fuzz 的日志输出和诊断字段。
- [ ] 为 Fuzz 编写单元测试或集成测试。
- [ ] 把 Fuzz 接入构建系统和 QEMU 镜像。
- [ ] 建立 Fuzz 的验收标准和失败回滚策略。
- [ ] 记录 Fuzz 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 Fuzz 增加性能指标和安全审计点。

#### 验收标准

- Fuzz 可以独立构建。
- Fuzz 可以在 QEMU 或测试环境中运行。
- Fuzz 的错误路径有日志。
- Fuzz 的权限检查可被测试覆盖。
- Fuzz 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 14.7 性能回归

测试体系 - 性能回归 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 性能回归 的最小可运行目标。
- [ ] 实现 性能回归 的 Rust 数据结构和错误模型。
- [ ] 接入 性能回归 的日志输出和诊断字段。
- [ ] 为 性能回归 编写单元测试或集成测试。
- [ ] 把 性能回归 接入构建系统和 QEMU 镜像。
- [ ] 建立 性能回归 的验收标准和失败回滚策略。
- [ ] 记录 性能回归 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 性能回归 增加性能指标和安全审计点。

#### 验收标准

- 性能回归 可以独立构建。
- 性能回归 可以在 QEMU 或测试环境中运行。
- 性能回归 的错误路径有日志。
- 性能回归 的权限检查可被测试覆盖。
- 性能回归 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 第 15 章：路线图

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

Rust 在这里提供的是长期维护能力。内核对象、系统服务状态机、包安装事务、权限授权记录、窗口层级、Surface 生命周期、Android 服务桥接都可以用强类型建模，减少隐式约定和野指针风险。

落地顺序需要保持务实。先跑 QEMU，再跑用户态；先有 framebuffer，再有 compositor；先有原生 Hello World，再有 AndroidBox；先跑简单 APK，再补复杂 Framework API；先有日志和测试，再做性能优化。

本章建议采用“先最小闭环、再模块扩展、最后兼容优化”的实现顺序。每个能力都要能在开发机上构建，在模拟器里运行，在日志系统中定位，在测试体系里回归。

### 15.0 当前落点与下一阶段

- [x] M20 服务层：两条独立常驻 Client session、一次 stalled-secondary revoke/reattach/stale-lease 隔离及精确五进程空闲图；仍是 `general_runtime=0` 的固定切片。
- [x] M21 存储层：FDT 32-slot discovery、modern virtio-mmio v2 `VERSION_1 | RO`、queue 8/two-frame DMA、只读双 sector polling 与三项负测；M20 服务账本逐字段保持不变。
- [x] M22：GICv2 interrupt-parent/SPI、INTID 79 IRQ completion、两个 generation-qualified outstanding request 与 timeout/reset/late-event 恢复。
- [x] M23：512-byte block layer、主备 GPT/CRC、单一只读 FAT16 分区与 `/system` VFS 启动期验证。
- [x] M24：两文件 immutable boot catalog、capability-scoped EL0 `FileOpenAt`/`VmoRead` 与 zero-runtime-disk-read 证明。
- [x] M25：private DATA 双槽 CRC record、16-byte GUID-bound epoch、相邻 generation、有序 WRITE/FLUSH/readback、跨 QEMU `0→1→2` 与坏最新槽回退。
- [x] M26：strict fw_cfg DMA/ramfb static splash 与 virtio-input keyboard；
  `check-framebuffer.sh`/`check-input.sh` 和完整 test matrix 通过。
- [x] M27：最小双层 compositor、mutable redraw、pointer/touch input 与 interaction test。
- [x] M28（历史）：kernel-owned clickable shell、tap capture/hit-test、Settings damage commit 与 full-frame screenshot；当时为 `owner=kernel userspace_surface=0`。
- [x] M29：ABI v14 EL0-owned single Surface、唯一 capability/session、真实单向 userspace ownership、input FIFO/coalescing 与 ramfb/headless 验收；337 项 host tests；该历史阶段 handles 总计 17、manager wait items 为 5。
- [x] M30：独立 EL0 SurfaceServer + Launcher 的认证单 Surface IPC；ABI v15 不改 syscall `0..27`，七进程、18 endpoint/9 pair、19 handle，349 项 host tests，完整 `./scripts/test.sh` 从头 exit 0。
- [x] M31：独立 EL0 App 与双 UI Channel；ABI v16 不改 syscall `0..27`，八进程、20 endpoint/10 pair、21 handle，默认 359 项 host tests。
- [x] M32：ABI v17 transferable GraphicsBuffer/buffer-present；八进程、20 endpoint/10 pair、23 handle，默认 377 项 host tests与完整矩阵通过。
- [x] M33：ABI v18 专用 `app-lifecycle-runtime` profile 完成通用 app lifecycle 与最小窗口端点管理；七事务、ALC/USC=`35/14`、USC operations=`2/3/1/1`，App1 正常退出后 App2 同 slot 下一 PID generation，最终 29 handle、26 endpoint/13 pair、五对窗口通道和 wait=`8`（many/array=`2/6`）。
- [x] M34：ABI v18 专用 `app-crash-recovery-runtime` profile 完成十事务 owner-death/crash restart；ALC message/transaction/state/crash=`46/10/19/1`，USC command/ack/owner-death=`9/9/1`、operations=`3/4/1/1`，App PID generation=`1→2→3`，process=`11/3/3/8`、reasons=`2/0/1`、terminate=`1/1`、graphics generation=3，最终 29 handle、26 endpoint/13 pair、wait=`8`、abandoned=`0/0/1`。
- [x] M35：ABI-v19 mapped/shared GraphicsBuffer、单槽 BufferQueue 与 acquire/release fence；`map=4/4 queue=4/4 acquire=4/4 releases=4 copy_writes=0/0`。
- [x] M36：consumer owner-death cleanup 与 producer recovery；`owner=1/1/0/1/1/0/1`、nonzero wake、两次 mapping removal 与零最终 graphics resource。
- [x] M37：SurfaceServer generation-2 restart/rebind、Launcher/App endpoint replacement 与 resident mapped generation-2 frame present；`scripts/check-graphics-surface-restart.sh` 已通过。
- [x] M38：producer-death orphan、两个完整 backing scrub/zero proof 与 two-slot safe reuse；`scripts/check-graphics-producer-orphan.sh` 已通过。
- [x] M39：ABI-v20 software frame clock/single-grant gated present。
- [x] M40：fixed two-buffer ownership/scheduling 与最终 B3 Acquired 证据。
- [x] M41：bounded multi-window z-order/occlusion/damage/input-routing proof。
- [x] M42：persistent window session、phone bounds、peer-close cleanup、generation-safe recreate 与通用事件序列。
- [x] M43：focus-scoped hardware keyboard 路由与有界 UTF-8 text-editor slice；完整矩阵已通过。
- [x] M44：SurfaceServer 内三键 soft keyboard、可信 nonfocusable overlay、隐藏 hit 拒绝与 focus-preserving editor；dedicated QEMU、三截图、host 与完整总套件均已通过。
- [x] M45：独立 bounded InputServer、唯一 InputCapability、capacity-64 kernel broker、broker-only route 与 Surface FIFO 零 fallback；dedicated QEMU、三截图、623 host/334 kernel 与完整总套件均已通过。
- [x] M46：SurfaceServer restart/rebind、route epoch `1→2`、gap release、App capture/contact cancel；dedicated QEMU、M45 regression、650 host/335 kernel 与完整总套件均已通过。
- [x] M47：ABI-v23 `InputSessionInfo`、InputServer 一次 restart/reacquire、fixed 30 ms backoff、Surface route resync 与 InputAcquire permission denial；674 host/338 kernel、pre/gap/post QEMU 与完整总套件均已通过，quarantine 仅 host-verified。
- [x] M48：single-InputServer `ServiceSupervisor::<1>`、strict fixed-64-byte `BSH1`、首个 process-exit 重启、第二次 health-timeout、budget 1 runtime quarantine、degraded UI 与严格 trace/validator/topology；694 host/338 kernel、dedicated QEMU、host/static matrix 与最终全量套件均已通过。
- [x] M49：依赖感知 `ServiceSupervisor::<2>` 同时监督 SurfaceServer+InputServer；`InputServer -> SurfaceServer` soft edge、Surface generation `1→2`、Input 100 ms health-timeout、fixed 30 ms backoff、degraded→recovered 与 restart-storm 防护均已由五阶段 QEMU 证据封口，ABI 保持 v23。
- [x] M50：恢复后 phone 外拒绝、App physical `6/7`、Present 与 health resident 闭环。
- [x] M51：恢复后 Launcher physical `8/9` capture/focus/Present 闭环。
- [x] M52：恢复后 App physical `10/11` capture、`Launcher/4→App/5` focus roundtrip 与 Present 闭环。
- [x] M53：session 2 `Ready→App/1→Launcher/2→App/3` 双 client 实时同步、RFCK/LFCK/AFCK ACK 收敛与零新增 input/frame 封口。
- [x] M54：ABI v24 opt-in capability-scoped AppData；双 checkpoint/双 bank、created/upgraded/stable、坏最新 checkpoint 回退、292+965 切点与 authority seal 均通过，完整 suite marker 包含 `app_data_runtime=1 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M55：独立 ABI-v25 StorageServer ELF、syscall 47—52、strict v2 4160-byte/最多 8-sector transport、non-DUP/non-TRANSFER volume/session、userspace namespace、100 Hz logical + physical one-shot scheduling、idle restart/rebind 与 `0→4→5→5` 三启动均封口；第三次 AppData 零 write/flush。release 三启动、严格 Clippy/host/feature matrix 与 2026-07-17 完整 `./scripts/test.sh` 均已通过，终态含 `storage_server_static=1 storage_server_runtime_boots=3 storage_negative_cases=11 persistence_reboot=1 recovery=1 irq_race=1`。
- [x] M56：ABI-v25 不变；三类一次性 QueueNotify 抑制封口 session-fatal read/write/flush timeout、old-owner cleanup/unbind-before-reset、kernel-only status-0 reset、identity/features/capacity 复核、queue DMA rebuild、两阶段 IRQ rearm 与 epoch+1 replacement remount。
- [x] M57：ABI-v25 不变；两轮串行 `WRFWRF`、suppression `2/2/2`、一次 IRQ commit rollback 后成功 retry、IRQ-safe broker access、Running-owner `ServiceAbandoned` 与 physical submission gate 已封口。
- [x] M58：ABI-v25 不变；四相位 cooperative physical recovery、跨相位 driver-borrow 释放、单 attempt id 的 rearm/broker/admission 提交、七个 timer/双 worker/已认证 EL0 progress window 与零 masked polling 已封口；短 mask 计量只覆盖 M58 driver/control 区间。
- [x] M59：ABI-v24 AppData child 与 ABI-v23 timeout self-test 两份账本共享 cooperative physical engine；AppData 一次 read loss 冻结 outcome、kernel 不重放、userspace 只对零输出 `FileOpenAt` 一次 retry，timeout 不声称 EL0 progress；admission 由 policy coordinator 显式 rearm/open。
- [x] M60：ABI-v25 StorageServer child 完成 `WRFWRFR`、ticketed cap/backoff/Probation/Healthy、kernel-prearmed 模拟永久 read、boot-local sticky Offline 与 direct terminal IRQ/DMA proof；动态 fallback quarantine 仍为零次。
- [x] M61：ABI-v25 StorageServer child 完成 fatal-latched 250 ms owner grace、exact monitor kill、ObjectWait abandonment、ordinary reaper barrier 与 replacement recovery；只证明 fault-latched owner，不声称通用 heartbeat。
- [x] M62—M64：terminal quarantine fallback、persistent health/fresh reprobe 与 exact-session durable close/no-later-storage 已分别封口。
- [x] M65：ABI-v26 init-only 两阶段 StorageServer shutdown、final flush/readback/exit/reap 与 durable close/seal 已封口。
- [x] M66：ABI-v27 fixed resident graph、kernel topology proof、逆拓扑 quiesce、opaque token 与两次 QEMU-only self-exit 已封口。
- [x] M67—M70：真实项目 UI/InputServer/AppData/关机 closure、一次 StorageServer replacement、固定 App hard dependency 与 strict FDT+PSCI 1.1 QEMU self-exit 已分别封口。
- [x] M71：ABI-v32 五服务/四边有界持续监督、事务 batch、容忍同窗双服务瞬态 miss、一次超限 StorageServer replacement 与两次 PSCI QEMU self-exit 已封口。
- [x] M72：ABI-v33/init-only syscall 55、kernel-validated immutable/read-only BMF1 VMO、allocation-free 事务 parser、非旧顺序、4 个 resident binding、manifest 驱动的 StorageServer 初次 spawn/replacement 与两次同盘 release 启动已封口；完整离线 suite 已 exit 0。
- [x] M73：ABI-v34/init-only syscall 56、只读 UI convergence query、kernel-authenticated process ledger、持续到认证 power 的多 Channel event loop、两次 clean StorageServer rotation、budget rearm、4-response cancel/drain、零 `ProcessTerminate`/零 Killed 与两次同盘 release PSCI self-exit已封口。
- [x] M74：ABI-v35 不新增 syscall；外部 BMS1、kernel-pinned fixture RSA-2048 PKCS#1 v1.5 SHA-256 校验、静态 floor 2，以及损坏签名/有效旧 index 在 manifest 发布前的 fail-closed 负启动已封口。
- [x] M75：ABI-v36 不新增 syscall；pre-EL0 签名→双槽持久 floor transaction→prepared evidence 排序、QEMU DATA 上 floor `2→3`、第二次冗余修复、第三次零写稳态，以及坏签名/有效旧 index 的 pre-EL0、零槽改写拒绝已由三正两负 release 门封口；不声称生产 key、RPMB/eFuse、host replay/erase/tamper resistance、hardware power-cut 或真机。
- [x] M76：ABI-v37 不新增 syscall；建立 key id/epoch 2/2→3/3→4/4 的有序 fixture keyring、绑定 floor/active anchor/keyring policy 的双槽 `BNDRKEY1`、两次认证 transition、冗余修复/零写稳态、坏签名/有效已退休 key 的 pre-EL0 零槽改写拒绝，以及 public-only split-signing roundtrip；五正两负 release 门已封口。
- [x] M77：ABI-v38 新增 init-only syscall 57；完成 signature-first BMA1 对 manifest/key-policy/device/maintenance 的精确绑定、双槽 `BNDRMAU1` exact-sequence SHA-256 hash-chain audit、write/flush/readback、boot-local session 对 mutating supervisor report 的门禁，以及只读 UI query 无授权可用。六正三负 release 门证明坏签名、错误 binding、replay 均在 EL0 前零 manifest publication/零槽改写地拒绝。
- [x] M78：ABI-v39 不新增 syscall；独立双槽 `BNDRMEX1` completion ledger、前序完成门禁、`audit2/execution1` 精确同授权零 audit 写恢复、kernel-validated `audit2/execution2` completion，以及 completed-replay 的 pre-EL0 零槽改写拒绝，已由两次 PSCI 正启动、一次宿主中断和三次负启动封口；明确不声称 exactly-once、arbitrary resume、trusted monotonic、hardware power-cut 或真机。
- [x] M79：ABI-v40 不新增 syscall；相对 sector 9/10 的双槽 376-byte `BNDRMST1` 固定记录 rotation1/rotation2/drain，支持 M78 migration anchor、已持久步骤只读 reconciliation、三个 post-marker cut、损坏最新槽回退/修复，并把 terminal chain 绑定进 aggregate completion；五次 PSCI 正启动、三次宿主中断和三次负启动通过；明确不声称外部副作用 exactly-once、arbitrary resume、trusted monotonic、hardware power-cut 或真机。
- [x] M80：ABI-v41 不新增 syscall；相对 sector 11/12 的双槽 424-byte `BNDRMPL1` 为三个固定 operation 记录 plan/operation-instance/idempotency identity、九次 `PREPARED→APPLYING→CONFIRMED` transition 和 apply 前 compensation；prepared/applying/effect/cancel 四次宿主中断、result-unknown/effect-observed reconciliation、损坏最新槽回退/修复、terminal plan chain binding 与五份终态磁盘收敛均通过；明确不声称外部副作用 exactly-once、arbitrary resume、trusted monotonic、hardware power-cut 或真机。
- [ ] 下一本地阶段：把固定三操作计划改为签名、数据驱动的 bounded plan，覆盖重复 authorization/rotation sequence、所有合法 cancel race、意外 effect divergence、更广双槽损坏组合与更长非确定性 soak；随后推进多 App 持久存储与包生命周期。可信硬件单调后端、BSP/启动链、真实控制器、RPMB/eFuse 与 PMIC 路径必须等用户给出目标设备并另行明确授权；未获授权时不连接、刷写或操作真机。之后才是 hotplug、真实 power-cut、SMP/IOMMU、通用文件/cache、网络、电话/蜂窝、Wi-Fi、音频、AndroidBox 与产品安全。

M30/M31/M32 结果保留为历史里程碑与默认回归基线。feature-off/default 为 ABI v23；M54/M59 AppData 为 ABI v24；M55—M64 StorageServer 为 ABI v25；M65 为 ABI v26/syscall 53；M66 为 ABI v27/syscall 54；M67/M68/M69/M70/M71 分别为 ABI v28/v29/v30/v31/v32 且均不新增 syscall；M72 为 ABI v33/syscall 55；M73 为 ABI v34/syscall 56；M74/M75/M76 分别为 ABI v35/v36/v37、syscall 0—56；M77 为 ABI v38/syscall 0—57；M78/M79/M80 分别为 ABI v39/v40/v41、syscall 0—57，raw 58 unknown。当前 M80 kernel chain 继承 M79→M78→M77→M76→M75→M74→M73→M72→M71→M70→M69→M68→M67→M66→M65→…→M55，并复用真实 M45 UI 路径。M33—M80 为 48 个 opt-in leaf，加 default 共 49 份独立账本；本阶段已有统一交互 UI/AppData/shutdown、StorageServer 有界恢复、BMF1 manifest、持续事件监督、外部 BMS1 RSA 校验、QEMU DATA 双槽持久 rollback/key policy、key 2→3→4 迁移、public-only split-signing、signature-first BMA1 精确 binding、双槽 `BNDRMAU1` authorization audit、boot-local maintenance session gate、独立双槽 `BNDRMEX1` aggregate completion、双槽 `BNDRMST1` 固定三步 journal，以及双槽 `BNDRMPL1` prepare/apply/confirm/compensate plan、result-unknown/effect-observed reconciliation、四个 cut/cancel、损坏槽回退与 terminal-plan-chain binding，以及 FDT+PSCI 1.1 QEMU self-exit，但仍无外部副作用 exactly-once、arbitrary resume、生产 HSM/key ceremony、可信硬件单调源、host replay/erase/tamper resistance、任意产品服务集合/计划、任意多次 authorization/rotation、意外硬件故障、任意时长 soak、PMIC/hardware poweroff、真实控制器、hotplug/真实 power-cut、SMP/IOMMU 或真机验证。

### 15.1 第一个月

路线图 - 第一个月 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 第一个月 的最小可运行目标。
- [ ] 实现 第一个月 的 Rust 数据结构和错误模型。
- [ ] 接入 第一个月 的日志输出和诊断字段。
- [ ] 为 第一个月 编写单元测试或集成测试。
- [ ] 把 第一个月 接入构建系统和 QEMU 镜像。
- [ ] 建立 第一个月 的验收标准和失败回滚策略。
- [ ] 记录 第一个月 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 第一个月 增加性能指标和安全审计点。

#### 验收标准

- 第一个月 可以独立构建。
- 第一个月 可以在 QEMU 或测试环境中运行。
- 第一个月 的错误路径有日志。
- 第一个月 的权限检查可被测试覆盖。
- 第一个月 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.2 三个月

路线图 - 三个月 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 三个月 的最小可运行目标。
- [ ] 实现 三个月 的 Rust 数据结构和错误模型。
- [ ] 接入 三个月 的日志输出和诊断字段。
- [ ] 为 三个月 编写单元测试或集成测试。
- [ ] 把 三个月 接入构建系统和 QEMU 镜像。
- [ ] 建立 三个月 的验收标准和失败回滚策略。
- [ ] 记录 三个月 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 三个月 增加性能指标和安全审计点。

#### 验收标准

- 三个月 可以独立构建。
- 三个月 可以在 QEMU 或测试环境中运行。
- 三个月 的错误路径有日志。
- 三个月 的权限检查可被测试覆盖。
- 三个月 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.3 六个月

路线图 - 六个月 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 六个月 的最小可运行目标。
- [ ] 实现 六个月 的 Rust 数据结构和错误模型。
- [ ] 接入 六个月 的日志输出和诊断字段。
- [ ] 为 六个月 编写单元测试或集成测试。
- [ ] 把 六个月 接入构建系统和 QEMU 镜像。
- [ ] 建立 六个月 的验收标准和失败回滚策略。
- [ ] 记录 六个月 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 六个月 增加性能指标和安全审计点。

#### 验收标准

- 六个月 可以独立构建。
- 六个月 可以在 QEMU 或测试环境中运行。
- 六个月 的错误路径有日志。
- 六个月 的权限检查可被测试覆盖。
- 六个月 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.4 十二个月

路线图 - 十二个月 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 十二个月 的最小可运行目标。
- [ ] 实现 十二个月 的 Rust 数据结构和错误模型。
- [ ] 接入 十二个月 的日志输出和诊断字段。
- [ ] 为 十二个月 编写单元测试或集成测试。
- [ ] 把 十二个月 接入构建系统和 QEMU 镜像。
- [ ] 建立 十二个月 的验收标准和失败回滚策略。
- [ ] 记录 十二个月 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 十二个月 增加性能指标和安全审计点。

#### 验收标准

- 十二个月 可以独立构建。
- 十二个月 可以在 QEMU 或测试环境中运行。
- 十二个月 的错误路径有日志。
- 十二个月 的权限检查可被测试覆盖。
- 十二个月 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.5 二十四个月

路线图 - 二十四个月 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 二十四个月 的最小可运行目标。
- [ ] 实现 二十四个月 的 Rust 数据结构和错误模型。
- [ ] 接入 二十四个月 的日志输出和诊断字段。
- [ ] 为 二十四个月 编写单元测试或集成测试。
- [ ] 把 二十四个月 接入构建系统和 QEMU 镜像。
- [ ] 建立 二十四个月 的验收标准和失败回滚策略。
- [ ] 记录 二十四个月 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 二十四个月 增加性能指标和安全审计点。

#### 验收标准

- 二十四个月 可以独立构建。
- 二十四个月 可以在 QEMU 或测试环境中运行。
- 二十四个月 的错误路径有日志。
- 二十四个月 的权限检查可被测试覆盖。
- 二十四个月 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.6 真机阶段

路线图 - 真机阶段 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 真机阶段 的最小可运行目标。
- [ ] 实现 真机阶段 的 Rust 数据结构和错误模型。
- [ ] 接入 真机阶段 的日志输出和诊断字段。
- [ ] 为 真机阶段 编写单元测试或集成测试。
- [ ] 把 真机阶段 接入构建系统和 QEMU 镜像。
- [ ] 建立 真机阶段 的验收标准和失败回滚策略。
- [ ] 记录 真机阶段 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 真机阶段 增加性能指标和安全审计点。

#### 验收标准

- 真机阶段 可以独立构建。
- 真机阶段 可以在 QEMU 或测试环境中运行。
- 真机阶段 的错误路径有日志。
- 真机阶段 的权限检查可被测试覆盖。
- 真机阶段 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

### 15.7 生态阶段

路线图 - 生态阶段 是 Bndroid OS 的重要组成部分。它的设计目标是让系统能力稳定、可测试、可维护，并且能和 Rust-first 架构、Capability 安全模型、AndroidBox 兼容层保持一致。更好的实现方式是先确定最小 API，再实现内部逻辑，然后通过系统服务或 IPC 暴露给其他模块。

#### 设计重点

- 使用 Rust 类型表达资源所有权。
- 使用明确错误码表达失败原因。
- 使用日志和 trace 记录关键路径。
- 使用 capability 控制敏感访问。
- 使用自动化测试保护后续重构。
- 使用版本化接口支持长期演进。

#### TODO

- [ ] 定义 生态阶段 的最小可运行目标。
- [ ] 实现 生态阶段 的 Rust 数据结构和错误模型。
- [ ] 接入 生态阶段 的日志输出和诊断字段。
- [ ] 为 生态阶段 编写单元测试或集成测试。
- [ ] 把 生态阶段 接入构建系统和 QEMU 镜像。
- [ ] 建立 生态阶段 的验收标准和失败回滚策略。
- [ ] 记录 生态阶段 与 AndroidBox、系统服务、权限系统之间的接口。
- [ ] 为 生态阶段 增加性能指标和安全审计点。

#### 验收标准

- 生态阶段 可以独立构建。
- 生态阶段 可以在 QEMU 或测试环境中运行。
- 生态阶段 的错误路径有日志。
- 生态阶段 的权限检查可被测试覆盖。
- 生态阶段 与其他服务的接口稳定并可版本化。

本节的目标是把方向变成可以执行的工程任务。更好的做法是先建立最小闭环，再逐步扩展能力。每个模块都要有输入、输出、日志、测试和验收标准。Rust 负责核心资源管理和系统边界，C/C++ 组件只在兼容层或必要第三方库中出现。

实现时优先选择可验证设计。一个功能只有在 QEMU 或测试环境中能稳定复现、能输出日志、能被自动化测试覆盖，才算进入主线。系统工程不能依赖口头假设，需要用构建脚本、镜像、测试用例和兼容性数据库固定下来。

Bndroid OS 的关键优势来自清晰边界。应用不能直接访问硬件，Android 应用不能绕过 AndroidBox，系统服务不能越权访问资源，驱动不能无限制进入内核。所有访问都通过 capability、handle、IPC 和权限数据库表达。

## 附录 A：P0 总 TODO

以下 `P0-001`—`P0-120` 是蓝图生成时保留的编号占位任务，并不与当前代码里程碑一一对应，不能按 M15 或任何单一里程碑的测试数量/验收项机械勾选。项目的实际完成状态、运行证据和下一步优先级始终以 `IMPLEMENTATION_STATUS.md` 为准。

- [ ] P0-001：完成 Rust-first 最小系统闭环中的第 1 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-002：完成 Rust-first 最小系统闭环中的第 2 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-003：完成 Rust-first 最小系统闭环中的第 3 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-004：完成 Rust-first 最小系统闭环中的第 4 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-005：完成 Rust-first 最小系统闭环中的第 5 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-006：完成 Rust-first 最小系统闭环中的第 6 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-007：完成 Rust-first 最小系统闭环中的第 7 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-008：完成 Rust-first 最小系统闭环中的第 8 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-009：完成 Rust-first 最小系统闭环中的第 9 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-010：完成 Rust-first 最小系统闭环中的第 10 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-011：完成 Rust-first 最小系统闭环中的第 11 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-012：完成 Rust-first 最小系统闭环中的第 12 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-013：完成 Rust-first 最小系统闭环中的第 13 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-014：完成 Rust-first 最小系统闭环中的第 14 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-015：完成 Rust-first 最小系统闭环中的第 15 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-016：完成 Rust-first 最小系统闭环中的第 16 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-017：完成 Rust-first 最小系统闭环中的第 17 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-018：完成 Rust-first 最小系统闭环中的第 18 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-019：完成 Rust-first 最小系统闭环中的第 19 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-020：完成 Rust-first 最小系统闭环中的第 20 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-021：完成 Rust-first 最小系统闭环中的第 21 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-022：完成 Rust-first 最小系统闭环中的第 22 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-023：完成 Rust-first 最小系统闭环中的第 23 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-024：完成 Rust-first 最小系统闭环中的第 24 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-025：完成 Rust-first 最小系统闭环中的第 25 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-026：完成 Rust-first 最小系统闭环中的第 26 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-027：完成 Rust-first 最小系统闭环中的第 27 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-028：完成 Rust-first 最小系统闭环中的第 28 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-029：完成 Rust-first 最小系统闭环中的第 29 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-030：完成 Rust-first 最小系统闭环中的第 30 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-031：完成 Rust-first 最小系统闭环中的第 31 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-032：完成 Rust-first 最小系统闭环中的第 32 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-033：完成 Rust-first 最小系统闭环中的第 33 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-034：完成 Rust-first 最小系统闭环中的第 34 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-035：完成 Rust-first 最小系统闭环中的第 35 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-036：完成 Rust-first 最小系统闭环中的第 36 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-037：完成 Rust-first 最小系统闭环中的第 37 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-038：完成 Rust-first 最小系统闭环中的第 38 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-039：完成 Rust-first 最小系统闭环中的第 39 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-040：完成 Rust-first 最小系统闭环中的第 40 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-041：完成 Rust-first 最小系统闭环中的第 41 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-042：完成 Rust-first 最小系统闭环中的第 42 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-043：完成 Rust-first 最小系统闭环中的第 43 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-044：完成 Rust-first 最小系统闭环中的第 44 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-045：完成 Rust-first 最小系统闭环中的第 45 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-046：完成 Rust-first 最小系统闭环中的第 46 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-047：完成 Rust-first 最小系统闭环中的第 47 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-048：完成 Rust-first 最小系统闭环中的第 48 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-049：完成 Rust-first 最小系统闭环中的第 49 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-050：完成 Rust-first 最小系统闭环中的第 50 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-051：完成 Rust-first 最小系统闭环中的第 51 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-052：完成 Rust-first 最小系统闭环中的第 52 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-053：完成 Rust-first 最小系统闭环中的第 53 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-054：完成 Rust-first 最小系统闭环中的第 54 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-055：完成 Rust-first 最小系统闭环中的第 55 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-056：完成 Rust-first 最小系统闭环中的第 56 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-057：完成 Rust-first 最小系统闭环中的第 57 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-058：完成 Rust-first 最小系统闭环中的第 58 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-059：完成 Rust-first 最小系统闭环中的第 59 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-060：完成 Rust-first 最小系统闭环中的第 60 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-061：完成 Rust-first 最小系统闭环中的第 61 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-062：完成 Rust-first 最小系统闭环中的第 62 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-063：完成 Rust-first 最小系统闭环中的第 63 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-064：完成 Rust-first 最小系统闭环中的第 64 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-065：完成 Rust-first 最小系统闭环中的第 65 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-066：完成 Rust-first 最小系统闭环中的第 66 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-067：完成 Rust-first 最小系统闭环中的第 67 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-068：完成 Rust-first 最小系统闭环中的第 68 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-069：完成 Rust-first 最小系统闭环中的第 69 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-070：完成 Rust-first 最小系统闭环中的第 70 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-071：完成 Rust-first 最小系统闭环中的第 71 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-072：完成 Rust-first 最小系统闭环中的第 72 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-073：完成 Rust-first 最小系统闭环中的第 73 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-074：完成 Rust-first 最小系统闭环中的第 74 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-075：完成 Rust-first 最小系统闭环中的第 75 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-076：完成 Rust-first 最小系统闭环中的第 76 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-077：完成 Rust-first 最小系统闭环中的第 77 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-078：完成 Rust-first 最小系统闭环中的第 78 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-079：完成 Rust-first 最小系统闭环中的第 79 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-080：完成 Rust-first 最小系统闭环中的第 80 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-081：完成 Rust-first 最小系统闭环中的第 81 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-082：完成 Rust-first 最小系统闭环中的第 82 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-083：完成 Rust-first 最小系统闭环中的第 83 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-084：完成 Rust-first 最小系统闭环中的第 84 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-085：完成 Rust-first 最小系统闭环中的第 85 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-086：完成 Rust-first 最小系统闭环中的第 86 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-087：完成 Rust-first 最小系统闭环中的第 87 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-088：完成 Rust-first 最小系统闭环中的第 88 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-089：完成 Rust-first 最小系统闭环中的第 89 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-090：完成 Rust-first 最小系统闭环中的第 90 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-091：完成 Rust-first 最小系统闭环中的第 91 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-092：完成 Rust-first 最小系统闭环中的第 92 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-093：完成 Rust-first 最小系统闭环中的第 93 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-094：完成 Rust-first 最小系统闭环中的第 94 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-095：完成 Rust-first 最小系统闭环中的第 95 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-096：完成 Rust-first 最小系统闭环中的第 96 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-097：完成 Rust-first 最小系统闭环中的第 97 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-098：完成 Rust-first 最小系统闭环中的第 98 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-099：完成 Rust-first 最小系统闭环中的第 99 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-100：完成 Rust-first 最小系统闭环中的第 100 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-101：完成 Rust-first 最小系统闭环中的第 101 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-102：完成 Rust-first 最小系统闭环中的第 102 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-103：完成 Rust-first 最小系统闭环中的第 103 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-104：完成 Rust-first 最小系统闭环中的第 104 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-105：完成 Rust-first 最小系统闭环中的第 105 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-106：完成 Rust-first 最小系统闭环中的第 106 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-107：完成 Rust-first 最小系统闭环中的第 107 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-108：完成 Rust-first 最小系统闭环中的第 108 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-109：完成 Rust-first 最小系统闭环中的第 109 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-110：完成 Rust-first 最小系统闭环中的第 110 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-111：完成 Rust-first 最小系统闭环中的第 111 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-112：完成 Rust-first 最小系统闭环中的第 112 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-113：完成 Rust-first 最小系统闭环中的第 113 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-114：完成 Rust-first 最小系统闭环中的第 114 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-115：完成 Rust-first 最小系统闭环中的第 115 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-116：完成 Rust-first 最小系统闭环中的第 116 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-117：完成 Rust-first 最小系统闭环中的第 117 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-118：完成 Rust-first 最小系统闭环中的第 118 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-119：完成 Rust-first 最小系统闭环中的第 119 个可验证任务，要求包含构建、运行、日志、测试、验收记录。
- [ ] P0-120：完成 Rust-first 最小系统闭环中的第 120 个可验证任务，要求包含构建、运行、日志、测试、验收记录。

## 附录 B：AndroidBox 兼容性 TODO

- [ ] AndroidBox-001：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-002：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-003：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-004：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-005：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-006：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-007：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-008：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-009：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-010：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-011：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-012：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-013：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-014：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-015：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-016：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-017：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-018：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-019：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-020：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-021：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-022：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-023：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-024：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-025：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-026：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-027：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-028：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-029：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-030：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-031：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-032：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-033：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-034：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-035：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-036：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-037：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-038：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-039：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-040：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-041：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-042：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-043：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-044：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-045：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-046：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-047：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-048：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-049：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-050：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-051：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-052：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-053：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-054：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-055：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-056：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-057：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-058：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-059：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-060：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-061：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-062：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-063：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-064：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-065：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-066：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-067：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-068：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-069：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-070：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-071：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-072：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-073：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-074：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-075：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-076：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-077：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-078：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-079：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-080：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-081：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-082：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-083：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-084：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-085：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-086：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-087：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-088：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-089：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-090：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-091：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-092：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-093：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-094：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-095：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-096：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-097：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-098：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-099：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-100：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-101：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-102：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-103：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-104：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-105：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-106：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-107：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-108：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-109：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-110：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-111：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-112：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-113：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-114：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-115：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-116：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-117：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-118：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-119：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。
- [ ] AndroidBox-120：补齐一个 Android API、系统服务映射或 ABI 行为，并写入兼容性数据库。

## 附录 C：最终路线

Bndroid OS 当前 head 是 ABI-v41/M80 `unified-product-maintenance-plan-runtime`。它继承 M72 init-only syscall 55、M73 init-only syscall 56 `ServiceSupervisorReport`、持续到认证 power 的 event loop、M74 的外部 BMS1 签名门、M75 的双槽持久 rollback floor、M76 的 ordered fixture keyring/`BNDRKEY1` key policy、M77 的 init-only syscall 57 `MaintenanceSessionOpen`、signature-first BMA1 与双槽 `BNDRMAU1` authorization audit、M78 的双槽 `BNDRMEX1` aggregate completion，以及 M79 的双槽 `BNDRMST1` fixed-step journal。M80 不新增 syscall；它在 `BNDROID_DATA` 相对 sector 11/12 增加独立 424-byte 双槽 `BNDRMPL1` plan ledger。任何 EL0 启动前，kernel 在 audit mutation 之前预读 audit/execution/step/plan heads。三个固定 operation 依次对应两次 clean StorageServer rotation 和 resident drain，每项绑定 exact authorization、plan ID、operation-instance ID、idempotency key、deterministic effect digest 与前一 SHA-256 chain；正常路径严格执行九次 PREPARED/APPLYING/CONFIRMED transition，只有 apply 前 PREPARED 可以 COMPENSATED。专项门证明 normal、prepared/applying/effect 三处宿主 cut 与恢复、result-unknown/effect-observed reconciliation、cancel-prepared、corrupt-newest fallback/repair，以及五份终态磁盘收敛；terminal plan chain 已纳入 aggregate runtime digest。public-only split-signing assembler 不接受 private-key/signing 输入。底层 manifest parser 仍只支持容量 5 services/10 dependencies，产品 validator 仍固定五种已知服务/四边，故 `arbitrary_service_set_claim=0`；audit/execution/step/plan 仍为 host-controlled QEMU disk，`external_effect_exactly_once_claim=0 arbitrary_resume_claim=0 trusted_monotonic_backend=0 real_phone_claim=0`。

历史 M71 head 是 ABI-v32/M71 `unified-product-continuous-supervision-runtime`，kernel 通过 M70/M69/M68/M67/M66/M65/M64/M63/M62/M61/M60/M58/M57/M56 继承 M55 standalone `bndroid-storage-server` ELF、九镜像 catalog、syscall 47—52 与无 DUP/TRANSFER 的 volume/session capabilities。M65 历史新增 init-only syscall 53；M66 历史新增 child-only syscall 54 `ServiceShutdown`；M67—M71 均不新增 syscall。M67 复用真实 M45 UI/InputServer 路径，M68 增加单个 StorageServer 的一次有界 liveness recovery，M69 再增加固定 App hard dependency 的 block/resume，M70 把最终 emulator exit 绑定到 strict FDT discovery、PSCI_VERSION 与 kernel-only descriptor，M71 再增加事务式五服务/四依赖目录、批量探测、16 轮额外健康 soak 与一个双服务瞬态漏报窗口。M63/M64 仍为 kernel-only。feature-off/default 为 ABI v23，历史 M54/M59 AppData 为 ABI v24。

历史 M55 strict v2 block wire 为 4160 bytes、batch 1—8 sectors；M56—M64 完成 recovery、bounded Offline、owner/quarantine、durable hint/fresh probe 与 exact-session close；M65 完成两个 client drain、Prepare/Commit 与 StorageServer final flush/readback/exit/reap；M66 完成八个认证 resident 节点、10 条依赖、三波逆拓扑 quiesce、kernel topology proof、opaque fail-closed token 与两次 QEMU semihosting self-exit；M67 再完成真实 UI/InputServer convergence、认证 power key 116、Launcher/App AppData、final StorageServer 与 exact 双截图；M68 完成单个 StorageServer 的 probe Healthy→withheld/timeout→backoff→同槽下一代 replacement→Healthy；M69 完成固定 StorageServer+App、一条 hard edge、三次 cadence、fault/block→replacement→recovery/resume 与最终双服务 Healthy。历史 M70 再验证 `/psci`、`arm,psci-1.0`、HVC、PSCI 1.1 与两次 `SYSTEM_OFF` QEMU self-exit；历史 M71 完成五服务、四边、21 轮/107 probes、并发瞬态 miss 容忍与超限 StorageServer replacement。双启动最终 AppData generation 6、health generation 4 closed。

M33—M80 四十八个 opt-in leaf 加 default M32 共四十九份账本。系统整体仍为 `general_runtime=0 real_phone_claim=0`：没有一般 POSIX/fd/cache、生产 HSM custody/key ceremony/授权恢复、可信 RPMB/eFuse 单调源、host replay/erase/tamper resistance、多 App storage、产品级 IME、网络/蜂窝/电话/Wi-Fi/音频/电源、真实硬件驱动、完整安全更新或真机闭环。它仍是 bounded single-core QEMU research prototype，没有 SMP/IOMMU，不是真手机、不可刷机、不可日用。M73—M80 的 loop 虽持续到认证 power，但 proof profile 仍固定两次 F5/三项 operation，不可外推为任意时长、任意计划或任意故障可靠性；M76 的 fixture RSA keyring、M77 的 fixture maintenance root/public-only assembly、M78/M79/M80 的 QEMU-disk execution/step/plan ledgers，也不可外推为 production secure boot、外部副作用 exactly-once、arbitrary resume、trusted monotonic anti-replay、physical power-cut 或 PMIC/hardware poweroff。下一本地 P0 是把固定三操作计划改为签名、数据驱动的 bounded plan，扩大多 sequence/cancellation/effect-divergence/corruption/soak，并推进多 App 持久存储与包生命周期；可信硬件单调后端、真实 BSP/电源路径、刷写和真机操作必须在用户指定目标并另行明确授权后才能推进，真实 power-cut、真实控制器与真机恢复仍未证明。
