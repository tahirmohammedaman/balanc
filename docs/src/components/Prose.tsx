import type { ReactNode } from "react";

/** Inline `code`-styled span, for referring to identifiers/keywords in prose. */
export function C({ children }: { children: ReactNode }) {
  return <code className="inline">{children}</code>;
}

export function Badge({
  tone = "blue",
  children,
}: {
  tone?: "blue" | "green" | "amber" | "red";
  children: ReactNode;
}) {
  return <span className={`badge badge-${tone}`}>{children}</span>;
}

export function Callout({
  title,
  tone = "info",
  children,
}: {
  title: string;
  tone?: "info" | "warn";
  children: ReactNode;
}) {
  return (
    <div className={`callout${tone === "warn" ? " warn" : ""}`}>
      <span className="callout-title">{title}</span>
      {children}
    </div>
  );
}

export interface Column {
  header: string;
  /** true renders the cell in the monospace "code" column style. */
  code?: boolean;
}

export function DocTable({
  columns,
  rows,
}: {
  columns: Column[];
  rows: ReactNode[][];
}) {
  return (
    <div className="doc-table">
      <table>
        <thead>
          <tr>
            {columns.map((c) => (
              <th key={c.header}>{c.header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i}>
              {row.map((cell, j) => (
                <td key={j} className={columns[j]?.code ? "code-col" : undefined}>
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** A small proof-style inference rule: premises over a line over the
 * conclusion, with the rule's name labeling the line and an optional prose
 * note below. */
export function Rule({
  premises,
  name,
  conclusion,
  side,
}: {
  premises: string;
  name: string;
  conclusion: string;
  side?: ReactNode;
}) {
  return (
    <div className="rule">
      <div className="premises">{premises}</div>
      <div className="bar">
        <span className="rule-name">{name}</span>
      </div>
      <div className="conclusion">{conclusion}</div>
      {side && <div className="side">{side}</div>}
    </div>
  );
}

export function Steps({ children }: { children: ReactNode }) {
  return <ol className="steps">{children}</ol>;
}

export function Step({ title, children }: { title: string; children: ReactNode }) {
  return (
    <li>
      <h3>{title}</h3>
      {children}
    </li>
  );
}
