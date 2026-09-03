import { useEffect, useState } from "react";
import { discoverDownload, discoverRepo } from "./api";
import { formatContext, formatCount, formatFileSize } from "./format";
import { ChevronLeftIcon, DownloadIcon } from "./icons";
import type { DiscoverDetail as Detail, QuantOffer } from "./types";

function params(count: number): string {
  if (count >= 1e9) return `${(count / 1e9).toFixed(count >= 1e11 ? 0 : 1)}B`;
  return `${Math.round(count / 1e6)}M`;
}

/// A green tick for what the weights clear and a red ring for what they do not — the only
/// verdict this page gives, because the file is not on disk and there is no header to read.
function Fit({ fits }: { fits: boolean | null }) {
  if (fits === null) return <span className="quant-fit is-unknown">?</span>;
  return (
    <span className={`quant-fit${fits ? " is-fitting" : " is-over"}`}>
      {fits ? "✓" : "✕"}
    </span>
  );
}

export default function DiscoverDetail({
  repo,
  onBack,
  onShowDownloads,
}: {
  repo: string;
  onBack: () => void;
  onShowDownloads: () => void;
}) {
  const [detail, setDetail] = useState<Detail | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [sent, setSent] = useState<string[]>([]);

  useEffect(() => {
    setDetail(null);
    setFailure(null);
    setSent([]);
    discoverRepo(repo).then(setDetail).catch((e) => setFailure(String(e)));
  }, [repo]);

  /// Stays on the page. Sending a download used to throw the reader back to the list with
  /// a line of text naming a quantisation and no model, which read as though something had
  /// gone wrong rather than right.
  const get = (offer: QuantOffer) => {
    setFailure(null);
    discoverDownload(repo, offer.candidate.paths)
      .then(() => setSent((prev) => [...prev, offer.candidate.label]))
      .catch((e) => setFailure(String(e)));
  };

  const facts: string[] = [];
  if (detail) {
    facts.push(`${formatCount(detail.facts.downloads)} downloads this month`);
    facts.push(`${formatCount(detail.facts.likes)} likes`);
    if (detail.facts.params) facts.push(`${params(detail.facts.params)} parameters`);
    if (detail.facts.architecture) facts.push(detail.facts.architecture);
    if (detail.facts.contextLength) {
      facts.push(`${formatContext(detail.facts.contextLength)} context`);
    }
    if (detail.facts.license) facts.push(detail.facts.license);
  }

  return (
    <>
      <header className="screen-header">
        <div className="detail-head">
          <button className="button button-plain" onClick={onBack}>
            <ChevronLeftIcon />
            Discover
          </button>
          <div>
            <h1>{detail?.name ?? repo}</h1>
            {/* The full repository id, because a name alone does not say which of the
                several repositories publishing this model you are looking at. */}
            <p className="screen-subtitle">{detail?.facts.id ?? repo}</p>
          </div>
        </div>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {sent.length > 0 && (
        <div className="notice notice-done">
          <span>
            <strong>
              {sent.length === 1 ? sent[0] : `${sent.length} quantisations`}
            </strong>{" "}
            {sent.length === 1 ? "is" : "are"} downloading
          </span>
          <button className="button button-plain" onClick={onShowDownloads}>
            View progress
          </button>
        </div>
      )}

      {!detail && !failure && (
        <div className="quant-list">
          {[0, 1, 2, 3, 4, 5].map((slot) => (
            <div className="quant-row is-waiting" key={slot}>
              <span className="skeleton skeleton-name" />
              <span className="skeleton skeleton-size" />
            </div>
          ))}
        </div>
      )}

      {detail && (
        <>
          <p className="card-sub detail-facts">{facts.join(" · ")}</p>

          {/* The parameter count comes off one file in the repository, so a repository
              whose first GGUF is a drafter reports a figure for the drafter. */}
          {detail.facts.params != null && detail.facts.params < 1e9 && (
            <p className="field-hint">
              Hugging Face reports this parameter count from a single file in the
              repository, which is not always the model.
            </p>
          )}

          {detail.note && <p className="notice">{detail.note}</p>}

          {detail.quants.length > 0 && (
            <>
              <h2 className="group-label">
                {detail.quants.length} quantisation
                {detail.quants.length === 1 ? "" : "s"}, largest first
              </h2>
              <div className="quant-list">
                {detail.quants.map((offer) => (
                  <div className="quant-row" key={offer.candidate.label}>
                    <Fit fits={offer.fits} />
                    <span className="quant-label">{offer.candidate.label}</span>
                    {offer.picked && <span className="badge">Our pick</span>}
                    {offer.candidate.paths.length > 1 && (
                      <span className="discover-note">
                        {offer.candidate.paths.length} files
                      </span>
                    )}
                    <span className="quant-size">
                      {formatFileSize(offer.candidate.size)}
                    </span>
                    <button
                      className="button"
                      disabled={sent.includes(offer.candidate.label)}
                      onClick={() => get(offer)}
                    >
                      <DownloadIcon />
                      {sent.includes(offer.candidate.label)
                        ? "Downloading"
                        : "Download"}
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}
        </>
      )}
    </>
  );
}
