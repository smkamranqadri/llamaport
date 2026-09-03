import { useEffect, useState } from "react";
import { discoverDownload, discoverRepo } from "./api";
import { formatContext, formatFileSize } from "./format";
import { ChevronLeftIcon, DownloadIcon } from "./icons";
import type { DiscoverDetail as Detail, QuantOffer } from "./types";

function params(count: number): string {
  if (count >= 1e9) return `${(count / 1e9).toFixed(count >= 1e11 ? 0 : 1)}B`;
  return `${Math.round(count / 1e6)}M`;
}

function counted(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1000) return `${Math.round(count / 1000)}k`;
  return String(count);
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
  onQueued,
}: {
  repo: string;
  onBack: () => void;
  onQueued: (message: string) => void;
}) {
  const [detail, setDetail] = useState<Detail | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    setDetail(null);
    setFailure(null);
    discoverRepo(repo).then(setDetail).catch((e) => setFailure(String(e)));
  }, [repo]);

  const get = (offer: QuantOffer) => {
    setFailure(null);
    discoverDownload(repo, offer.candidate.paths)
      .then(() => onQueued(`${offer.candidate.label} added to Downloads`))
      .catch((e) => setFailure(String(e)));
  };

  const facts: string[] = [];
  if (detail) {
    facts.push(`${counted(detail.facts.downloads)} downloads this month`);
    facts.push(`${counted(detail.facts.likes)} likes`);
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
            <p className="screen-subtitle">by {detail?.owner ?? "…"}</p>
          </div>
        </div>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {!detail && !failure && <p className="field-hint">Reading Hugging Face…</p>}

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
                    <button className="button" onClick={() => get(offer)}>
                      <DownloadIcon />
                      Download
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
