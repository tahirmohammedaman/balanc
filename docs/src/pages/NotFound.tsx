import { Link } from "react-router-dom";
import { NavBar } from "../components/NavBar";
import { Footer } from "../components/Footer";

export function NotFound() {
  return (
    <>
      <NavBar variant="docs" />
      <div className="wrap">
        <div className="not-found">
          <h1>404</h1>
          <p>This page doesn't exist — or the ledger for it was never posted.</p>
          <Link className="btn btn-primary" to="/docs/introduction">
            Go to the docs →
          </Link>
        </div>
      </div>
      <Footer />
    </>
  );
}
