#!/usr/bin/env python3
"""把若干张同尺寸 PNG 拼成一个循环 GIF。

本机没有 ImageMagick，Pillow 是唯一依赖：pip install pillow
用法：python make-gif.py 输出.gif 帧1.png 帧2.png ...
最后一个参数之后可跟每个帧的停留毫秒（逗号分隔），缺省 1200。
"""
import os
import sys

from PIL import Image

MAX_WIDTH = 1240
COLORS = 128


def main() -> int:
    out = sys.argv[1]
    args = sys.argv[2:]
    durations = None
    if args and "," in args[-1] and all(part.isdigit() for part in args[-1].split(",")):
        durations = [int(part) for part in args[-1].split(",")]
        args = args[:-1]
    if not args:
        print("没有输入帧", file=sys.stderr)
        return 2

    frames = []
    for path in args:
        image = Image.open(path).convert("RGB")
        if image.width > MAX_WIDTH:
            ratio = MAX_WIDTH / image.width
            image = image.resize((MAX_WIDTH, round(image.height * ratio)), Image.LANCZOS)
        frames.append(image.convert("P", palette=Image.ADAPTIVE, colors=COLORS))

    if durations is None:
        durations = [1200] * len(frames)
    if len(durations) != len(frames):
        print(f"时长个数({len(durations)})与帧数({len(frames)})不一致", file=sys.stderr)
        return 2

    frames[0].save(
        out,
        save_all=True,
        append_images=frames[1:],
        duration=durations,
        loop=0,
        optimize=True,
        disposal=2,
    )
    print(f"{out}  {os.path.getsize(out) / 1024:.0f} KB  {len(frames)} 帧")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
