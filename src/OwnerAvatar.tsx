import { useEffect, useState } from "react";
import { discoverAvatar } from "./api";
import { OwnerIcon } from "./icons";

/// One request per owner for the life of the process, shared by every screen that draws a
/// row. A miss is remembered as firmly as a hit: the owners with no picture are the ones
/// publishing the most repositories, so they would otherwise be asked for the most often.
/// Rust holds the same map and a copy on disk; this one only stops the same render asking
/// twice before either answers.
const known = new Map<string, string | null>();
const inflight = new Map<string, Promise<string | null>>();

function lookup(owner: string): Promise<string | null> {
  const settled = inflight.get(owner);
  if (settled) return settled;
  const asking = discoverAvatar(owner)
    .catch(() => null)
    .then((found) => {
      known.set(owner, found);
      inflight.delete(owner);
      return found;
    });
  inflight.set(owner, asking);
  return asking;
}

/// The publisher's picture, or one generic mark. Deliberately not a coloured initial: a
/// letter invents a distinction between owners the app has no basis for, and a row whose
/// origin is unknown should look the same as every other unknown.
export default function OwnerAvatar({
  owner,
  small,
}: {
  owner: string | null;
  small?: boolean;
}) {
  const [uri, setUri] = useState<string | null>(() =>
    owner ? (known.get(owner) ?? null) : null,
  );

  useEffect(() => {
    if (!owner || known.has(owner)) {
      setUri(owner ? (known.get(owner) ?? null) : null);
      return;
    }
    let live = true;
    lookup(owner).then((found) => {
      if (live) setUri(found);
    });
    return () => {
      live = false;
    };
  }, [owner]);

  return (
    <span
      className={`owner-avatar${small ? " is-small" : ""}`}
      title={owner ?? undefined}
    >
      {uri ? <img src={uri} alt="" /> : <OwnerIcon />}
    </span>
  );
}
