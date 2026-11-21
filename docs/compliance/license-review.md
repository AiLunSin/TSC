# 高风险组件清单与使用边界

> 追溯 SPEC 1.4-3 · DESIGN 2.4.2、2.1.1
> 状态：**待复核签字**（M0 前须完成安全 + 法务双签，未签署不得进入 G1 门禁）

## 1. 强制边界

| 许可证类别 | 允许的接触方式 | 禁止的接触方式 |
|---|---|---|
| MIT / Apache-2.0 / BSD / ISC / MPL-2.0 | 直接依赖、静态链接、随包分发 | 无额外限制（保留版权声明） |
| GPL-2.0 / GPL-3.0 | 只读参考架构思路，不复制源码 | 复制代码、静态或动态链接、随包分发 |
| AGPL-3.0 | **仅作为用户自行部署的外部 HTTP 服务地址** | 内置、修改、分发、默认捆绑 |
| SSPL / OSL / EUPL | 不使用 | 全部 |

## 2. 逐项结论

| 组件 | 许可证 | 使用方式 | 边界结论 | 复核人 | 签字日期 |
|---|---|---|---|---|---|
| SearXNG | AGPL-3.0 | 用户自建检索服务，客户端仅通过 HTTP 调用其 API | **不内置、不分发、不修改**；配置示例不得暗示官方背书；默认不启用 | 待签 | 待签 |
| BlackHole（macOS 虚拟声卡） | GPL-3.0 | 不随包，仅在帮助文档说明"可由用户自行安装" | **不得捆绑**；macOS 音频走 ScreenCaptureKit | 待签 | 待签 |
| obs-studio | GPL-2.0 | 只读参考 WASAPI 环回与重采样参数 | **不复制代码**，仅借鉴工程参数 | 待签 | 待签 |
| sherpa-onnx（库） | Apache-2.0 | 直接依赖 | 可分发，保留 NOTICE | 待签 | 待签 |
| sherpa-onnx 模型权重（中文流式） | 需逐模型核实 | 随包分发 | **待法务确认**；未确认前不得进入发布包 | 待签 | 待签 |
| Silero VAD ONNX 模型 | MIT（仓库）/ 模型卡待核 | 随包分发 | 附模型卡与许可原文 | 待签 | 待签 |
| trafilatura | Apache-2.0 | 仅作回归对照，非默认运行 | 不引入 Python 运行时 | 待签 | 待签 |
| Tesseract 语言包 | Apache-2.0 / 语言包单独许可 | 可选 OCR 兜底 | 逐语言包核查，默认不打包 | 待签 | 待签 |
| sqlite-vec | MIT / Apache-2.0 | 直接依赖 | 随包，保留声明 | 待签 | 待签 |
| SQLCipher + OpenSSL | BSD-style | 直接链接 | 需出口合规声明 | 待签 | 待签 |
| schemars 1.2.3 | MIT | 直接依赖（Schema 派生） | 仅用于由领域类型派生 JSON Schema，不引入运行时校验引擎；无额外传递依赖；保留版权声明 | 待签 | 待签 |
| TLS 传输组件（`native-tls`，阶段 D，optional） | MIT / Apache-2.0 | 已引入为可选依赖（`cfg(windows)` 与 `cfg(target_os = "macos")`），默认构建零增量 | 系统 TLS：Windows → SChannel、macOS → Security.framework；**不额外打包 OpenSSL/ring**。`Cargo.lock` 新增 native-tls 0.2.18、schannel 0.1.30（Windows 路径）；macOS 路径为 security-framework。备选 `rustls + ring` 仅在系统 TLS 不可用时评估。**个人自用，未做商业分发合规评审** | 个人项目口径免双签 | 已登记 |

> **阶段 D 结论**：`llm-https = ["dep:native-tls"]` 已从空 feature 升级为真实依赖；许可类别仍受 `cargo deny` 约束（GPL/AGPL/SSPL 不可豁免）。

> **Phase A 结论**：生产 HTTPS 传输**挂起**，`llm-https = []` 为空 feature；`Cargo.lock` 与依赖闭包零变化。`schemars` 落在 `domain`（导出 Schema）与 `infrastructure`（编解码校验）两侧，属任务 L1.1 备注〔D1〕的第二种选择。

## 3. 发布前检查项

1. `cargo deny check licenses bans sources advisories` 通过，AGPL/GPL 命中 0。
2. `sbom.cdx.json` 已生成且通过 `cyclonedx-cli validate`。
3. 安装包内不存在 `searxng`、`blackhole`、`obs-studio` 的任何二进制或源码片段。
4. 第三方 NOTICE 文件包含全部直接依赖的许可证原文。
5. 模型权重目录附带 `LICENSE` 与来源 URL。

## 4. 复核记录

| 轮次 | 日期 | 安全负责人 | 法务负责人 | 结论 |
|---|---|---|---|---|
| R1（M0 前） | 待填 | 待签 | 待签 | 待定 |
| R2（M2 前） | 待填 | 待签 | 待签 | 待定 |
| R3（发布前） | 待填 | 待签 | 待签 | 待定 |