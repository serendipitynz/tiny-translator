#!/usr/bin/env python3
"""Measure llama-server running a PLaMo 3 Translate GGUF on macOS.

Starts llama-server the way the app will (127.0.0.1, free port, one slot), sends
PFML3 prompts to /completion, and reports load time, memory, generation speed,
how generation stopped, and whether memory goes back to the OS after exit.

macOS only: process memory comes from `ps` (RSS) and `footprint`
(phys_footprint, the value Activity Monitor shows as "Memory"), and system
memory from `vm_stat`. Standard library only, so it runs without a venv.
"""

import argparse
import json
import os
import re
import signal
import socket
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request

STOP = ["<|plamo:tag|>", "<|plamo:bos|>"]

SAMPLES = {
    "en-ja": (
        "en",
        "ja",
        (
            "The meeting has been moved to next Monday because several members of the "
            "design team will be travelling this week. Please review the attached draft "
            "before then and send any comments to the project channel. If you cannot "
            "attend, let us know in advance so that we can share the notes with you "
            "afterwards. We will also decide who takes over the release checklist while "
            "the current owner is on leave."
        ),
    ),
    "ja-en": (
        "ja",
        "en",
        (
            "設計チームの何人かが今週出張するため、会議は来週の月曜日に変更されました。"
            "それまでに添付の草案に目を通し、意見があればプロジェクトのチャンネルに送ってください。"
            "出席できない場合は前もって知らせてください。後で議事録を共有します。"
            "現在の担当者が休暇の間、リリースのチェックリストを誰が引き継ぐかもこの会議で決めます。"
        ),
    ),
}


def pfml3(text, target, source=None):
    source_attr = f"<|plamo:key|>lang<|plamo:val|>{source}" if source else ""
    return (
        "<|plamo:tag|>dataset<|plamo:msg|>translation"
        f"<|plamo:tag|>input{source_attr}<|plamo:msg|>{text}"
        f"<|plamo:tag|>output<|plamo:key|>lang<|plamo:val|>{target}<|plamo:msg|>"
    )


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def rss_bytes(pid):
    out = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    return int(out.strip()) * 1024 if out.strip() else None


