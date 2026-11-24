"""vosk 流式识别：stdin 读取 16 kHz 单声道 s16le，stdout 输出逐行 JSON。

- 中间结果：{"partial": "..."}
- 稳定句：{"text": "..."}

vosk 只在检测到静音端点（`AcceptWaveform` 返回 True）时才给 final。会议里经常长时间
连续说话、或音频一直不停，端点迟迟不触发，表现就是"一直在识别却拿不到任何文本"。
因此这里对 partial 做"稳定判定 + 超时强制收句"，主动产出 {"text"} 行：

- 同一 partial 连续 STABLE_ROUNDS 轮没变化 → 视为一句说完；
- 一句累计超过 MAX_SEGMENT_SECONDS → 强制收句，避免长句永不落地。

收句后 `Reset()`，从下一段音频重新累积。
"""

import json
import sys

from vosk import KaldiRecognizer, Model, SetLogLevel

SAMPLE_RATE = 16_000
BLOCK_BYTES = 4096
# 每轮读取 BLOCK_BYTES 字节，16 kHz s16le 单声道约 128 ms
STABLE_ROUNDS = 6
MAX_SEGMENT_SECONDS = 4.0


def emit(out, payload) -> None:
    out.write(json.dumps(payload, ensure_ascii=False) + "\n")
    out.flush()


def main() -> None:
    model_path = sys.argv[1] if len(sys.argv) > 1 else ".tmp/asr/vosk-model-small-cn-0.22"
    SetLogLevel(-1)
    model = Model(model_path)
    recognizer = KaldiRecognizer(model, SAMPLE_RATE)
    recognizer.SetWords(False)
    out = sys.stdout

    pending = ""
    stable = 0
    segment_ms = 0.0

    while True:
        data = sys.stdin.buffer.read(BLOCK_BYTES)
        if not data:
            break
        chunk_ms = 1_000.0 * (len(data) / 2) / SAMPLE_RATE
        if recognizer.AcceptWaveform(data):
            text = json.loads(recognizer.Result()).get("text", "").strip()
            if text:
                emit(out, {"text": text})
            pending = ""
            stable = 0
            segment_ms = 0.0
            continue
        partial = json.loads(recognizer.PartialResult()).get("partial", "").strip()
        if not partial:
            pending = ""
            stable = 0
            segment_ms = 0.0
            continue
        if partial != pending:
            emit(out, {"partial": partial})
            pending = partial
            stable = 0
        else:
            stable += 1
        segment_ms += chunk_ms
        if stable >= STABLE_ROUNDS or segment_ms >= MAX_SEGMENT_SECONDS * 1_000:
            emit(out, {"text": partial})
            recognizer.Reset()
            pending = ""
            stable = 0
            segment_ms = 0.0

    final = json.loads(recognizer.FinalResult()).get("text", "").strip()
    if final:
        emit(out, {"text": final})


if __name__ == "__main__":
    main()
