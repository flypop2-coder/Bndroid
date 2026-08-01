# Bndroid OS 白皮书

## 摘要

Bndroid OS 是一个研究型操作系统，用于探索在 Rust-first、非 Linux 技术栈上构建
Android 兼容用户体验。项目关注一个明确问题：在引入完整 Android Framework 或
ART 之前，Android 应用模型中有多少部分可以被投影到能力导向内核、受监督服务、
确定性存储和小型原生 UI 运行时之上？

当前答案是刻意收窄的。Bndroid 可以在 QEMU 中启动，运行自己的用户态服务，渲染
移动 UI surface，并通过证据驱动测试执行 AndroidBox fixture。它还不是 Android
发行版，不是生产级移动操作系统，也不承诺兼容任意 APK。

## 设计目标

- 让可信计算基保持可理解。
- 显式建模服务故障与重启行为。
- 将存储、包状态和 UI 更新视为可恢复的系统契约。
- 通过有界、可审计的解析器摄取 APK 证据。
- 用确定性测试替代宽泛的兼容性宣称。
- 在完整 Android runtime 成为明确架构选择之前，避免意外依赖它。

## 架构

Bndroid 围绕自研 AArch64 内核和一组受监督的用户态服务组织。内核对象通过句柄暴露
有界能力；进程通过通道、事件、VMO 和服务协议通信。

UI 路径包含原生 compositor、frame clock、input broker、surface model 和移动文本/
字体流水线。存储路径拆分为块访问、包状态、应用数据、持久化和恢复策略。这些边界
不仅是内部实现细节，也作为产品行为被测试。

AndroidBox 是兼容层。目前它专注于受控 APK fixture：envelope 校验、manifest/
resource 解析、选定 DEX 执行、Activity 状态、UI scene 投影、布局几何和点击命中
测试。每个里程碑都会记录协议版本、fixture 证据、QEMU 运行次数和 host test 数量，
用来证明行为边界。

## 当前边界

Bndroid 当前目标平台是 QEMU `virt` AArch64。AndroidBox 还不包含 ART、Android
Framework、Play services、Binder 兼容、硬件设备集成或任意第三方 APK 支持。已记录
的 AndroidBox 门禁默认禁用网络，除非未来任务明确授权。

这些限制是研究方法的一部分：项目通过一次增加一个可观察契约的方式成长，并保留
旧契约仍然成立的证据。

## 许可证立场

源代码采用 Apache-2.0，因为它宽松、通用，并包含明确的专利授权。对于操作系统和
Android 兼容研究项目来说，这比没有专利条款的极简宽松许可证更合适。

捆绑的 Bndroid Sans Raster 字体数据与源代码许可证分开处理。它源自 OFL 授权字体
软件，因此继续使用 OFL-1.1。仓库通过 `LICENSE`、`NOTICE`、`LICENSES/` 和
`.reuse/dep5` 记录这种拆分。

## 路线图

近期方向是继续把 AndroidBox 行为转化为 opt-in、证据驱动的门禁：更丰富的布局契约、
资源语义、Activity 生命周期覆盖、基于存储的包行为，以及更严格的重启监督。完整
Android runtime/container 是单独的架构决策，采用前应先明确评估。
