import { useCallback, useEffect, useState } from "react";
import { discoverBrowse, discoverDownload, downloadStart } from "./api";
import DiscoverDetail from "./DiscoverDetail";
import { formatFileSize } from "./format";
import { DownloadIcon, SearchIcon } from "./icons";
import type { DiscoverRow, DiscoverSort } from "./types";

/// Four lists, and every one of them is something the API or this machine can actually
/// answer. Coding and Chat were drawn and are not here: over the fifty most trending GGUF
/// repositories the `code` tag appears on none of them and `conversational` on forty-six,
/// so one filter would have had nothing behind it and the other would have removed four
/// rows in fifty.
const LISTS = [
  {
    id: "fits",
    label: "Fits this Mac",
    sort: "trending" as DiscoverSort,
    heading: "Trending on Hugging Face, and small enough for this Mac",
  },
  {
    id: "small",
    label: "Small & fast",
    sort: "trending" as DiscoverSort,
    heading: "What fits this Mac, smallest first",
  },
  {
    id: "downloads",
    label: "Most downloaded",
    sort: "downloads" as DiscoverSort,
    heading: "Most downloaded in the last month",
  },
  {
    id: "likes",
    label: "Most liked",
    sort: "likes" as DiscoverSort,
    heading: "Most liked on Hugging Face",
  },
] as const;

type ListId = (typeof LISTS)[number]["id"];

function compact(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1000) return `${Math.round(count / 1000)}k`;
  return String(count);
}

function updated(iso: string | null): string | null {
  if (!iso) return null;
  const at = Date.parse(iso);
  if (Number.isNaN(at)) return null;
  const days = Math.floor((Date.now() - at) / 86_400_000);
  if (days <= 0) return "updated today";
  if (days === 1) return "updated yesterday";
  if (days < 30) return `updated ${days} days ago`;
  const months = Math.round(days / 30);
  if (months < 12) return `updated ${months} mo ago`;
  return `updated ${Math.round(days / 365)} yr ago`;
}

/// The size, and never the word "fits".
///
/// **The ceiling used to be printed beside it and had to go.** A row read "25.1 GB of
/// 25.0 GB" while claiming to fit, because the size counts in decimal GB — what Finder and
/// Hugging Face show for the same file — and the ceiling counted in binary GiB, which is
/// what Activity Monitor shows for the same machine. Both printed "GB". One figure cannot
/// be checked against the other, so only one is shown.
///
/// All this knows is a file size — the model is not on disk, so there is no header to read
/// and no cache to charge. Every term left out moves what a launch really needs upwards,
/// so "will not fit" is safe to say and its opposite is not.
function Sizing({ row }: { row: DiscoverRow }) {
  if (!row.pick) return <span className="discover-note">{row.note}</span>;

  return (
    <span className="discover-size">
      <span className="badge quant-badge">{row.pick.candidate.label}</span>
      <span className={row.pick.fits === false ? "discover-over" : undefined}>
        {formatFileSize(row.pick.candidate.size)}
      </span>
      {row.pick.fits === false && (
        <span className="discover-note">over this Mac</span>
      )}
      {row.pick.fits === null && (
        <span className="discover-note">llama-server not found, so no size check</span>
      )}
    </span>
  );
}

