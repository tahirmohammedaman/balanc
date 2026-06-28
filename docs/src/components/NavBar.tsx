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
    <nav className="site-nav">
      <div className="wrap">
        {leading}
        <NavLink className="brand" to="/" end>
          <BrandMark />
          balanc
          <span className="tag">{variant === "docs" ? "docs" : "linear-money DSL"}</span>
        </NavLink>
        <div className="nav-links">
          {variant === "landing" ? (
            <>
              <NavLink to="/docs/getting-started">Get Started</NavLink>
              <NavLink to="/docs/language/core-concepts">Language</NavLink>
              <NavLink to="/docs/architecture/jvm-backend">JVM Backend</NavLink>
              <NavLink to="/docs/introduction">Docs</NavLink>
            </>
          ) : (
            <NavLink to="/" end>
              Home
            </NavLink>
          )}
        </div>
        <div className="nav-spacer" />
        <span className="nav-cli">cargo run -- examples/coffee.bal</span>
        <ThemeToggle />
      </div>
    </nav>
  );
}
