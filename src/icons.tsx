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

export function CloseIcon() {
  return <Icon path='<path d="M4 4 L12 12"/><path d="M12 4 L4 12"/>' size={14} />;
}

export function CheckIcon() {
  return <Icon path='<path d="M3 8.5 L6.5 12 L13 4.5"/>' size={14} />;
}

export function CopyIcon() {
  return (
    <Icon
      path='<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 5.5 V4 A1.5 1.5 0 0 0 9 2.5 H4 A1.5 1.5 0 0 0 2.5 4 V9 A1.5 1.5 0 0 0 4 10.5 H5.5"/>'
      size={14}
    />
  );
}

export function StopIcon() {
  return <Icon path='<rect x="4" y="4" width="8" height="8" rx="1.5"/>' size={13} />;
}

export function CubeIcon() {
  return (
    <Icon
      path='<path d="M8 1.5 L14 4.5 V11.5 L8 14.5 L2 11.5 V4.5 Z"/><path d="M2 4.5 L8 7.5 L14 4.5"/><path d="M8 7.5 V14.5"/>'
      size={44}
    />
  );
}

export function SearchIcon() {
  return (
    <Icon path='<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 L14 14"/>' size={14} />
  );
}

/// pi's own mark, from the logo the project publishes (`https://pi.dev/logo-auto.svg`,
/// linked by its README). Filled rather than stroked, and cropped to the artwork's own
/// bounds so it sits at the weight of the stroked icons beside it.
/// Stands in for an owner with no picture. Deliberately one generic mark rather than a
/// coloured initial: a letter invents a distinction between owners that the app has no
/// basis for, and every row without an avatar should look the same kind of unknown.
export function OwnerIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3 20 7.5v9L12 21 4 16.5v-9z" />
      <path d="M12 12 20 7.5M12 12v9M12 12 4 7.5" />
    </svg>
  );
}

export function PiIcon() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="165.29 165.29 469.43 469.43"
      fill="currentColor"
      aria-hidden="true"
    >
      <path
        fillRule="evenodd"
        d="M165.29 165.29 H517.36 V400 H400 V517.36 H282.65 V634.72 H165.29 Z M282.65 282.65 V400 H400 V282.65 Z"
      />
      <path d="M517.36 400 H634.72 V634.72 H517.36 Z" />
    </svg>
  );
}
