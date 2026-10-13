#!/usr/bin/env python3
"""Record the examples and render compact terminal GIFs. Requires Pillow and pexpect."""

import json
import math
import os
from pathlib import Path
import shutil
import subprocess

import pexpect
from PIL import Image, ImageChops, ImageDraw, ImageFont


SCRIPTS = Path(__file__).resolve().parent
ROOT = SCRIPTS.parent
DOCS = ROOT / "docs"
WIDTH, HEIGHT = 880, 476
LEFT, TOP, ROW_HEIGHT = 24, 68, 21
ROWS = 18
COLORS = {
    "background": "#151922",
    "chrome": "#232833",
    "text": "#dce3ef",
    "muted": "#8e99ad",
    "prompt": "#98d1a1",
    "error": "#f38b95",
    "yellow": "#ebbd70",
}
FONT_PATH = subprocess.check_output(
    ["fc-match", "-f", "%{file}", "DejaVu Sans Mono"], text=True
).strip()
FONT = ImageFont.truetype(FONT_PATH, 16)
TITLE_FONT = ImageFont.truetype(FONT_PATH, 13)
CELL = FONT.getlength("M")
COLS = math.floor((WIDTH - LEFT * 2) / CELL)
DEMOS = [
    {
        "name": "program-state",
        "title": "cvar / ProgramState",
        "example": "program-state",
        "args": [],
        "commands": ["", "number 13", "poke! the value is", "text", "number nope", "number"],
        "expected": ['text is "the value is: 13"', 'error: number "nope"'],
    },
    {
        "name": "nesting-techniques",
        "title": "cvar / scopes and action aliases",
        "example": "nesting-techniques",
        "args": [],
        "commands": ["foo", "foo.bool false", "foo.int 7", "foo.string", "foo.float 2.5", "foo.string"],
        "expected": ['foo.bool is "false"', 'foo.string is "0"', 'foo.string is "2.5"'],
    },
    {
        "name": "runtime-properties",
        "title": "cvar / runtime properties",
        "example": "runtime-properties",
        "args": [],
        "commands": [
            "create! int player.health 100",
            "create! float player.speed 4.5",
            "create! string player.name Ada",
            "player",
            "player.health 75",
            "player",
            "destroy! player.speed",
            "player",
            "player.speed",
            "destroy! player.health",
            "destroy! player.name",
            "player",
        ],
        "expected": ['player.health is "75"', 'player.name is "Ada"', "unknown: player.speed", "unknown: player\n"],
    },
]


def record(demo):
    args = ["run", "--quiet", "--example", demo["example"], *demo["args"]]
    env = dict(os.environ, TERM="xterm-256color", CARGO_TERM_COLOR="never")
    child = pexpect.spawn("cargo", args, cwd=str(ROOT), env=env, encoding="utf-8",
                          echo=False, dimensions=(ROWS, COLS), timeout=60)
    try:
        child.expect_exact(">>> ")
        startup = child.before.replace("\r", "")
        steps = []
        for command in demo["commands"]:
            child.sendline(command)
            child.expect_exact(">>> ")
            steps.append({"command": command, "output": child.before.replace("\r", "")})
        child.sendeof()
        child.expect(pexpect.EOF)
        child.close()
        if child.exitstatus != 0:
            raise RuntimeError(f'{demo["name"]} exited with {child.exitstatus}')
    finally:
        if child.isalive():
            child.close(force=True)
    transcript = startup + "".join(step["output"] for step in steps)
    for expected in demo["expected"]:
        if expected not in transcript:
            raise RuntimeError(f'Missing expected output in {demo["name"]}: {expected!r}')
    return {"title": demo["title"], "launch": "cargo " + " ".join(args), "startup": startup, "steps": steps}


def rgb(hex_color):
    return tuple(int(hex_color[i:i + 2], 16) for i in (1, 3, 5))


def make_palette():
    colors = [rgb(color) for color in COLORS.values()]
    for foreground in ("text", "muted", "prompt", "error", "yellow"):
        for background in ("background", "chrome"):
            fg, bg = rgb(COLORS[foreground]), rgb(COLORS[background])
            for step in range(1, 12):
                colors.append(tuple(round(bg[i] + (fg[i] - bg[i]) * step / 12) for i in range(3)))
    colors.extend([(0, 0, 0)] * (256 - len(colors)))
    palette = Image.new("P", (1, 1))
    palette.putpalette([channel for color in colors for channel in color])
    return palette


PALETTE = make_palette()


