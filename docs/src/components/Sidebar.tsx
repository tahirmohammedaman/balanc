import { useMemo, useState } from "react";
import { NavLink } from "react-router-dom";
import { NAV } from "../data/nav";

interface SidebarProps {
  open: boolean;
  onNavigate: () => void;
}

export function Sidebar({ open, onNavigate }: SidebarProps) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return NAV;
    return NAV.map((g) => ({
      ...g,
      items: g.items.filter((i) => i.title.toLowerCase().includes(q)),
    })).filter((g) => g.items.length > 0);
  }, [query]);

  return (
    <aside className={`docs-sidebar${open ? " open" : ""}`}>
      <div className="sidebar-search">
        <input
          type="search"
          placeholder="Filter pages…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Filter documentation pages"
        />
      </div>
      {filtered.length === 0 && <p className="side-empty">No pages match "{query}".</p>}
      {filtered.map((group) => (
        <div className="side-group" key={group.title}>
          <div className="side-title">{group.title}</div>
          {group.items.map((item) => (
            <NavLink key={item.path} to={item.path} onClick={onNavigate}>
              {item.title}
            </NavLink>
          ))}
        </div>
      ))}
    </aside>
  );
}
