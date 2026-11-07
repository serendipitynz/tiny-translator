#!/usr/bin/env python3
"""Measure llama-server running a PLaMo 3 Translate GGUF on macOS or Windows.

Starts llama-server the way the app will (127.0.0.1, free port, one slot), sends
PFML3 prompts to /completion, and reports load time, memory, generation speed,
how generation stopped, and whether memory goes back to the OS after exit.

On macOS, process memory comes from `ps` (RSS) and `footprint` (phys_footprint,
the value Activity Monitor shows as "Memory"), and system memory from `vm_stat`.

On Windows, process memory is the working set and private bytes, and the
process's dedicated and shared GPU memory (the columns Task Manager shows): with
the Vulkan build the weights live in GPU memory, which the working set does not
count — in VRAM on a discrete GPU, and in RAM ("shared GPU memory") on an
integrated one. System memory is the available physical memory and the commit
charge, and each result also records the machine (CPU, GPU drivers, AC power).

Standard library only, so it runs without a venv.
"""

import argparse
import ctypes
import json
import os
import platform
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

WINDOWS = sys.platform == "win32"

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


class ProcessMemoryCounters(ctypes.Structure):
    # PROCESS_MEMORY_COUNTERS_EX2. PrivateWorkingSetSize is the value Task Manager's
    # "Memory" column shows.
    _fields_ = [
        ("cb", ctypes.c_uint32),
        ("PageFaultCount", ctypes.c_uint32),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
        ("PrivateUsage", ctypes.c_size_t),
        ("PrivateWorkingSetSize", ctypes.c_size_t),
        ("SharedCommitUsage", ctypes.c_uint64),
    ]


class MemoryStatus(ctypes.Structure):
    # MEMORYSTATUSEX
    _fields_ = [
        ("dwLength", ctypes.c_uint32),
        ("dwMemoryLoad", ctypes.c_uint32),
        ("ullTotalPhys", ctypes.c_uint64),
        ("ullAvailPhys", ctypes.c_uint64),
        ("ullTotalPageFile", ctypes.c_uint64),
        ("ullAvailPageFile", ctypes.c_uint64),
        ("ullTotalVirtual", ctypes.c_uint64),
        ("ullAvailVirtual", ctypes.c_uint64),
        ("ullAvailExtendedVirtual", ctypes.c_uint64),
    ]


class PdhCounterValue(ctypes.Structure):
    # PDH_FMT_COUNTERVALUE read as PDH_FMT_LARGE; the union's 8-byte alignment puts
    # the value after 4 bytes of padding, as ctypes lays it out too.
    _fields_ = [("CStatus", ctypes.c_uint32), ("largeValue", ctypes.c_int64)]


class PdhCounterValueItem(ctypes.Structure):
    _fields_ = [("szName", ctypes.c_wchar_p), ("FmtValue", PdhCounterValue)]


PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
PROCESS_VM_READ = 0x0010
STILL_ACTIVE = 259
PDH_FMT_LARGE = 0x400
PDH_MORE_DATA = 0x800007D2


def win_open_process(pid):
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.restype = ctypes.c_void_p
    handle = kernel32.OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, False, pid
    )
    return kernel32, handle


def win_process_alive(pid):
    kernel32, handle = win_open_process(pid)
    if not handle:
        return False
    try:
        code = ctypes.c_uint32()
        ok = kernel32.GetExitCodeProcess(ctypes.c_void_p(handle), ctypes.byref(code))
        return bool(ok) and code.value == STILL_ACTIVE
    finally:
        kernel32.CloseHandle(ctypes.c_void_p(handle))


def win_process_memory(pid):
    kernel32, handle = win_open_process(pid)
    if not handle:
        return {}
    try:
        counters = ProcessMemoryCounters()
        counters.cb = ctypes.sizeof(counters)
        ok = kernel32.K32GetProcessMemoryInfo(
            ctypes.c_void_p(handle), ctypes.byref(counters), counters.cb
        )
        if not ok:
            # Windows before PROCESS_MEMORY_COUNTERS_EX2: the same fields up to
            # PrivateUsage, without the private working set.
            counters.cb = ProcessMemoryCounters.PrivateWorkingSetSize.offset
            ok = kernel32.K32GetProcessMemoryInfo(
                ctypes.c_void_p(handle), ctypes.byref(counters), counters.cb
            )
            counters.PrivateWorkingSetSize = 0
        if not ok:
            return {}
        return {
            "working_set": counters.WorkingSetSize,
            "peak_working_set": counters.PeakWorkingSetSize,
            "private_working_set": counters.PrivateWorkingSetSize or None,
            "private_bytes": counters.PrivateUsage,
        }
    finally:
        kernel32.CloseHandle(ctypes.c_void_p(handle))


