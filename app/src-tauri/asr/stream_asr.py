"""vosk 流式识别：stdin 读取 16 kHz 单声道 s16le，stdout 输出逐行 JSON。

- 稳定句：{"text": "..."}
- 中间结果：{"partial": "..."}
"""

import json
import sys

from vosk import KaldiRecognizer, Model, SetLogLevel


def main() -> None:
    model_path = sys.argv[1] if len(sys.argv) > 1 else ".tmp/asr/vosk-model-small-cn-0.22"
    SetLogLevel(-1)
    model = Model(model_path)
    recognizer = KaldiRecognizer(model, 16000)
    recognizer.SetWords(False)
    out = sys.stdout

    while True:
        data = sys.stdin.buffer.read(4096)
        if not data:
            break
        if recognizer.AcceptWaveform(data):
            text = json.loads(recognizer.Result()).get("text", "").strip()
            if text:
                out.write(json.dumps({"text": text}, ensure_ascii=False) + "\n")
                out.flush()
        else:
            partial = json.loads(recognizer.PartialResult()).get("partial", "").strip()
            if partial:
                out.write(json.dumps({"partial": partial}, ensure_ascii=False) + "\n")
                out.flush()

    final = json.loads(recognizer.FinalResult()).get("text", "").strip()
    if final:
        out.write(json.dumps({"text": final}, ensure_ascii=False) + "\n")
        out.flush()


if __name__ == "__main__":
    main()