def same_playback(original, candidate):
    if original.size != candidate.size or original.n_frames != candidate.n_frames:
        return False
    if original.info.get("loop") != candidate.info.get("loop"):
        return False
    for frame in range(original.n_frames):
        original.seek(frame)
        candidate.seek(frame)
        if original.info.get("duration") != candidate.info.get("duration"):
            return False
        difference = ImageChops.difference(original.convert("RGB"), candidate.convert("RGB"))
        # ImageMagick may round a few palette shades used for font antialiasing.
        if max(upper for _, upper in difference.getextrema()) > 5:
            return False
    return True


def render(recording, output):
    frames, durations, lines = [], [], []
    base = Image.new("RGB", (WIDTH, HEIGHT), COLORS["background"])
    draw = ImageDraw.Draw(base)
    draw.rectangle((0, 0, WIDTH, 45), fill=COLORS["chrome"])
    for x, color in [(23, "error"), (43, "yellow"), (63, "prompt")]:
        draw.ellipse((x - 5, 18, x + 5, 28), fill=COLORS[color])
    draw.text((WIDTH / 2, 23), recording["title"], font=TITLE_FONT, fill=COLORS["text"], anchor="mm")

    def append_output(text):
        for line in text.splitlines():
            color = "error" if line.startswith(("error:", "unknown:")) else "text"
            lines.append([(line, color)])

    def snapshot(duration=100, cursor=True):
        visible = []
        for segments in lines:
            row, used = [], 0
            for text, color in segments:
                while text:
                    part, text = text[:COLS - used], text[COLS - used:]
                    row.append((part, color))
                    used += len(part)
                    if used == COLS:
                        visible.append(row)
                        row, used = [], 0
            visible.append(row)
        visible = visible[-ROWS:]
        frame = base.copy()
        painter = ImageDraw.Draw(frame)
        for row, segments in enumerate(visible):
            x = LEFT
            for text, color in segments:
                painter.text((x, TOP + row * ROW_HEIGHT), text, font=FONT, fill=COLORS[color])
                x += CELL * len(text)
        if cursor and visible:
            x = LEFT + sum(len(text) for text, _ in visible[-1]) * CELL
            y = TOP + (len(visible) - 1) * ROW_HEIGHT + 2
            painter.rectangle((int(x), y, int(x + CELL - 2), y + 16), fill=COLORS["text"])
        frames.append(frame.quantize(palette=PALETTE, dither=Image.Dither.NONE))
        durations.append(duration)

    def type_command(prompt, command, chunk=2, speed=80):
        lines.append([(prompt, "prompt"), ("", "text")])
        snapshot(300)
        for end in range(chunk, len(command) + chunk, chunk):
            lines[-1][-1] = (command[:end], "text")
            snapshot(speed)
        snapshot(300, cursor=False)

    type_command("$ ", recording["launch"], chunk=3, speed=60)
    append_output(recording["startup"])
    lines.append([(">>> ", "prompt")])
    snapshot(1400)
    lines.pop()
    for step in recording["steps"]:
        type_command(">>> ", step["command"])
        append_output(step["output"])
        lines.append([(">>> ", "prompt")])
        snapshot(1400 if step["output"] else 500)
        lines.pop()
    lines.append([(">>> ", "prompt")])
    snapshot(3000)
    frames[0].save(output, save_all=True, append_images=frames[1:], duration=durations,
                   loop=0, disposal=1, optimize=True)
    preview = output.with_suffix(".png")
    frames[-1].convert("RGB").save(preview)
    preview.replace(ROOT / "target" / f"{output.stem}-preview.png")

    # Pillow already stores changed rectangles. Try ImageMagick's layer optimizer
    # as well, retaining the smaller result only when frames and timing match.
    if shutil.which("convert"):
        optimized = output.with_name(output.stem + ".optimized.gif")
        subprocess.run(["convert", str(output), "-layers", "Optimize", str(optimized)], check=True)
        if optimized.stat().st_size < output.stat().st_size:
            with Image.open(output) as original, Image.open(optimized) as candidate:
                equivalent = same_playback(original, candidate)
            if equivalent:
                optimized.replace(output)
        if optimized.exists():
            optimized.unlink()
    with Image.open(output) as gif:
        total = sum(gif.seek(i) or gif.info.get("duration", 0) for i in range(gif.n_frames))
        print(f"{output.name}: {gif.size}, {gif.n_frames} frames, {total / 1000:.1f}s, {output.stat().st_size / 1024:.1f} KiB")


def main():
    DOCS.mkdir(exist_ok=True)
    subprocess.run(["cargo", "build", "--examples", "--quiet"], cwd=ROOT, check=True)
    recordings = {}
    for demo in DEMOS:
        recording = record(demo)
        recordings[demo["name"]] = recording
        render(recording, DOCS / (demo["name"] + ".gif"))
    (SCRIPTS / "demo-recordings.json").write_text(json.dumps(recordings, indent=2) + "\n")


if __name__ == "__main__":
    main()
