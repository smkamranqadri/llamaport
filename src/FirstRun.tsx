import { useEffect, useState } from "react";
import {
  downloadStart,
  downloadStatus,
  machineMemory,
  onDownloadState,
} from "./api";
import { formatFileSize, formatMemory } from "./format";
import { CubeIcon, DownloadIcon, SearchIcon } from "./icons";
import type { DirInfo, DownloadJob, MachineMemory } from "./types";

/// The three the app offers someone whose models directory is empty. Hard-coded on
/// purpose: a first run has no catalogue to search and nothing measured to rank by. The
/// byte counts are the ones Hugging Face reports for these exact files, so the size beside
/// a card is the size that will land.
const STARTERS = [
  {
    name: "Qwen 3.5 2B",
    badge: "starter",
    detail: "Light and quick — answers fast, good for trying things out.",
    bytes: 1_280_835_840,
    url: "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
  },
  {
    name: "Qwen 3.6 35B A3B",
    badge: "popular",
    detail:
      "A big model that answers like a small one: only 3B of its 35B run at a time.",
    bytes: 22_134_528_992,
    url: "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF/resolve/main/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
  },
  {
    name: "Qwen 3.8 27B",
    badge: "strongest",
    detail: "Smartest of the three; a little slower to answer.",
    bytes: 16_464_440_224,
    url: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/Qwen3.8-27B-UD-Q4_K_M.gguf",
  },
];

// Weights are only part of what a launch asks for — the cache grows with the context, and
// llama.cpp keeps a margin below the ceiling besides. Half the ceiling leaves room for a
// long conversation, three quarters for a short one, and a file that clears the ceiling
// itself still runs, with almost nothing left to hold a conversation in. Only past the
// ceiling is a launch actually refused, so only there does the card say so.
const EASY = 0.5;
const ROOMY = 0.75;

export default function FirstRun({ dir }: { dir: DirInfo | null }) {
  const [memory, setMemory] = useState<MachineMemory | null>(null);
  const [jobs, setJobs] = useState<DownloadJob[]>([]);
  const [failure, setFailure] = useState<string | null>(null);
  const [link, setLink] = useState("");

  useEffect(() => {
    machineMemory().then(setMemory).catch(() => {});
    downloadStatus().then(setJobs).catch(() => {});
    const stop = onDownloadState(setJobs);
    return () => {
      void stop.then((off) => off());
    };
  }, []);

  // Installed memory is the wrong ceiling — nothing allocates the whole machine, and on an
  // M2 Pro the GPU's own limit is 7 GB below it. It is used only when llama-server has not
  // been found yet, and the sentence says so rather than passing it off as the real one.
  const budget = memory?.deviceBudgetBytes ?? null;
  const ceiling = budget ?? memory?.installedBytes ?? null;

  const start = (url: string) => {
    setFailure(null);
    downloadStart(url).then(setJobs).catch((e) => setFailure(String(e)));
  };

  const submitLink = (e: React.FormEvent) => {
    e.preventDefault();
    const target = link.trim();
    if (!target) return;
    setLink("");
    start(target);
  };

  const coming = jobs.filter(
    (job) => job.state === "active" || job.state === "queued",
  );

  let lead = "One click downloads it; press Run when it lands.";
  if (budget != null) {
    lead = `Sized against the ${formatMemory(budget)} this Mac's GPU can hold. ${lead}`;
  } else if (ceiling != null) {
    lead = `Sized against this Mac's ${formatMemory(ceiling)} of memory — llama-server has not been found, so its GPU limit, which is lower, is unknown. ${lead}`;
  }

  return (
    <div className="first-run">
      <span className="first-run-mark">
        <CubeIcon />
      </span>
      <h2 className="first-run-title">Get your first model</h2>
      <p className="first-run-lead">{lead}</p>

      {failure && <p className="notice notice-error">{failure}</p>}

      <div className="starters">
        {STARTERS.map((starter) => {
          const job = jobs.find((j) => j.url === starter.url);
          const running = job?.state === "active" || job?.state === "queued";

          // No ceiling, no verdict. Printing "fits" from a figure the app does not have is
          // the failure this project keeps finding by looking at the screen.
          let verdict = "";
          let tight = false;
          if (ceiling != null) {
            verdict = " · fits";
            if (starter.bytes <= ceiling * EASY) {
              verdict = " · fits easily";
            } else if (starter.bytes > ceiling) {
              verdict = " · too big for this Mac";
              tight = true;
            } else if (starter.bytes > ceiling * ROOMY) {
              verdict = " · tight — little room for a conversation";
              tight = true;
            }
          }

          return (
            <div className="card starter" key={starter.url}>
              <span className="starter-head">
                <span className="starter-name">{starter.name}</span>
                <span className="badge">{starter.badge}</span>
              </span>
              <span className="card-sub">{starter.detail}</span>
              <span className={`field-hint${tight ? " tone-warn" : ""}`}>
                {formatFileSize(starter.bytes)}
                {verdict}
              </span>
              <span className="starter-action">
                <button
                  className={`button${tight ? "" : " button-primary"}`}
                  disabled={running || job?.state === "complete"}
                  onClick={() => start(starter.url)}
                >
                  <DownloadIcon />
                  {running ? "Downloading…" : "Download"}
                </button>
              </span>
            </div>
          );
        })}
      </div>

      <form className="first-run-link" onSubmit={submitLink}>
        <span className="field-hint">Already know what you want?</span>
        <span className="link-field">
          <SearchIcon />
          <input
            value={link}
            onChange={(e) => setLink(e.target.value)}
            placeholder="Paste a Hugging Face link"
          />
        </span>
        <button className="button" type="submit" disabled={!link.trim()}>
          <DownloadIcon />
          Get
        </button>
      </form>

      {coming.length > 0 && (
        <p className="field-hint">
          {coming.length === 1
            ? `${coming[0].fileName} is on its way`
            : `${coming.length} downloads are on their way`}
          . Models appear in this list as they land; Downloads has the progress.
        </p>
      )}

      {dir != null && !dir.exists && (
        <p className="field-hint">
          {dir.path} does not exist yet — it is created when a download starts.
        </p>
      )}
    </div>
  );
}
