#!/usr/bin/env python3
"""What will actually run on this machine, and how fast.

A development tool, and deliberately a second opinion rather than a convenience.
It computes the cache size from a real GGUF on disk by an independent route from
`src-tauri/src/estimate.rs`, which is only ever exercised against synthetic
headers in its own tests. Where the two disagree about a file in the models
directory, one of them is wrong and that is worth knowing.

It also reads the two ceilings the app got wrong until 2026-08-31: the GPU
working set, taken from `llama-server --list-devices` rather than computed as a
fraction of installed RAM, and what is actually free right now.

  tools/fits.py MODEL.gguf [MODEL.gguf ...]   what is allowed
  tools/fits.py MODEL.gguf --run              launch the best one and measure it

Needs python3 and llama-server on PATH. Nothing else; no packages.
"""
import json, os, re, struct, subprocess, sys, time, urllib.request

MARGIN_MIB = 1024          # llama.cpp's --fit-target default, per device
MIB = 1024 ** 2
BPE = {"f16": 2.0, "bf16": 2.0, "q8_0": 34 / 32, "q5_1": 24 / 32,
       "q5_0": 22 / 32, "q4_1": 20 / 32, "q4_0": 18 / 32, "iq4_nl": 18 / 32}


# ---------- the machine ----------

def devices():
    """The GPU working set, read from llama.cpp rather than guessed from RAM."""
    out = subprocess.run(["llama-server", "--list-devices"],
                         capture_output=True, text=True).stdout
    found = []
    for line in out.splitlines():
        m = re.search(r"^\s*(\w+):\s*(.+?)\s*\((\d+) MiB, (\d+) MiB free\)", line)
        if m and int(m.group(3)) > 0:
            found.append({"id": m.group(1), "name": m.group(2),
                          "total_mib": int(m.group(3)), "free_mib": int(m.group(4))})
    return found


def host_memory():
    ps = 4096
    stats = subprocess.run(["vm_stat"], capture_output=True, text=True).stdout
    m = re.search(r"page size of (\d+)", stats)
    if m:
        ps = int(m.group(1))
    def pages(label):
        m = re.search(rf"{label}:\s+(\d+)", stats)
        return int(m.group(1)) if m else 0
    free = (pages("Pages free") + pages("Pages inactive") + pages("Pages speculative")) * ps
    swap = subprocess.run(["sysctl", "-n", "vm.swapusage"], capture_output=True, text=True).stdout
    used = re.search(r"used = ([\d.]+)M", swap)
    total = re.search(r"total = ([\d.]+)M", swap)
    installed = int(subprocess.run(["sysctl", "-n", "hw.memsize"],
                                   capture_output=True, text=True).stdout)
    return {"installed": installed, "available": free,
            "swap_used_mib": float(used.group(1)) if used else 0.0,
            "swap_total_mib": float(total.group(1)) if total else 0.0}


# ---------- the model ----------

