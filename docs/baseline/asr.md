# ASR 基线（M0 · 12.4 部分）

> 追溯 SPEC 4.1-1/2 · DESIGN 2.8.2-M0、2.1.3(7)
> 记录日期：2026-11-20
> 状态：**参考实现已实测；Rust 接入待完成**

## 1. 模型与运行时

| 项 | 值 |
|---|---|
| 模型 | `csukuangfj/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20` |
| 许可证 | Apache-2.0（仓库标注） |
| 量化版本 | int8 encoder / decoder / joiner |
| 模型体积 | encoder 181 MB、decoder 13 MB、joiner 3.2 MB、tokens 56 KB |
| 本地路径 | `models/asr/bilingual-zh-en-2023-02-20/`（已加入 `.gitignore`） |
| 下载源 | `hf-mirror.com`（GitHub / HuggingFace 直连不可达） |
| 参考运行时 | `sherpa-onnx 2.0.2`（Python，清华 PyPI 源） |

## 2. 实测数据（test_wavs/0.wav）

| 指标 | 实测 | 目标 | 结论 |
|---|---|---|---|
| 音频时长 | 10.053 s | — | — |
| 处理耗时 | 0.512 s | — | — |
| **RTF** | **0.0509** | < 1.0 | 达标（约 20× 实时） |
| **首字延迟** | **53 ms** | 目标 p95 ≤ 1.5 s | 达标（仅代表本地 wav 回放） |
| partial 更新次数 | 16 | — | 100 ms 分块 |
| 线程数 | 2 | ≤ 2 | 达标 |
| 峰值内存 | 待测 | ≤ 550 MB | 待补 |

转写结果示例（中英混合）：

```
昨天是 MONDAY TODAY IS LIBR THE DAY AFTER TOMORROW是星期三
```

中文部分准确；英文部分存在串词（`Tuesday` → `LIBR`），需用真实会议音频补测 CER。

## 3. 复现命令

```bash
pip install -i https://pypi.tuna.tsinghua.edu.cn/simple --only-binary=:all: sherpa-onnx
python scripts/asr_probe.py \
  models/asr/bilingual-zh-en-2023-02-20 \
  models/asr/bilingual-zh-en-2023-02-20/test_wavs/0.wav 100
```

## 4. 结论与待办

- **结论**：本地流式 ASR 在 2 线程下 RTF 约 0.05，首字延迟 53 ms，作为“断网可本地降级”方案可行。
- 待办：
  1. Rust 接入 `sherpa-onnx` crate 需 CMake + C++ 工具链（本机未安装 CMake）；
  2. 用真实会议音频补测中英混合 CER、final 时延 p95、内存峰值；
  3. 模型权重许可需法务复核（当前仅依据模型仓库标注 Apache-2.0）。