export function BrandMark() {
  return (
    <svg className="mark" viewBox="0 0 32 32" aria-hidden="true">
      <rect width="32" height="32" rx="7" fill="#3457d5" />
      <g fill="none" stroke="#ffffff" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <line x1="16" y1="7" x2="16" y2="23" />
        <line x1="7" y1="11" x2="25" y2="11" />
        <path d="M7 11 L4.5 17 a4 4 0 0 0 5 0 Z" />
        <path d="M25 11 L22.5 17 a4 4 0 0 0 5 0 Z" />
        <line x1="11" y1="23" x2="21" y2="23" />
      </g>
    </svg>
  );
}
