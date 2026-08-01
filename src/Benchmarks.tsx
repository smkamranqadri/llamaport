import { useCallback, useEffect, useMemo, useState } from "react";
import {
  benchmarkDelete,
  benchmarkNote,
  benchmarksExport,
  benchmarksList,
} from "./api";
import { formatBytes, formatContext } from "./format";
import type { BenchmarkRecord, BenchmarkSort } from "./types";

const SORTS: { id: BenchmarkSort; label: string }[] = [
  { id: "timestamp", label: "Date" },
  { id: "genTps", label: "Generation" },
  { id: "promptTps", label: "Prompt eval" },
  { id: "timeToFirstToken", label: "First token" },
  { id: "peakMemory", label: "Peak memory" },
];

function when(seconds: number) {
  return new Date(seconds * 1000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function num(value: number | null, digits = 1, suffix = "") {
  if (value == null) return "—";
  return `${value.toFixed(digits)}${suffix}`;
}

/// Positive means the second run is better on this measure; direction differs per field.
function delta(a: number | null, b: number | null, higherIsBetter: boolean) {
  if (a == null || b == null || a === 0) return null;
  const change = ((b - a) / a) * 100;
  return { change, better: higherIsBetter ? change > 0 : change < 0 };
}

function Comparison({ pair }: { pair: [BenchmarkRecord, BenchmarkRecord] }) {
  const [a, b] = pair;
  const rows: [string, string, string, ReturnType<typeof delta>][] = [
    [
      "Generation",
      num(a.genTps, 1, " tok/s"),
      num(b.genTps, 1, " tok/s"),
      delta(a.genTps, b.genTps, true),
    ],
    [
      "Prompt eval",
      num(a.promptTps, 0, " tok/s"),
      num(b.promptTps, 0, " tok/s"),
      delta(a.promptTps, b.promptTps, true),
    ],
    [
      "First token",
      a.timeToFirstTokenMs == null ? "—" : `${a.timeToFirstTokenMs} ms`,
      b.timeToFirstTokenMs == null ? "—" : `${b.timeToFirstTokenMs} ms`,
      delta(a.timeToFirstTokenMs, b.timeToFirstTokenMs, false),
    ],
    [
      "Peak process memory",
      a.peakProcessBytes == null ? "—" : formatBytes(a.peakProcessBytes),
      b.peakProcessBytes == null ? "—" : formatBytes(b.peakProcessBytes),
      delta(a.peakProcessBytes, b.peakProcessBytes, false),
    ],
    [
      "Peak swap",
      a.peakSwapBytes == null ? "—" : formatBytes(a.peakSwapBytes),
      b.peakSwapBytes == null ? "—" : formatBytes(b.peakSwapBytes),
      delta(a.peakSwapBytes, b.peakSwapBytes, false),
    ],
  ];

  const differs = (left: unknown, right: unknown) => left !== right;

  return (
    <section className="panel">
      <h2>Comparison</h2>
      <p className="field-hint">
        {a.quantisation ?? "?"} at {formatContext(a.ctx)} versus{" "}
        {b.quantisation ?? "?"} at {formatContext(b.ctx)}.
        {(differs(a.ctx, b.ctx) ||
          differs(a.cacheTypeK, b.cacheTypeK) ||
          differs(a.cacheTypeV, b.cacheTypeV) ||
          differs(a.llamaVersion, b.llamaVersion)) &&
          " These runs differ in more than quantisation, so the difference is not attributable to one thing."}
      </p>

      <ul className="check-list">
        {rows.map(([label, left, right, change]) => (
          <li key={label} className="compare-row">
            <span className="check-name">{label}</span>
            <span className="telemetry-value">{left}</span>
            <span className="telemetry-value">{right}</span>
            <span
              className={`telemetry-value ${change ? (change.better ? "delta-better" : "delta-worse") : ""}`}
            >
              {change ? `${change.change > 0 ? "+" : ""}${change.change.toFixed(1)}%` : "—"}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export default function Benchmarks() {
  const [records, setRecords] = useState<BenchmarkRecord[]>([]);
  const [sort, setSort] = useState<BenchmarkSort>("timestamp");
  const [descending, setDescending] = useState(true);
  const [model, setModel] = useState<string>("");
  const [quant, setQuant] = useState<string>("");
  const [selected, setSelected] = useState<string[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const load = useCallback(() => {
    benchmarksList({
      modelFile: model === "" ? null : model,
      quantisation: quant === "" ? null : quant,
      sort,
      descending,
    })
      .then(setRecords)
      .catch((e) => setFailure(String(e)));
  }, [model, quant, sort, descending]);

  useEffect(load, [load]);

  // Filter options come from everything recorded, not the filtered view, or choosing
  // one value would hide every other.
  const [allRecords, setAllRecords] = useState<BenchmarkRecord[]>([]);
  useEffect(() => {
    benchmarksList().then(setAllRecords).catch(() => {});
  }, [records]);

  const models = useMemo(
    () => [...new Set(allRecords.map((r) => r.modelFile))].sort(),
    [allRecords],
  );
  const quants = useMemo(
    () =>
      [...new Set(allRecords.map((r) => r.quantisation).filter(Boolean))].sort(),
    [allRecords],
  );

  const pair = useMemo(() => {
    if (selected.length !== 2) return null;
    const found = selected
      .map((id) => allRecords.find((r) => r.id === id))
      .filter((r): r is BenchmarkRecord => r != null);
    return found.length === 2 ? ([found[0], found[1]] as const) : null;
  }, [selected, allRecords]);

  const toggle = (id: string) =>
    setSelected((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id].slice(-2),
    );

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Benchmarks</h1>
          <p className="screen-subtitle">
            {records.length} run{records.length === 1 ? "" : "s"} recorded ·
            select two to compare
          </p>
        </div>
        <div className="actions">
          <button
            className="button"
            onClick={() =>
              benchmarksExport("csv")
                .then((path) => setMessage(`Saved ${path}`))
                .catch((e) => setFailure(String(e)))
            }
          >
            Export CSV
          </button>
          <button
            className="button"
            onClick={() =>
              benchmarksExport("json")
                .then((path) => setMessage(`Saved ${path}`))
                .catch((e) => setFailure(String(e)))
            }
          >
            Export JSON
          </button>
        </div>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}
      {message && <p className="notice">{message}</p>}

      {allRecords.length === 0 ? (
        <div className="empty">
          <p className="empty-title">No runs recorded yet</p>
          <p className="empty-detail">
            Start a model and press “Test model” — each test records a row here.
          </p>
        </div>
      ) : (
        <>
          <section className="panel">
            <div className="filters">
              <label className="field">
                <span className="field-label">Model</span>
                <select value={model} onChange={(e) => setModel(e.currentTarget.value)}>
                  <option value="">All models</option>
                  {models.map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span className="field-label">Quantisation</span>
                <select value={quant} onChange={(e) => setQuant(e.currentTarget.value)}>
                  <option value="">All quantisations</option>
                  {quants.map((name) => (
                    <option key={name} value={name ?? ""}>
                      {name}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span className="field-label">Sort by</span>
                <select
                  value={sort}
                  onChange={(e) => setSort(e.currentTarget.value as BenchmarkSort)}
                >
                  {SORTS.map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span className="field-label">Direction</span>
                <button
                  className="button"
                  onClick={() => setDescending((value) => !value)}
                >
                  {descending ? "Descending" : "Ascending"}
                </button>
              </label>
            </div>
          </section>

          {pair && <Comparison pair={[pair[0], pair[1]]} />}

          <ul className="model-list">
            {records.map((record) => (
              <li key={record.id}>
                <div
                  className={`benchmark-row${selected.includes(record.id) ? " is-selected" : ""}`}
                >
                  <input
                    type="checkbox"
                    checked={selected.includes(record.id)}
                    onChange={() => toggle(record.id)}
                  />
                  <span className="model-identity">
                    <span className="model-name">{record.modelFile}</span>
                    <span className="model-file">
                      {when(record.timestampSecs)} · {record.quantisation ?? "?"} ·{" "}
                      {formatContext(record.ctx)} · {record.cacheTypeK}/
                      {record.cacheTypeV} · ngl {record.ngl} ·{" "}
                      {record.llamaVersion ?? "unknown build"}
                    </span>
                    {record.note && (
                      <span className="model-file">note: {record.note}</span>
                    )}
                  </span>

                  <span className="model-stat">{num(record.genTps, 1)} tok/s</span>
                  <span className="model-stat">{num(record.promptTps, 0)} tok/s</span>
                  <span className="model-stat">
                    {record.timeToFirstTokenMs == null
                      ? "—"
                      : `${record.timeToFirstTokenMs} ms`}
                  </span>
                  <span className="model-stat">
                    {record.peakProcessBytes == null
                      ? "—"
                      : formatBytes(record.peakProcessBytes)}
                  </span>

                  <span className="actions">
                    <button
                      className="button"
                      onClick={() => {
                        const note = window.prompt("Note for this run", record.note ?? "");
                        if (note === null) return;
                        benchmarkNote(record.id, note)
                          .then(() => load())
                          .catch((e) => setFailure(String(e)));
                      }}
                    >
                      Note
                    </button>
                    <button
                      className="button button-danger"
                      onClick={() =>
                        benchmarkDelete(record.id)
                          .then(() => {
                            setSelected((prev) => prev.filter((id) => id !== record.id));
                            load();
                          })
                          .catch((e) => setFailure(String(e)))
                      }
                    >
                      Delete
                    </button>
                  </span>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
    </>
  );
}
