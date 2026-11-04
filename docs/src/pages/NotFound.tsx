import { Link } from "react-router-dom";
import { NavBar } from "../components/NavBar";
import { Footer } from "../components/Footer";

export function NotFound() {
  return (
    <>
      <NavBar variant="docs" />
      <main className="wrap" id="main">
        <div className="not-found">
          <h1>Nothing posted at this address</h1>
          <p>This page doesn't exist. If a link brought you here, the docs were probably reorganised; the sidebar has everything.</p>
          <Link className="btn btn-primary" to="/docs/introduction">
            Go to the docs →
          </Link>
        </div>
      </main>
      <Footer />
    </>
  );
}
