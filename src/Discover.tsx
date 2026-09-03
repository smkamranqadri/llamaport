import { useCallback, useEffect, useState } from "react";
import { discoverBrowse, discoverDownload, downloadStart } from "./api";
import DiscoverDetail from "./DiscoverDetail";
import { formatFileSize } from "./format";
import { CloseIcon, DownloadIcon, SearchIcon } from "./icons";
import type { DiscoverRow, DiscoverSort } from "./types";

/// Sorts and filters are different questions and stopped sharing a widget on 2026-09-03,
/// after the author used the version where they did. A sort says what order; a filter says
/// what is left. Coding and Chat are still absent and still measured: over the fifty most
/// trending GGUF repositories the `code` tag appears on none and `conversational` on
/// forty-six.
const SORTS = [
  { id: "trending" as DiscoverSort, label: "Trending", heading: "Trending on Hugging Face" },
  {
    id: "downloads" as DiscoverSort,
    label: "Most downloaded",
    heading: "Most downloaded in the last month",
  },
  { id: "likes" as DiscoverSort, label: "Most liked", heading: "Most liked on Hugging Face" },
] as const;

/// Smallest first is an ordering, not a claim about what small is, so it belongs here and
/// not among the filters. It sorts what the page already holds rather than asking the API,
/// because the API cannot sort by a size it never returns.
const BY_SIZE = "smallest";
type SortId = (typeof SORTS)[number]["id"] | typeof BY_SIZE;

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

export default function Discover({
  onShowDownloads,
}: {
  onShowDownloads: () => void;
}) {
  const [sort, setSort] = useState<SortId>("trending");
  const [onlyFits, setOnlyFits] = useState(true);
  const [onlyMoe, setOnlyMoe] = useState(false);
  const [rows, setRows] = useState<DiscoverRow[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [more, setMore] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [got, setGot] = useState<string | null>(null);
  const [opened, setOpened] = useState<string | null>(null);

  // Smallest first is not something the API can do, so it browses trending and reorders.
  const asked: DiscoverSort = sort === BY_SIZE ? "trending" : sort;
  const chosen = SORTS.find((entry) => entry.id === asked) ?? SORTS[0];

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
    load(asked, searched);
  }, [asked, searched, load]);

  /// Appends rather than replaces, and only ever follows the cursor the last page gave —
  /// rebuilding the query would come back sorted differently from what is already on screen.
  const loadMore = () => {
    if (!next) return;
    setMore(true);
    discoverBrowse(asked, searched, next)
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
    const label = row.pick.candidate.label;
    discoverDownload(row.id, row.pick.candidate.paths)
      .then(() => setGot(`${row.name} · ${label}`))
      .catch((e) => setFailure(String(e)));
  };

  // Filters combine; the sort is applied after them. A row with no pick has no size to
  // judge, so it survives every filter and sorts last.
  let shown = rows;
  if (onlyFits) shown = shown.filter((row) => row.pick !== null && row.pick.fits !== false);
  if (onlyMoe) shown = shown.filter((row) => row.moe);
  if (sort === BY_SIZE) {
    shown = [...shown].sort(
      (a, b) =>
        (a.pick?.candidate.size ?? Number.MAX_SAFE_INTEGER) -
        (b.pick?.candidate.size ?? Number.MAX_SAFE_INTEGER),
    );
  }

  if (opened) {
    return (
      <DiscoverDetail
        repo={opened}
        onBack={() => setOpened(null)}
        onShowDownloads={onShowDownloads}
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
            {(query !== "" || searched) && (
              <button
                className="search-clear"
                type="button"
                title="Clear the search"
                onClick={() => {
                  setQuery("");
                  setSearched(null);
                }}
              >
                <CloseIcon />
              </button>
            )}
          </span>
        </form>
      </header>

      <div className="chip-row">
        <select
          className="sort-select"
          value={sort}
          onChange={(e) => setSort(e.currentTarget.value as SortId)}
        >
          {SORTS.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.label}
            </option>
          ))}
          <option value={BY_SIZE}>Smallest first</option>
        </select>
        <span className="chip-divider" />
        <button
          className={`chip${onlyFits ? " is-active" : ""}`}
          onClick={() => setOnlyFits((on) => !on)}
        >
          Fits this Mac
        </button>
        <button
          className={`chip${onlyMoe ? " is-active" : ""}`}
          title="Marked only where the file's architecture names it. The index carries no expert count, so some mixtures of experts are not marked and this filter will not show them."
          onClick={() => setOnlyMoe((on) => !on)}
        >
          MoE
        </button>
      </div>

      {failure && <p className="notice notice-error">{failure}</p>}
      {got && (
        <div className="notice notice-done">
          <span>
            <strong>{got}</strong> is downloading
          </span>
          <button className="button button-plain" onClick={onShowDownloads}>
            View progress
          </button>
          <button className="button button-plain" onClick={() => setGot(null)}>
            Dismiss
          </button>
        </div>
      )}

      <h2 className="group-label">
        {searched
          ? `Matching “${searched}”`
          : sort === BY_SIZE
            ? "Trending on Hugging Face, smallest first"
            : chosen.heading}
      </h2>

      {busy && (
        <div className="discover-list">
          {[0, 1, 2, 3, 4].map((slot) => (
            <div className="discover-row is-waiting" key={slot}>
              <span className="skeleton skeleton-name" />
              <span className="skeleton skeleton-size" />
            </div>
          ))}
        </div>
      )}

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
        {!busy &&
          shown.map((row) => {
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
                  {row.moe && <span className="badge badge-moe">MoE</span>}
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
