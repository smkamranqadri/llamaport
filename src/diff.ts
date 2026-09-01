export type DiffLine = { kind: "same" | "add" | "remove"; text: string };

/// A longest-common-subsequence diff over lines.
///
/// The panel writes into files the user maintains by hand, so showing two JSON blobs and
/// letting them find the difference is not good enough: the point of the confirm is that
/// they can see exactly which lines move.
export function lineDiff(before: string, after: string): DiffLine[] {
  const a = before.length === 0 ? [] : before.split("\n");
  const b = after.split("\n");

  const lengths: number[][] = Array.from({ length: a.length + 1 }, () =>
    new Array<number>(b.length + 1).fill(0),
  );
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      lengths[i][j] =
        a[i] === b[j]
          ? lengths[i + 1][j + 1] + 1
          : Math.max(lengths[i + 1][j], lengths[i][j + 1]);
    }
  }

  const lines: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      lines.push({ kind: "same", text: a[i] });
      i += 1;
      j += 1;
    } else if (lengths[i + 1][j] >= lengths[i][j + 1]) {
      lines.push({ kind: "remove", text: a[i] });
      i += 1;
    } else {
      lines.push({ kind: "add", text: b[j] });
      j += 1;
    }
  }
  while (i < a.length) {
    lines.push({ kind: "remove", text: a[i] });
    i += 1;
  }
  while (j < b.length) {
    lines.push({ kind: "add", text: b[j] });
    j += 1;
  }
  return lines;
}

export function changedCount(lines: DiffLine[]) {
  return {
    added: lines.filter((line) => line.kind === "add").length,
    removed: lines.filter((line) => line.kind === "remove").length,
  };
}