def pdh_values(path):
    """Every instance of a counter given by its English path, as {instance: value}.

    PDH rather than `Get-Counter`: counter names are localized (Japanese Windows names
    them in Japanese), and PdhAddEnglishCounterW is the one call that takes the
    English name everywhere.
    """
    pdh = ctypes.WinDLL("pdh")
    for name in (
        "PdhOpenQueryW",
        "PdhAddEnglishCounterW",
        "PdhCollectQueryData",
        "PdhGetFormattedCounterArrayW",
        "PdhCloseQuery",
    ):
        getattr(pdh, name).restype = ctypes.c_uint32
    query = ctypes.c_void_p()
    if pdh.PdhOpenQueryW(None, None, ctypes.byref(query)) != 0:
        return None
    try:
        counter = ctypes.c_void_p()
        if pdh.PdhAddEnglishCounterW(query, path, None, ctypes.byref(counter)) != 0:
            return None
        if pdh.PdhCollectQueryData(query) != 0:
            return None
        size = ctypes.c_uint32(0)
        count = ctypes.c_uint32(0)
        status = pdh.PdhGetFormattedCounterArrayW(
            counter, PDH_FMT_LARGE, ctypes.byref(size), ctypes.byref(count), None
        )
        if status != PDH_MORE_DATA:
            return {} if status == 0 else None
        buffer = ctypes.create_string_buffer(size.value)
        status = pdh.PdhGetFormattedCounterArrayW(
            counter, PDH_FMT_LARGE, ctypes.byref(size), ctypes.byref(count), buffer
        )
        if status != 0:
            return None
        items = ctypes.cast(buffer, ctypes.POINTER(PdhCounterValueItem))
        return {
            items[i].szName: items[i].FmtValue.largeValue
            for i in range(count.value)
            if items[i].FmtValue.CStatus in (0, 1)  # PDH_CSTATUS_VALID_DATA / NEW_DATA
        }
    finally:
        pdh.PdhCloseQuery(query)


def gpu_memory(counter_set, counter, pid=None):
    """The sum over GPUs of a GPU memory counter, for one process or the whole system.

    Instances of "GPU Process Memory" are named `pid_<PID>_luid_<adapter>_phys_<n>`.
    """
    values = pdh_values(rf"\{counter_set}(*)\{counter}")
    if values is None:
        return None
    if pid is not None:
        prefix = f"pid_{pid}_"
        values = {k: v for k, v in values.items() if k.startswith(prefix)}
    return sum(values.values())


def win_system_memory():
    kernel32 = ctypes.WinDLL("kernel32")
    status = MemoryStatus()
    status.dwLength = ctypes.sizeof(status)
    if not kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        return {}
    return {
        "available": status.ullAvailPhys,
        # Task Manager's "Committed": what processes have committed, RAM and page file.
        "committed": status.ullTotalPageFile - status.ullAvailPageFile,
        "gpu_dedicated": gpu_memory("GPU Adapter Memory", "Dedicated Usage"),
        "gpu_shared": gpu_memory("GPU Adapter Memory", "Shared Usage"),
    }


