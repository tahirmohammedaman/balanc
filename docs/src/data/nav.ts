export interface NavItem {
  path: string;
  title: string;
}

export interface NavGroup {
  title: string;
  items: NavItem[];
}

export const NAV: NavGroup[] = [
  {
    title: "Overview",
    items: [
      { path: "/docs/introduction", title: "Introduction" },
      { path: "/docs/installation", title: "Installation" },
      { path: "/docs/getting-started", title: "Getting Started" },
    ],
  },
  {
    title: "Language Guide",
    items: [
      { path: "/docs/language/core-concepts", title: "Core Concepts" },
      { path: "/docs/language/syntax-reference", title: "Syntax Reference" },
      { path: "/docs/language/type-system", title: "The Type System" },
      { path: "/docs/language/operations-reference", title: "Operations Reference" },
    ],
  },
  {
    title: "Reference",
    items: [
      { path: "/docs/diagnostics", title: "Diagnostics" },
      { path: "/docs/architecture/compiler", title: "Compiler Architecture" },
      { path: "/docs/architecture/jvm-backend", title: "JVM Backend" },
      { path: "/docs/cli-reference", title: "CLI Reference" },
    ],
  },
  {
    title: "More",
    items: [
      { path: "/docs/examples", title: "Examples Gallery" },
      { path: "/docs/glossary", title: "Glossary" },
    ],
  },
];

export const FLAT_NAV: NavItem[] = NAV.flatMap((g) => g.items);

export function groupOf(path: string): string {
  return NAV.find((g) => g.items.some((i) => i.path === path))?.title ?? "";
}
