import { useEffect, useState, type RefObject } from "react";

export interface Heading {
  id: string;
  text: string;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-");
}

/** Scans `<h2>` elements under `ref` after each render of `deps`, assigns a
 * stable slug `id` to any that lack one, and returns them in document order —
 * the data source for the right-rail table of contents and its scroll-spy. */
export function useHeadings(ref: RefObject<HTMLElement | null>, deps: unknown[]): Heading[] {
  const [headings, setHeadings] = useState<Heading[]>([]);

  useEffect(() => {
    const container = ref.current;
    if (!container) {
      setHeadings([]);
      return;
    }
    const seen = new Map<string, number>();
    const nodes = Array.from(container.querySelectorAll<HTMLHeadingElement>("h2"));
    const next = nodes.map((node) => {
      if (!node.id) {
        const base = slugify(node.textContent ?? "");
        const count = seen.get(base) ?? 0;
        seen.set(base, count + 1);
        node.id = count === 0 ? base : `${base}-${count}`;
      }
      return { id: node.id, text: node.textContent ?? "" };
    });
    setHeadings(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return headings;
}
