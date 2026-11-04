/** The balanc mark: two ledger entries over a double rule — a total that balances. */
export function BrandMark() {
  return (
    <svg className="mark" viewBox="0 0 32 32" aria-hidden="true">
      <rect width="32" height="32" rx="6" fill="var(--brand)" />
      <g stroke="var(--brand-ink)" strokeWidth={2.2} strokeLinecap="round">
        <line x1="8" y1="9.5" x2="24" y2="9.5" />
        <line x1="14" y1="15" x2="24" y2="15" />
        <line x1="8" y1="21" x2="24" y2="21" />
        <line x1="8" y1="24.6" x2="24" y2="24.6" />
      </g>
    </svg>
  );
}