export default function Discover() {
  const [list, setList] = useState<ListId>("fits");
  const [rows, setRows] = useState<DiscoverRow[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [more, setMore] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [got, setGot] = useState<string | null>(null);
  const [opened, setOpened] = useState<string | null>(null);

  const chosen = LISTS.find((entry) => entry.id === list) ?? LISTS[0];

  const load = useCallback((sort: DiscoverSort, search: string | null) => {
    setBusy(true);
    setFailure(null);
    discoverBrowse(sort, search, null)
      .then((page) => {
        setRows(page.rows);
        setNext(page.next);
      })
      .catch((e) => {
        setRows([]);
        setNext(null);
        setFailure(String(e));
      })
      .finally(() => setBusy(false));
  }, []);

  useEffect(() => {
    load(chosen.sort, searched);
  }, [chosen.sort, searched, load]);

  /// Appends rather than replaces, and only ever follows the cursor the last page gave —
  /// rebuilding the query would come back sorted differently from what is already on screen.
  const loadMore = () => {
    if (!next) return;
    setMore(true);
    discoverBrowse(chosen.sort, searched, next)
      .then((page) => {
        setRows((prev) => [...prev, ...page.rows]);
        setNext(page.next);
      })
      .catch((e) => setFailure(String(e)))
      .finally(() => setMore(false));
  };

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const typed = query.trim();
    // Pasting a link is Downloads' job and has been since before this screen existed, so
    // it hands straight over rather than growing a second way to do the same thing.
    if (typed.startsWith("https://")) {
      setBusy(true);
      setFailure(null);
      downloadStart(typed)
        .then(() => {
          setGot("Added to Downloads");
          setQuery("");
        })
        .catch((e) => setFailure(String(e)))
        .finally(() => setBusy(false));
      return;
    }
    setGot(null);
    setSearched(typed === "" ? null : typed);
  };

  const get = (row: DiscoverRow) => {
    if (!row.pick) return;
    setFailure(null);
    discoverDownload(row.id, row.pick.candidate.paths)
      .then(() => setGot(`${row.name} — ${row.pick?.candidate.label} added to Downloads`))
      .catch((e) => setFailure(String(e)));
  };

  // Both of these filter to what fits and differ only in order, which is what makes them
  // two questions rather than one: what can this Mac run, and what can it run quickest. A
  // chip that says small must never list something over the ceiling.
  const fitting = rows.filter((row) => row.pick !== null && row.pick.fits !== false);
  let shown = rows;
  if (list === "fits") shown = fitting;
  if (list === "small") {
    shown = [...fitting].sort(
      (a, b) => (a.pick?.candidate.size ?? 0) - (b.pick?.candidate.size ?? 0),
    );
  }

  if (opened) {
    return (
      <DiscoverDetail
        repo={opened}
        onBack={() => setOpened(null)}
        onQueued={(message) => {
          setGot(message);
          setOpened(null);
        }}
      />
    );
  }

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Discover</h1>
          <p className="screen-subtitle">Find a model on Hugging Face</p>
        </div>
        <form className="get-row" onSubmit={submit}>
          <span className="search-field get-field discover-field">
            <SearchIcon />
            <input
              value={query}
              placeholder="Search models, or paste a link"
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </span>
        </form>
      </header>

      <div className="chip-row">
        {LISTS.map((entry) => (
          <button
            key={entry.id}
            className={`chip${entry.id === list ? " is-active" : ""}`}
            onClick={() => setList(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </div>

      {failure && <p className="notice notice-error">{failure}</p>}
      {got && <p className="notice">{got}</p>}

      <h2 className="group-label">
        {searched ? `Matching “${searched}”` : chosen.heading}
      </h2>

      {busy && <p className="field-hint">Reading Hugging Face…</p>}

      {!busy && shown.length === 0 && (
        <div className="empty">
          <p className="empty-title">Nothing to show</p>
          <p className="empty-detail">
            {searched
              ? "No repository matched that. Hugging Face searches repository names, so fewer words find more."
              : "Hugging Face returned no repository this app can read."}
          </p>
        </div>
      )}

      <div className="discover-list">
        {shown.map((row) => {
          const facts = [`${compact(row.downloads)} downloads this month`];
          const when = updated(row.lastModified);
          if (when) facts.push(when);
          if (row.quants > 0) {
            facts.push(`${row.quants} quant${row.quants === 1 ? "" : "s"}`);
          }

          return (
            <div className="discover-row" key={row.id}>
              <button
                className="discover-body discover-open"
                title="Every quantisation, and what each one costs"
                onClick={() => setOpened(row.id)}
              >
                <span className="discover-head">
                  <span className="discover-name">{row.name}</span>
                  <span className="discover-owner">by {row.owner}</span>
                  {row.gated && <span className="badge badge-warn">Gated</span>}
                </span>
                <span className="card-sub">{facts.join(" · ")}</span>
              </button>
              <Sizing row={row} />
              <button
                className="button"
                disabled={!row.pick}
                title={row.note ?? undefined}
                onClick={() => get(row)}
              >
                <DownloadIcon />
                Download
              </button>
            </div>
          );
        })}
      </div>

      {next && !busy && (
        <button className="button discover-more" disabled={more} onClick={loadMore}>
          {more ? "Reading Hugging Face…" : "Load more"}
        </button>
      )}
    </>
  );
}
