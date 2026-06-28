import { useEffect } from "react";
import { useLocation } from "react-router-dom";

/** Docs sites always reset scroll position on navigation — the browser's
 * default "keep scroll position" behavior reads as broken when the page
 * content underneath has completely changed. */
export function ScrollToTop() {
  const { pathname } = useLocation();
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [pathname]);
  return null;
}
