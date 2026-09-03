const GIB = 1024 ** 3;
const MIB = 1024 ** 2;

const GB = 1000 ** 3;
export const MB = 1000 ** 2;
const KB = 1000;

/// Files, disk space and anything transferred, counted the way Finder and Hugging Face
/// count: a figure here is meant to be compared with what those show for the same file,
/// so it has to be the same figure.
export function formatFileSize(bytes: number): string {
  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(1)} GB`;
  }
  return `${Math.round(bytes / MB)} MB`;
}

/// Memory, counted in the binary units the machine is sold and reported in: a 32 GB Mac
/// holds 32 GiB, and Activity Monitor says so too. The label stays GB because changing it
/// would disagree with every other reading of the same machine the user can reach.
export function formatMemory(bytes: number): string {
  if (bytes >= GIB) {
    return `${(bytes / GIB).toFixed(1)} GB`;
  }
  return `${Math.round(bytes / MIB)} MB`;
}

/// Decimal, with the download screen's rate field: what a limit is typed as and what it
/// reads back must divide by the same thing, or 10 comes back as 9.5.
export function formatRate(bytesPerSecond: number): string {
  if (bytesPerSecond >= GB) {
    return `${(bytesPerSecond / GB).toFixed(1)} GB/s`;
  }
  if (bytesPerSecond >= MB) {
    return `${(bytesPerSecond / MB).toFixed(1)} MB/s`;
  }
  return `${Math.round(bytesPerSecond / KB)} KB/s`;
}

/// Coarse on purpose. A transfer with half an hour left does not know that to the second,
/// and a figure that precise invites being believed.
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))}s`;

  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min`;

  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (rest === 0) return `${hours} hr`;
  return `${hours} hr ${rest} min`;
}

export function formatContext(tokens: number): string {
  if (tokens >= 1024) {
    return `${Math.round(tokens / 1024)}K`;
  }
  return String(tokens);
}

export function formatRelative(unixSeconds: number): string {
  const days = Math.floor((Date.now() / 1000 - unixSeconds) / 86400);
  if (days <= 0) return "today";
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;
  const months = Math.round(days / 30);
  if (months < 12) return `${months} mo ago`;
  return `${Math.round(days / 365)} yr ago`;
}

/// Download and like counts, rounded the way Hugging Face shows them.
export function formatCount(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1000) return `${Math.round(count / 1000)}k`;
  return String(count);
}