def header(path):
    f = open(path, "rb")
    if f.read(4) != b"GGUF":
        raise SystemExit(f"{path}: not a GGUF file")
    struct.unpack("<I", f.read(4)); f.read(8)
    n, = struct.unpack("<Q", f.read(8))
    def rs():
        l, = struct.unpack("<Q", f.read(8)); return f.read(l).decode("utf-8", "replace")
    def rv(t):
        if t in (0, 1, 7): return f.read(1)[0]
        if t in (2, 3): return struct.unpack("<H", f.read(2))[0]
        if t in (4, 5): return struct.unpack("<I", f.read(4))[0]
        if t == 6: return struct.unpack("<f", f.read(4))[0]
        if t == 8: return rs()
        if t in (10, 11): return struct.unpack("<Q", f.read(8))[0]
        if t == 12: return struct.unpack("<d", f.read(8))[0]
        if t == 9:
            et, = struct.unpack("<I", f.read(4)); ln, = struct.unpack("<Q", f.read(8))
            for _ in range(ln): rv(et)
            return None
        raise SystemExit(f"unknown gguf value type {t}")
    kv = {}
    for _ in range(n):
        k = rs(); t, = struct.unpack("<I", f.read(4)); kv[k] = rv(t)
    a = kv.get("general.architecture", "unknown")
    g = lambda s: kv.get(f"{a}.{s}")
    kd = g("attention.key_length") or (
        (g("embedding_length") // g("attention.head_count"))
        if g("embedding_length") and g("attention.head_count") else None)
    return {"path": path, "name": os.path.basename(path), "arch": a,
            "bytes": os.path.getsize(path), "layers": g("block_count"),
            "kv_heads": g("attention.head_count_kv"), "kd": kd,
            "vd": g("attention.value_length") or kd,
            "interval": g("full_attention_interval") or g("attention.sliding_window_pattern"),
            "window": g("attention.sliding_window"), "max_ctx": g("context_length")}


def cache_mib(m, ctx, cache):
    """Only layers that hold a context-scaled cache are charged."""
    layers = m["layers"] or 0
    full = layers // m["interval"] if m["interval"] else layers
    per = m["kv_heads"] * (m["kd"] * BPE[cache] + m["vd"] * BPE[cache])
    return full * ctx * per / MIB, full


# ---------- verdicts ----------

def ladder(m, gpu_mib, avail_mib):
    budget = gpu_mib - MARGIN_MIB
    weights = m["bytes"] / MIB
    rows = []
    contexts = [c for c in (4096, 8192, 16384, 32768, 65536, 131072, 262144)
                if c <= (m["max_ctx"] or 0)]
    if m["max_ctx"] and m["max_ctx"] not in contexts:
        contexts.append(m["max_ctx"])
    for cache in ("q8_0", "f16"):
        for ctx in contexts:
            kvm, full = cache_mib(m, ctx, cache)
            need = weights + kvm + 96          # a rough compute-buffer allowance
            # Weights are mmapped: they page from disk rather than having to be
            # resident, which is why a 21 GB model runs at 5 GB resident. What must be
            # real memory is the cache and the compute buffers.
            resident = kvm + 96
            rows.append({"ctx": ctx, "cache": cache, "kv_mib": kvm, "need_mib": need,
                         "resident_mib": resident,
                         "on_gpu": need <= budget, "in_free_ram": resident <= avail_mib,
                         "attention_layers": full})
    return rows


def candidates(rows, on_gpu):
    """A few that fit, spread across the axes that actually differ: how much context,
    and how precise the cache. Measuring every rung would take all afternoon."""
    fits = [r for r in rows if r["on_gpu"]]
    picks, seen = [], set()
    wanted = [on_gpu]
    small = [r for r in fits if r["ctx"] <= 8192]
    if small:
        wanted.append(max(small, key=lambda r: (r["cache"] == "f16", r["ctx"])))
    mid = [r for r in fits if 16384 <= r["ctx"] <= 65536]
    if mid:
        wanted.append(max(mid, key=lambda r: (r["cache"] == "f16", r["ctx"])))
    other = [r for r in fits if r["ctx"] == on_gpu["ctx"] and r["cache"] != on_gpu["cache"]]
    wanted += other
    for r in wanted:
        key = (r["ctx"], r["cache"])
        if key not in seen:
            seen.add(key)
            picks.append(r)
    return picks


def best(rows, key):
    ok = [r for r in rows if r[key]]
    if not ok:
        return None
    return max(ok, key=lambda r: (r["ctx"], r["cache"] == "f16"))


# ---------- measuring ----------

# A prompt long enough that prompt-eval measures throughput rather than startup. Built
# from varied text rather than one repeated line, because a repeated line is not what a
# coding agent sends and is not what the cache does with it.
_WORDS = ("model context window memory cache attention layer token embedding gradient "
          "kernel buffer offload quantise inference throughput latency batch server "
          "prompt decode residual weights tensor matrix vector scalar precision").split()


def long_prompt(tokens):
    """Roughly `tokens` tokens of prose. English runs near 0.75 words per token."""
    words = max(32, int(tokens * 0.75))
    out = []
    for i in range(words):
        out.append(_WORDS[(i * 7 + i // len(_WORDS)) % len(_WORDS)])
        if i % 12 == 11:
            out.append(".\n")
    return " ".join(out)


def measure(m, ctx, cache, prompt, port=9977):
    args = ["llama-server", "-m", m["path"], "--host", "127.0.0.1", "--port", str(port),
            "-c", str(ctx), "--cache-type-k", cache, "--cache-type-v", cache,
            "--flash-attn", "on", "-np", "1", "--no-warmup"]
    proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    base = f"http://127.0.0.1:{port}"
    try:
        for _ in range(600):
            if proc.poll() is not None:
                return {"error": (proc.stderr.read() or "").strip().splitlines()[-1:] or ["exited"]}
            try:
                with urllib.request.urlopen(f"{base}/health", timeout=2) as r:
                    if r.status == 200:
                        break
            except Exception:
                time.sleep(0.5)
        body = json.dumps({"prompt": prompt, "n_predict": 64, "stream": False,
                           "cache_prompt": False}).encode()
        req = urllib.request.Request(f"{base}/completion", data=body,
                                     headers={"Content-Type": "application/json"})
        started = time.time()
        with urllib.request.urlopen(req, timeout=300) as r:
            out = json.loads(r.read())
        t = out.get("timings", {})
        rss = subprocess.run(["ps", "-o", "rss=", "-p", str(proc.pid)],
                             capture_output=True, text=True).stdout.strip()
        return {"gen_tps": t.get("predicted_per_second"), "prompt_tps": t.get("prompt_per_second"),
                "prompt_n": t.get("prompt_n"), "wall_s": round(time.time() - started, 1),
                "rss_gb": round(int(rss) / 1024 / 1024, 1) if rss else None}
    finally:
        proc.terminate()
        try: proc.wait(timeout=10)
        except Exception: proc.kill()


# ---------- report ----------

def gb(mib): return mib * MIB / 1000 ** 3

def main():
    paths = [a for a in sys.argv[1:] if not a.startswith("--")]
    do_run = "--run" in sys.argv
    if not paths:
        raise SystemExit(__doc__)

    devs = devices()
    host = host_memory()
    gpu = devs[0] if devs else None
    avail_mib = host["available"] / MIB

    print("MACHINE")
    print(f"  installed          {host['installed']/1000**3:6.2f} GB")
    if gpu:
        print(f"  {gpu['id']} working set   {gb(gpu['total_mib']):6.2f} GB   ({gpu['name']})")
        print(f"  usable after margin{gb(gpu['total_mib']-MARGIN_MIB):6.2f} GB   llama.cpp keeps {MARGIN_MIB} MiB free")
    print(f"  free right now     {gb(avail_mib):6.2f} GB")
    print(f"  swap in use        {host['swap_used_mib']/1024:6.2f} GB of {host['swap_total_mib']/1024:.2f} GB")

    for p in paths:
        m = header(p)
        rows = ladder(m, gpu["total_mib"] if gpu else 0, avail_mib)
        held = f"1 layer in {m['interval']} holds a cache" if m["interval"] else "every layer holds a cache"
        print(f"\n{m['name']}")
        print(f"  {m['bytes']/1000**3:.2f} GB weights · {m['layers']} layers, {held} · max ctx {m['max_ctx']:,}")

        on_gpu = best(rows, "on_gpu")
        in_ram = best(rows, "in_free_ram")
        print(f"\n  {'context':>9}  {'cache':<6} {'cache size':>10} {'total':>9}   on GPU   room now")
        for r in rows:
            star = " <- best on GPU" if on_gpu and r is on_gpu else ""
            print(f"  {r['ctx']:>9,}  {r['cache']:<6} {gb(r['kv_mib']):>7.2f} GB {gb(r['need_mib']):>7.2f} GB"
                  f"   {'yes' if r['on_gpu'] else 'NO ':>5}    {'yes' if r['in_free_ram'] else 'NO ':>5}{star}")

        print()
        if on_gpu:
            print(f"  best that fits the GPU      : {on_gpu['ctx']:,} ctx, {on_gpu['cache']} cache")
        else:
            print("  nothing fits the GPU; every option would spill to the CPU")
        if in_ram:
            print(f"  cache has room right now up to: {in_ram['ctx']:,} ctx, {in_ram['cache']} cache")
            print(f"  weights ({m['bytes']/1000**3:.1f} GB) are mmapped and page from disk; with"
                  f" {gb(avail_mib):.1f} GB free they will not all stay resident, which costs speed, not correctness")
        else:
            print("  nothing has room right now even for its cache — close something")

        if do_run and on_gpu:
            print("\n  MEASURING — a memory sum says a launch is allowed, never that it is good")
            cands = candidates(rows, on_gpu)
            # One prompt for every candidate, sized to the smallest context so each does
            # identical work. Comparing configs on different prompts compares nothing.
            budget = min(c["ctx"] for c in cands)
            prompt = long_prompt(min(4096, max(256, budget // 2)))
            print(f"  same prompt for each, about {min(4096, max(256, budget//2)):,} tokens\n")
            results = []
            for cand in cands:
                print(f"    {cand['ctx']:>9,} ctx / {cand['cache']:<5} ...", end="", flush=True)
                got = measure(m, cand["ctx"], cand["cache"], prompt)
                if got.get("error"):
                    print(f" did not run: {got['error']}")
                    continue
                got.update(ctx=cand["ctx"], cache=cand["cache"])
                results.append(got)
                print(f" {got['gen_tps']:6.1f} tok/s generation · {got['prompt_tps']:7.1f} prompt"
                      f" ({got['prompt_n']} tokens) · {got['rss_gb']} GB resident")
            if results:
                quickest_prompt = max(results, key=lambda r: r["prompt_tps"])
                fastest = max(results, key=lambda r: r["gen_tps"])
                widest = max(results, key=lambda r: r["ctx"])
                print(f"\n  fastest        : {fastest['ctx']:,} ctx / {fastest['cache']}"
                      f"  at {fastest['gen_tps']:.1f} tok/s")
                if fastest is not widest:
                    lost = (1 - widest["gen_tps"] / fastest["gen_tps"]) * 100
                    print(f"  widest context : {widest['ctx']:,} ctx / {widest['cache']}"
                          f"  at {widest['gen_tps']:.1f} tok/s — {lost:.0f}% slower for"
                          f" {widest['ctx']-fastest['ctx']:,} more tokens")
                else:
                    print("  the widest context was also the fastest — nothing was traded")
                print(f"  quickest prompt: {quickest_prompt['ctx']:,} ctx /"
                      f" {quickest_prompt['cache']}  at {quickest_prompt['prompt_tps']:.1f} tok/s"
                      f"  — the number that matters for an agent sending long context")

if __name__ == "__main__":
    main()
