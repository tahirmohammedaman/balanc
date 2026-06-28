import { useRef, useState } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { NavBar } from "./NavBar";
import { Footer } from "./Footer";
import { Sidebar } from "./Sidebar";
import { ScrollToTop } from "./ScrollToTop";
import { TableOfContents } from "./TableOfContents";
import { useHeadings } from "../hooks/useHeadings";

export function DocsLayout() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const mainRef = useRef<HTMLElement>(null);
  const { pathname } = useLocation();
  const headings = useHeadings(mainRef, [pathname]);

  return (
    <>
      <ScrollToTop />
      <NavBar
        variant="docs"
        leading={
          <button
            className="mobile-nav-toggle"
            type="button"
            aria-label="Toggle navigation"
            onClick={() => setSidebarOpen((v) => !v)}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
              <path d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
        }
      />
      <div className={`docs-sidebar-scrim${sidebarOpen ? " open" : ""}`} onClick={() => setSidebarOpen(false)} />
      <div className="wrap">
        <div className="docs-shell">
          <Sidebar open={sidebarOpen} onNavigate={() => setSidebarOpen(false)} />
          <main className="docs-main" ref={mainRef}>
            <Outlet />
          </main>
          <TableOfContents headings={headings} />
        </div>
      </div>
      <Footer />
    </>
  );
}
