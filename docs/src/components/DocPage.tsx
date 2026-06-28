import type { ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { groupOf } from "../data/nav";
import { PrevNext } from "./PrevNext";

export function DocPage({
  title,
  lede,
  children,
}: {
  title: string;
  lede?: ReactNode;
  children: ReactNode;
}) {
  const { pathname } = useLocation();
  const group = groupOf(pathname);
  return (
    <article className="doc-section">
      <nav className="breadcrumb" aria-label="Breadcrumb">
        <Link to="/docs/introduction">Docs</Link>
        {group && (
          <>
            <span className="sep">/</span>
            <span>{group}</span>
          </>
        )}
        <span className="sep">/</span>
        <span className="current">{title}</span>
      </nav>
      <h1>{title}</h1>
      {lede && <p className="doc-lede">{lede}</p>}
      {children}
      <PrevNext />
    </article>
  );
}
