import { Link } from "react-router-dom";

export function Footer() {
  return (
    <footer className="site-footer">
      <div className="wrap">
        <p>
          balanc · MIT licensed · a Rust compiler for double-entry bookkeeping, with an interpreter and a hand-written
          JVM backend
        </p>
        <div className="foot-links">
          <Link to="/docs/tour">Tour</Link>
          <Link to="/docs/errors">Error catalogue</Link>
          <Link to="/docs/cli-reference">CLI reference</Link>
          <a href="https://github.com/tahirmohammedaman/balanc">GitHub</a>
        </div>
      </div>
    </footer>
  );
}
