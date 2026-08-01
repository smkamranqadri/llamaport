# Downloader — specification

The outstanding half of the original goal, unbuilt. Written before any code and
unchanged since: the analysis below still holds, and no part of it was
invalidated by the runtime work.

The problem it solves, in the user's words: `curl -L -C -` against a Hugging Face
URL "sometimes does not work, speed limit create problem and it does not resume",
which is why model downloads currently go through an external download manager.

## Design

### Why the current command fails

`https://huggingface.co/{repo}/resolve/main/{file}` returns a 302 to a CDN URL
carrying an expiring signature. `curl -C -` re-requests the *original* URL on
resume and re-enters the redirect chain, which is where resume breaks down. The
fix is to re-resolve the redirect on every resume attempt and issue `Range`
requests against a freshly signed URL.

Speed is a separate issue: a single connection is the bottleneck, not the line.
Multiple ranged connections are why an external download manager is faster,
and why `--limit-rate 10M` felt necessary in the first place.

### Engine

Per file:

1. Follow redirects manually to capture the final CDN URL and the headers
   `x-linked-size` (true size) and `x-linked-etag` (sha256 for LFS files).
   Confirm `accept-ranges: bytes`.
2. Preallocate a sparse `{name}.part`, split into 4–8 segments.
3. Each segment runs a ranged GET, writing at its own offset with positioned
   writes. No seeking coordination between tasks.
4. A sidecar `{name}.part.json` holds
   `{ source_url, total, etag, segments: [{ start, end, completed }] }`,
   flushed every ~2 s. This is what makes resume survive a process exit rather
   than only a pause.
5. On completion, optionally verify sha256 against the etag (a background pass;
   21 GB takes a minute or two), then rename `.part` to the final name.

Resume re-resolves the redirect, compares the etag against the sidecar, and
restarts each segment from its `completed` offset. An etag mismatch means the
upstream file changed: discard and restart.

### States

```
Queued → Active → Verifying → Complete
           ↕
        Paused
           ↓
        Failed → (manual resume) → Active
```

`Failed` is always resumable from the sidecar; it is a parked state, not a lost
transfer.

### Failure taxonomy

Retries have to distinguish three cases. Treating them uniformly means either
abandoning recoverable transfers or hammering a wall.

| Class | Examples | Response |
| --- | --- | --- |
| Transient | connection reset, 5xx, timeout | Retry the segment, exponential backoff, 5 attempts, then park the download in `Failed`. |
| Signature expiry | 403 on a CDN URL that was working | Not a failure. Re-resolve the redirect and continue. Routine on multi-hour transfers and should be invisible. |
| Fatal | 404, etag changed, no `accept-ranges` | Stop, explain, do not retry. |

Stall detection is separate and necessary: a segment reporting zero bytes for
30 s while sibling segments progress has hung without raising anything. Kill and
reissue that range. Without it a transfer can sit at 97% indefinitely.

### Gotchas to build in from the start

- **Strip `Authorization` on the cross-host redirect.** Forwarding the HF token
  to the CDN produces confusing 403s on gated repos.
- **Share one token bucket across all segments.** Otherwise a 10 MB/s rate limit
  becomes 10 MB/s × segment count.
- **One file at a time by default.** Parallel large files compete for the same
  pipe and multiply the failure surface.
- **Check free disk before enqueueing.** These are 13–21 GB files.

