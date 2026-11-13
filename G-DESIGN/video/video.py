"""Dry Run demo video: the voice drives the edit (adapted from the Hireclaw V3 pipeline).

  1. tts       ElevenLabs voice for every line of lines.json -> voice/<id>.mp3 + voice/durations.json
               (key injected by HACKATONS/tools/with-secrets.ps1 as SCAN_ELEVENLABS_2, never printed;
               voice id in $env:VOICE_ID)
  2. capture.js plays each shot for exactly its line's duration and records the marks -> raw/
  3. assemble  frames -> video, set-up time cut out, captions in a band under the picture,
               each line of voice placed at its mark -> dryrun-demo.mp4
"""
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
VOICE = HERE / os.environ.get("VOICE_DIR", "voice")
RAW = HERE / "raw"
LINES = json.loads((HERE / os.environ.get("LINES", "lines.json")).read_text(encoding="utf-8"))
W, H, BAND = 1440, 810, 110       # picture, then the caption band under it
FPS = 30
MODEL = "eleven_v3"
TEMPO = float(os.environ.get("TEMPO", "1.1"))
TAIL = 1.0                        # hold after the last line
OUT = HERE / os.environ.get("OUT", "dryrun-demo.mp4")


def run(*cmd: str) -> str:
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()


def duration(p: Path) -> float:
    return float(run("ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(p)))


def tts() -> None:
    raw = os.environ.get("SCAN_ELEVENLABS_2", "").strip()
    if not raw:
        sys.exit("SCAN_ELEVENLABS_2 missing: run through with-secrets.ps1")
    key = "s" + raw[1:] if raw[:3].lower() == "sk_" else raw   # vault copy starts with 'Sk_'
    voice_id = os.environ["VOICE_ID"]
    VOICE.mkdir(exist_ok=True)
    durs = {}
    for line in LINES:
        out = VOICE / f"{line['id']}.mp3"
        stamp = VOICE / f"{line['id']}.txt"
        tag = MODEL + "|" + voice_id + "|" + line["text"]
        if not out.exists() or not stamp.exists() or stamp.read_text(encoding="utf-8") != tag:
            body = json.dumps({"text": line["text"], "model_id": MODEL,
                               "voice_settings": {"stability": 0.5, "similarity_boost": 0.8}}).encode()
            req = urllib.request.Request(
                f"https://api.elevenlabs.io/v1/text-to-speech/{voice_id}?output_format=mp3_44100_128",
                data=body, headers={"xi-api-key": key, "Content-Type": "application/json"})
            try:
                with urllib.request.urlopen(req, timeout=120) as r:
                    out.write_bytes(r.read())
            except urllib.error.HTTPError as e:
                sys.exit(f"ElevenLabs {e.code}: {e.read().decode()[:300]}")
            stamp.write_text(tag, encoding="utf-8")
        # trim the silence v3 leaves around each line, then a light speed-up (pitch kept)
        fx = VOICE / f"{line['id']}.fx.wav"
        subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", str(out), "-af",
                        "silenceremove=start_periods=1:start_threshold=-45dB,areverse,"
                        "silenceremove=start_periods=1:start_threshold=-45dB,areverse,"
                        f"atempo={TEMPO},apad=pad_dur=0.05", str(fx)], check=True)
        durs[line["id"]] = round(duration(fx), 3)
        print(f"{line['id']:8s} {durs[line['id']]:5.2f} s")
    (VOICE / "durations.json").write_text(json.dumps(durs, indent=1))
    print(f"total voice {sum(durs.values()):.1f} s")


def timeline():
    """Kept frames with their output durations, and each mark mapped to output time."""
    frames = json.loads((RAW / "frames.json").read_text())        # [[file, epoch_s], ...]
    frames.sort(key=lambda f: f[1])
    meta = json.loads((RAW / "marks.json").read_text())           # {"marks": {id: epoch}, "cuts": [[a, b]], "stop": epoch}
    cuts, stop = meta["cuts"], meta["stop"]

    def kept(a: float, b: float) -> float:                        # length of [a, b] outside the cuts
        span = max(0.0, b - a)
        for c0, c1 in cuts:
            span -= max(0.0, min(b, c1) - max(a, c0))
        return max(0.0, span)

    t0 = frames[0][1]
    out, items = {}, []
    for i, (name, ts) in enumerate(frames):
        nxt = frames[i + 1][1] if i + 1 < len(frames) else stop
        d = kept(ts, nxt)
        if d > 0.0005:
            items.append((name, d))
    for k, ts in meta["marks"].items():
        out[k] = kept(t0, ts)
    return items, out, kept(t0, stop)


def assemble() -> None:
    if not all((RAW / f"cap_{l['id']}.png").exists() for l in LINES):
        sys.exit("caption PNGs missing: run capture.js first")
    items, marks, total = timeline()
    # constant 30 fps image sequence (hard links): the screencast frames arrive at an irregular rate
    seq = RAW / "seq"
    seq.mkdir(exist_ok=True)
    for old in seq.glob("*.jpg"):
        old.unlink()
    starts, t = [], 0.0
    for _, d in items:
        starts.append(t)
        t += d
    k, j = 0, 0
    while k / FPS < total:
        while j + 1 < len(items) and starts[j + 1] <= k / FPS:
            j += 1
        os.link(RAW / items[j][0], seq / f"{k:06d}.jpg")
        k += 1
    durs = json.loads((VOICE / "durations.json").read_text())
    inputs = ["-framerate", str(FPS), "-i", str(seq / "%06d.jpg")]
    vf = [f"[0:v]scale={W}:{H}:flags=lanczos,pad={W}:{H + BAND}:0:0:color=0x000000,"
          f"tpad=stop_mode=clone:stop_duration={TAIL},setpts=PTS-STARTPTS[v0]"]
    af = []
    for i, line in enumerate(LINES):
        a = marks[line["id"]]
        b = marks[LINES[i + 1]["id"]] if i + 1 < len(LINES) else total + TAIL
        inputs += ["-loop", "1", "-i", str(RAW / f"cap_{line['id']}.png")]
        inputs += ["-i", str(VOICE / f"{line['id']}.fx.wav")]
        img, snd = 1 + 2 * i, 2 + 2 * i
        vf.append(f"[v{i}][{img}:v]overlay=0:{H}:enable='between(t,{a:.3f},{b - 0.05:.3f})':shortest=1[v{i + 1}]")
        af.append(f"[{snd}:a]adelay={int(a * 1000)}:all=1[a{i}]")
        print(f"{line['id']:8s} at {a:6.2f} s  voice {durs[line['id']]:5.2f} s  shot {b - a:5.2f} s")
    n = len(LINES)
    mix = "".join(f"[a{i}]" for i in range(n))
    graph = ";".join(vf + af + [f"{mix}amix=inputs={n}:normalize=0,loudnorm=I=-16:TP=-1.5,apad[aout]"])
    subprocess.run(["ffmpeg", "-y", "-v", "error", *inputs, "-filter_complex", graph,
                    "-map", f"[v{n}]", "-map", "[aout]", "-t", f"{total + TAIL:.3f}",
                    "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
                    "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart", str(OUT)],
                   check=True, cwd=RAW)
    print(f"done: {OUT} ({duration(OUT):.1f} s)")


if __name__ == "__main__":
    {"tts": tts, "assemble": assemble}[sys.argv[1] if len(sys.argv) > 1 else os.environ.get("STAGE", "")]()