def phys_footprint(pid):
    out = subprocess.run(
        ["footprint", "-p", str(pid), "-f", "bytes", "--noCategories"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    cur = re.search(r"phys_footprint:\s+(\d+) B", out)
    peak = re.search(r"phys_footprint_peak:\s+(\d+) B", out)
    return (int(cur.group(1)) if cur else None, int(peak.group(1)) if peak else None)


def system_used_bytes():
    # Activity Monitor's "Memory Used" = App Memory + Wired + Compressed.
    out = subprocess.run(
        ["vm_stat"], capture_output=True, text=True, check=False
    ).stdout
    page = int(re.search(r"page size of (\d+) bytes", out).group(1))

    def pages(label):
        return int(re.search(rf"{re.escape(label)}:\s+(\d+)\.", out).group(1))

    app = pages("Anonymous pages") - pages("Pages purgeable")
    return (
        app + pages("Pages wired down") + pages("Pages occupied by compressor")
    ) * page


def post(url, body, timeout=600):
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r)


def wait_ready(base, proc, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(
                f"llama-server exited with {proc.returncode} while loading"
            )
        try:
            with urllib.request.urlopen(f"{base}/health", timeout=1) as r:
                if r.status == 200:
                    return
        except (urllib.error.URLError, ConnectionError, TimeoutError):
            pass
        time.sleep(0.05)
    raise TimeoutError("llama-server did not become ready")


def complete(base, prompt, use_stop, n_predict, want_last_token=False):
    body = {
        "prompt": prompt,
        "n_predict": n_predict,
        "temperature": 0,
        "cache_prompt": False,
        "stream": False,
    }
    if use_stop:
        body["stop"] = STOP
    if want_last_token:
        # The EOG token that ends generation is not part of `content`; the last
        # entry of completion_probabilities is the only place it shows up.
        body["n_probs"] = 1
    r = post(f"{base}/completion", body)
    t = r.get("timings", {})
    probs = r.get("completion_probabilities") or []
    last = probs[-1] if probs else {}
    return {
        "last_token": [last.get("id"), last.get("token")] if want_last_token else None,
        "content": r.get("content"),
        "stop_type": r.get("stop_type"),
        "stopping_word": r.get("stopping_word"),
        "prompt_n": t.get("prompt_n"),
        "prompt_tps": t.get("prompt_per_second"),
        "predicted_n": t.get("predicted_n"),
        "predicted_tps": t.get("predicted_per_second"),
    }


def measure(server, model, ctx, extra, reps, n_predict, log_path, settle):
    port = free_port()
    base = f"http://127.0.0.1:{port}"
    args = [
        server,
        "-m",
        model,
        "-c",
        str(ctx),
        "--parallel",
        "1",
        "--host",
        "127.0.0.1",
        "--port",
        str(port),
        *extra,
    ]
    result = {"model": os.path.basename(model), "ctx": ctx, "args": args[1:]}

    result["system_used_before"] = system_used_bytes()
    with open(log_path, "w") as log:
        t0 = time.monotonic()
        proc = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
        try:
            wait_ready(base, proc, timeout=300)
            result["load_s"] = time.monotonic() - t0
            result["rss_loaded"] = rss_bytes(proc.pid)
            result["footprint_loaded"] = phys_footprint(proc.pid)[0]
            result["system_used_loaded"] = system_used_bytes()

            runs = {}
            for name, (src, tgt, text) in SAMPLES.items():
                prompt = pfml3(text, tgt, src)
                runs[name] = [
                    complete(base, prompt, True, n_predict) for _ in range(reps)
                ]
            # Without the stop list, to see which token ends generation on its own.
            for name, (src, tgt, text) in SAMPLES.items():
                runs[f"{name}-no-stop"] = [
                    complete(base, pfml3(text, tgt, src), False, n_predict, True)
                ]
            result["runs"] = runs

            result["rss_after_gen"] = rss_bytes(proc.pid)
            fp_cur, fp_peak = phys_footprint(proc.pid)
            result["footprint_after_gen"] = fp_cur
            result["footprint_peak"] = fp_peak
            result["system_used_after_gen"] = system_used_bytes()
        finally:
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
    result["exit_code"] = proc.returncode
    result["process_gone"] = rss_bytes(proc.pid) is None
    time.sleep(settle)
    result["system_used_after_exit"] = system_used_bytes()

    tps = [r["predicted_tps"] for n in ("en-ja", "ja-en") for r in runs[n]]
    result["predicted_tps_median"] = statistics.median(tps)
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--server", required=True, help="path to llama-server")
    p.add_argument("--ctx", type=int, default=8192)
    p.add_argument("--launches", type=int, default=3, help="server launches per model")
    p.add_argument(
        "--reps", type=int, default=1, help="requests per direction per launch"
    )
    p.add_argument(
        "--extra", default="", help="extra llama-server arguments, space separated"
    )
    p.add_argument("--n-predict", type=int, default=512)
    p.add_argument(
        "--settle",
        type=float,
        default=3.0,
        help="seconds to wait after exit before reading vm_stat",
    )
    p.add_argument(
        "--cooldown",
        type=float,
        default=0.0,
        help="seconds to idle before each launch; fanless Macs throttle the GPU under sustained load",
    )
    p.add_argument("--log-dir", default=".")
    p.add_argument("models", nargs="+", help="GGUF files")
    a = p.parse_args()

    results = []
    for m in a.models:
        for i in range(a.launches):
            log = os.path.join(a.log_dir, f"{os.path.basename(m)}.{i}.log")
            time.sleep(a.cooldown)
            print(f"measuring {m} (launch {i + 1}/{a.launches})", file=sys.stderr)
            r = measure(
                a.server, m, a.ctx, a.extra.split(), a.reps, a.n_predict, log, a.settle
            )
            r["launch"] = i + 1
            results.append(r)
    json.dump(results, sys.stdout, ensure_ascii=False, indent=2)
    print()


if __name__ == "__main__":
    main()
