import { NavLink } from "react-router-dom";
import { BrandMark } from "./BrandMark";
import { ThemeToggle } from "./ThemeToggle";

interface NavBarProps {
  /** Rendered before the brand, e.g. the mobile sidebar toggle on docs pages. */
  leading?: React.ReactNode;
  variant?: "landing" | "docs";
}

export function NavBar({ leading, variant = "landing" }: NavBarProps) {
  return (
    <nav className={`site-nav ${variant}`} aria-label="Site">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <div className="wrap">
        {leading}
        <NavLink className="brand" to="/" end>
          <BrandMark />
          balanc
        </NavLink>
        <div className="nav-links">
          <NavLink to="/docs/tour">Tour</NavLink>
          <NavLink to="/docs/introduction" className={({ isActive }) => (isActive ? "active" : "")}>
            Docs
          </NavLink>
          <NavLink to="/docs/errors">Errors</NavLink>
        </div>
        <div className="nav-spacer" />
        <a className="nav-gh" href="https://github.com/tahirmohammedaman/balanc">
          GitHub
        </a>
        <ThemeToggle />
      </div>
    </nav>
  );
}
