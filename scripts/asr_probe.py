import json
import sys
import time
import wave
from pathlib import Path

import numpy as np
import sherpa_onnx


def read_wave_16k(path: Path):
    with wave.open(str(path), "rb") as handle:
        sample_rate = handle.getframerate()
        channels = handle.getnchannels()
        width = handle.getsampwidth()
        frames = handle.readframes(handle.getnframes())
    if width != 2:
        raise ValueError(f"expect 16-bit PCM, got {width * 8}-bit")
    samples = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
    if channels > 1:
        samples = samples.reshape(-1, channels).mean(axis=1)
    return samples, sample_rate


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: asr_probe.py <model_dir> <wav> [chunk_ms]")
        return 2

    model_dir = Path(sys.argv[1])
    wav = Path(sys.argv[2])
    chunk_ms = int(sys.argv[3]) if len(sys.argv) > 3 else 100

    samples, sample_rate = read_wave_16k(wav)
    if sample_rate != 16000:
        print(f"expected 16kHz, got {sample_rate}")
        return 2

    recognizer = sherpa_onnx.OnlineRecognizer.from_transducer(
        tokens=str(model_dir / "tokens.txt"),
        encoder=str(model_dir / "encoder-epoch-99-avg-1.int8.onnx"),
        decoder=str(model_dir / "decoder-epoch-99-avg-1.int8.onnx"),
        joiner=str(model_dir / "joiner-epoch-99-avg-1.int8.onnx"),
        num_threads=2,
        sample_rate=16000,
        feature_dim=80,
        decoding_method="greedy_search",
    )

    stream = recognizer.create_stream()
    chunk = int(16000 * chunk_ms / 1000)
    started = time.perf_counter()
    first_text_at = None
    partials = []

    for offset in range(0, len(samples), chunk):
        stream.accept_waveform(16000, samples[offset : offset + chunk])
        while recognizer.is_ready(stream):
            recognizer.decode_stream(stream)
        text = recognizer.get_result(stream)
        if text and first_text_at is None:
            first_text_at = time.perf_counter() - started
        if text and (not partials or partials[-1] != text):
            partials.append(text)

    stream.accept_waveform(16000, [0.0] * int(16000 * 0.3))
    stream.input_finished()
    while recognizer.is_ready(stream):
        recognizer.decode_stream(stream)
    final_text = recognizer.get_result(stream)
    elapsed = time.perf_counter() - started
    audio_seconds = len(samples) / 16000

    result = {
        "audio_seconds": round(audio_seconds, 3),
        "processing_seconds": round(elapsed, 3),
        "rtf": round(elapsed / audio_seconds, 4) if audio_seconds else None,
        "first_text_latency_ms": round(first_text_at * 1000, 1)
        if first_text_at is not None
        else None,
        "partial_updates": len(partials),
        "final_text": final_text,
        "chunk_ms": chunk_ms,
        "threads": 2,
    }
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())