import { useEffect, useState } from "react";
import { piApply, piPreview } from "./api";
import { changedCount, lineDiff } from "./diff";
import type { PiFileChange, PiPreview } from "./types";

const MARK = { same: " ", add: "+", remove: "−" };

function Diff({ change, written }: { change: PiFileChange; written: boolean }) {
  const lines = lineDiff(change.before ?? "", change.after);
  const { added, removed } = changedCount(lines);

  return (
    <div className="pi-file">
      <h4 className="pi-file-head">
        <span className="pi-path">{change.path}</span>
        {!written && (
          <span className="pi-counts">
            {added > 0 && <em className="pi-added">+{added}</em>}
            {removed > 0 && <em className="pi-removed">−{removed}</em>}
            {change.createsFile && <em className="pi-new">new file</em>}
          </span>
        )}
      </h4>
      <pre className="pi-json">
        {lines.map((line, index) => (
          <div key={index} className={`pi-line pi-${line.kind}`}>
            <span className="pi-mark">{MARK[line.kind]}</span>
            {line.text}
          </div>
        ))}
      </pre>
    </div>
  );
}

export default function PiPanel({ onClose }: { onClose: () => void }) {
  const [preview, setPreview] = useState<PiPreview | null>(null);
  const [reasoning, setReasoning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [written, setWritten] = useState(false);
  const [backed, setBacked] = useState<string[]>([]);
  const [writing, setWriting] = useState(false);

  useEffect(() => {
    piPreview()
      .then((seen) => {
        setPreview(seen);
        setReasoning(seen.reasoning);
      })
      .catch((reason) => setError(String(reason)));
  }, []);

  const confirm = () => {
    setWriting(true);
    setError(null);
    const existed = [preview?.provider, preview?.enabled]
      .filter((change) => change != null && !change.createsFile)
      .map((change) => change!.path);
    piApply(reasoning)
      .then((seen) => {
        setPreview(seen);
        setBacked(existed);
        setWritten(true);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setWriting(false));
  };

  return (
    <div className="pi-overlay" onClick={onClose}>
      <section className="pi-dialog" onClick={(event) => event.stopPropagation()}>
        <h3 className="pi-title">
          {written ? "pi can reach this model" : "Point pi at this model"}
        </h3>

        {error && <p className="notice notice-error">{error}</p>}

        {preview && (
          <>
            {preview.sharingPort.length > 0 && (
              <p className="field-hint">
                {preview.sharingPort.join(" and ")} already point at this port.
                Only one server can hold it, so whichever is running answers to
                those names too.
              </p>
            )}

            {preview.pruned.length > 0 && (
              <p className="field-hint">
                {written ? "Dropped" : "Drops"} {preview.pruned.join(", ")} —
                this provider lists one model, so the rest name models it no
                longer has.
              </p>
            )}

            <label className="pi-check">
              <input
                type="checkbox"
                checked={reasoning}
                disabled={written}
                onChange={(event) => setReasoning(event.target.checked)}
              />
              This model reasons
            </label>

            <Diff change={preview.provider} written={written} />
            <Diff change={preview.enabled} written={written} />

            {!written && (
              <p className="field-hint">
                The provider alone is not enough — pi will not offer a model
                until it is named in enabledModels.
              </p>
            )}
            {written && backed.length > 0 && (
              <p className="field-hint">
                {backed.length === 2 ? "Each file" : "The file"} as it was is
                beside it, as .llamaport.bak.
              </p>
            )}
          </>
        )}

        <div className="actions pi-actions">
          <button className="button" onClick={onClose}>
            {written ? "Close" : "Cancel"}
          </button>
          {!written && (
            <button
              className="button button-primary"
              disabled={preview == null || writing}
              onClick={confirm}
            >
              {writing ? "Writing…" : "Write both"}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
