import { useEffect, useState } from "react";
import type { Heading } from "../hooks/useHeadings";

/** Right-rail "on this page" navigation. Highlights whichever heading is
 * currently nearest the top of the viewport via IntersectionObserver, rather
 * than a scroll-position calculation. */
export function TableOfContents({ headings }: { headings: Heading[] }) {
  const [activeId, setActiveId] = useState<string | null>(null);

  useEffect(() => {
    if (headings.length === 0) {
      setActiveId(null);
      return;
    }
    const elements = headings
      .map((h) => document.getElementById(h.id))
      .filter((el): el is HTMLElement => el !== null);

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries.filter((e) => e.isIntersecting);
        if (visible.length > 0) {
          setActiveId(visible[0].target.id);
        }
      },
      { rootMargin: "-80px 0px -70% 0px", threshold: 0 },
    );
    elements.forEach((el) => observer.observe(el));
    setActiveId(headings[0].id);
    return () => observer.disconnect();
  }, [headings]);

  if (headings.length === 0) return null;

  return (
    <aside className="docs-toc" aria-label="Table of contents">
      <div className="toc-title">On this page</div>
      <ul>
        {headings.map((h) => (
          <li key={h.id}>
            <a href={`#${h.id}`} className={h.id === activeId ? "active" : undefined}>
              {h.text}
            </a>
          </li>
        ))}
      </ul>
    </aside>
  );
}
