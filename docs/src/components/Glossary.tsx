import type { ReactNode } from "react";

export function GlossaryEntry({
  term,
  dev,
  fin,
}: {
  term: string;
  dev: ReactNode;
  fin: ReactNode;
}) {
  return (
    <>
      <dt>{term}</dt>
      <div className="g-def dev">
        <span className="g-tag">DEV</span>
        <p>{dev}</p>
      </div>
      <div className="g-def fin">
        <span className="g-tag">ACCOUNTING</span>
        <p>{fin}</p>
      </div>
    </>
  );
}
