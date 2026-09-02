function Icon({ path, size = 16 }: { path: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: path }}
    />
  );
}

export function StackIcon() {
  return (
    <Icon path='<path d="M8 2 L14 5 L8 8 L2 5 Z"/><path d="M2 8 L8 11 L14 8"/><path d="M2 11 L8 14 L14 11"/>' />
  );
}

export function CompassIcon() {
  return (
    <Icon path='<circle cx="8" cy="8" r="6"/><path d="M10.5 5.5 L9 9 L5.5 10.5 L7 7 Z"/>' />
  );
}

export function DownloadIcon() {
  return (
    <Icon path='<path d="M8 2 V10"/><path d="M4.5 7 L8 10.5 L11.5 7"/><path d="M2.5 13.5 H13.5"/>' />
  );
}

export function ChartIcon() {
  return (
    <Icon path='<path d="M2 13.5 H14"/><path d="M3 11 L6.5 7 L9.5 9.5 L13.5 3.5"/>' />
  );
}

export function SlidersIcon() {
  return (
    <Icon path='<path d="M2 5 H14"/><circle cx="10" cy="5" r="1.8"/><path d="M2 11 H14"/><circle cx="6" cy="11" r="1.8"/>' />
  );
}

export function PlayIcon() {
  return <Icon path='<path d="M5 3.5 L12 8 L5 12.5 Z"/>' size={13} />;
}

export function ChevronRightIcon() {
  return <Icon path='<path d="M6 3.5 L10.5 8 L6 12.5"/>' size={14} />;
}

export function ChevronLeftIcon() {
  return <Icon path='<path d="M10 3.5 L5.5 8 L10 12.5"/>' size={14} />;
}

export function StopIcon() {
  return <Icon path='<rect x="4" y="4" width="8" height="8" rx="1.5"/>' size={13} />;
}
