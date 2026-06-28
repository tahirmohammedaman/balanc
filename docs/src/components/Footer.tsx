import { Link } from "react-router-dom";

export function Footer() {
  return (
    <footer className="site-footer">
      <div className="wrap">
        <p>balanc · MIT licensed · a from-scratch Rust interpreter and JVM bytecode backend for double-entry bookkeeping</p>
        <div className="foot-links">
          <Link to="/docs/introduction">Documentation</Link>
          <Link to="/docs/cli-reference">CLI reference</Link>
          <Link to="/docs/glossary">Glossary</Link>
        </div>
      </div>
    </footer>
  );
}
