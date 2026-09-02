import { ChevronRightIcon } from "./icons";

/// A section a screen keeps folded: a title, one line saying what is inside, and the
/// full content only on demand. The redesign's launch screen is these rows — the memory
/// verdict, the speed history, the model's facts, the command — with only the choice of
/// how to run left open.
export default function Disclosure({
  title,
  sub,
  dot,
  flat,
  action,
  open,
  onToggle,
  children,
}: {
  title: string;
  sub?: string;
  /// Shown before the title, coloured by the verdict it carries.
  dot?: "ok" | "warn" | "bad";
  /// Without the card: a line on the page that happens to open.
  flat?: boolean;
  /// Sits at the right of the summary. Its clicks do not open the row.
  action?: React.ReactNode;
  open?: boolean;
  onToggle?: (open: boolean) => void;
  children: React.ReactNode;
}) {
  return (
    <details
      className={`disclosure${flat ? " is-flat" : ""}`}
      open={open}
      onToggle={(e) => onToggle?.(e.currentTarget.open)}
    >
      <summary>
        <ChevronRightIcon />
        {dot && <span className={`dot tone-${dot}`} />}
        <span className="d-title">{title}</span>
        {sub && <span className="d-sub">{sub}</span>}
        {action && (
          <span
            className="d-action"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
            }}
          >
            {action}
          </span>
        )}
      </summary>
      <div className="d-body">{children}</div>
    </details>
  );
}