def win_machine(server):
    class PowerStatus(ctypes.Structure):
        # SYSTEM_POWER_STATUS
        _fields_ = [
            ("ACLineStatus", ctypes.c_uint8),
            ("BatteryFlag", ctypes.c_uint8),
            ("BatteryLifePercent", ctypes.c_uint8),
            ("SystemStatusFlag", ctypes.c_uint8),
            ("BatteryLifeTime", ctypes.c_uint32),
            ("BatteryFullLifeTime", ctypes.c_uint32),
        ]

    power = PowerStatus()
    ctypes.WinDLL("kernel32").GetSystemPowerStatus(ctypes.byref(power))
    memory = MemoryStatus()
    memory.dwLength = ctypes.sizeof(memory)
    ctypes.WinDLL("kernel32").GlobalMemoryStatusEx(ctypes.byref(memory))
    query = (
        "[Console]::OutputEncoding = [Text.Encoding]::UTF8; "
        "@{cpu = @(Get-CimInstance Win32_Processor | ForEach-Object Name); "
        "gpus = @(Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion, "
        "@{n = 'DriverDate'; e = {$_.DriverDate.ToString('yyyy-MM-dd')}})} | ConvertTo-Json -Compress"
    )
    out = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", query],
        capture_output=True,
        encoding="utf-8",
        check=False,
    ).stdout
    try:
        devices = json.loads(out)
    except ValueError:
        devices = {}
    # The GPUs llama-server itself sees, with their memory: on an integrated GPU, how
    # much the UMA setting gives it. Timed, because on a copied exe this is its first
    # start, which an antivirus may scan first (doc-1 §4.5 saw a one-off ~10 s on macOS).
    t0 = time.monotonic()
    listed = subprocess.run(
        [server, "--list-devices"], capture_output=True, text=True, check=False
    )
    list_devices_s = time.monotonic() - t0
    return {
        "windows": platform.version(),
        "cpu": [name.strip() for name in devices.get("cpu") or []],
        "gpus": devices.get("gpus"),
        "llama_devices": [
            line.strip()
            for line in (listed.stdout + listed.stderr).splitlines()
            if line.strip() and line.strip() != "Available devices:"
        ],
        "list_devices_s": list_devices_s,
        "total_physical": memory.ullTotalPhys,
        "ac_power": {0: False, 1: True}.get(power.ACLineStatus),
    }


def process_alive(pid):
    return win_process_alive(pid) if WINDOWS else rss_bytes(pid) is not None


def record_process(result, pid, stage):
    if WINDOWS:
        memory = win_process_memory(pid)
        memory["gpu_dedicated"] = gpu_memory("GPU Process Memory", "Dedicated Usage", pid)
        memory["gpu_shared"] = gpu_memory("GPU Process Memory", "Shared Usage", pid)
        for key, value in memory.items():
            result[f"{key}_{stage}"] = value
        return
    result[f"rss_{stage}"] = rss_bytes(pid)
    current, peak = phys_footprint(pid)
    result[f"footprint_{stage}"] = current
    if stage == "after_gen":
        result["footprint_peak"] = peak


def record_system(result, stage):
    if WINDOWS:
        for key, value in win_system_memory().items():
            result[f"system_{key}_{stage}"] = value
    else:
        result[f"system_used_{stage}"] = system_used_bytes()


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

    record_system(result, "before")
    with open(log_path, "w") as log:
        t0 = time.monotonic()
        proc = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
        try:
            wait_ready(base, proc, timeout=300)
            result["load_s"] = time.monotonic() - t0
            record_process(result, proc.pid, "loaded")
            record_system(result, "loaded")

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

            record_process(result, proc.pid, "after_gen")
            record_system(result, "after_gen")
        finally:
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
    result["exit_code"] = proc.returncode
    result["process_gone"] = not process_alive(proc.pid)
    time.sleep(settle)
    record_system(result, "after_exit")

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
        help="seconds to wait after exit before reading system memory",
    )
    p.add_argument(
        "--cooldown",
        type=float,
        default=0.0,
        help="seconds to idle before each launch; fanless Macs throttle the GPU under sustained load",
    )
    p.add_argument("--log-dir", default=".")
    p.add_argument(
        "--out",
        help="write the JSON here instead of stdout; on Windows, a shell redirect "
        "would re-encode it in the console's code page",
    )
    p.add_argument("models", nargs="+", help="GGUF files")
    a = p.parse_args()

    # Absolute: Windows' CreateProcess does not resolve a relative path written with
    # forward slashes.
    server = os.path.abspath(a.server)
    machine = win_machine(server) if WINDOWS else None
    results = []
    for m in a.models:
        for i in range(a.launches):
            log = os.path.join(a.log_dir, f"{os.path.basename(m)}.{i}.log")
            time.sleep(a.cooldown)
            print(f"measuring {m} (launch {i + 1}/{a.launches})", file=sys.stderr)
            r = measure(
                server, m, a.ctx, a.extra.split(), a.reps, a.n_predict, log, a.settle
            )
            r["launch"] = i + 1
            if machine:
                r["machine"] = machine
            results.append(r)
    if a.out:
        with open(a.out, "w", encoding="utf-8") as out:
            json.dump(results, out, ensure_ascii=False, indent=2)
            out.write("\n")
    else:
        json.dump(results, sys.stdout, ensure_ascii=False, indent=2)
        print()


if __name__ == "__main__":
    main()
