import { Link, useLocation } from "react-router-dom";
import { FLAT_NAV } from "../data/nav";

export function PrevNext() {
  const { pathname } = useLocation();
  const index = FLAT_NAV.findIndex((i) => i.path === pathname);
  if (index === -1) return null;

  const prev = index > 0 ? FLAT_NAV[index - 1] : null;
  const next = index < FLAT_NAV.length - 1 ? FLAT_NAV[index + 1] : null;

  return (
    <nav className="prev-next" aria-label="Page navigation">
      {prev ? (
        <Link className="pn-link pn-prev" to={prev.path}>
          <span className="pn-label">← Previous</span>
          <span className="pn-title">{prev.title}</span>
        </Link>
      ) : (
        <span />
      )}
      {next ? (
        <Link className="pn-link pn-next" to={next.path}>
          <span className="pn-label">Next →</span>
          <span className="pn-title">{next.title}</span>
        </Link>
      ) : (
        <span />
      )}
    </nav>
  );
}
