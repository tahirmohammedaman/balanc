import { lazy, Suspense } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ThemeProvider } from "./context/ThemeContext";
import { Landing } from "./pages/Landing";
import { DocsLayout } from "./components/DocsLayout";
import { Introduction } from "./pages/docs/Introduction";
import { Installation } from "./pages/docs/Installation";
import { GettingStarted } from "./pages/docs/GettingStarted";
import { CoreConcepts } from "./pages/docs/CoreConcepts";
import { SyntaxReference } from "./pages/docs/SyntaxReference";
import { TypeSystem } from "./pages/docs/TypeSystem";
import { OperationsReference } from "./pages/docs/OperationsReference";
import { ErrorCatalogue } from "./pages/docs/ErrorCatalogue";
import { Tour } from "./pages/docs/Tour";
import { CompilerArchitecture } from "./pages/docs/CompilerArchitecture";
import { JvmBackend } from "./pages/docs/JvmBackend";
import { CliReference } from "./pages/docs/CliReference";
import { Examples } from "./pages/docs/Examples";
import { Glossary } from "./pages/docs/Glossary";
import { NotFound } from "./pages/NotFound";

// Lazy, so the playground and the WebAssembly compiler it loads stay out of the
// landing and docs bundles.
const Playground = lazy(() => import("./pages/Playground").then((m) => ({ default: m.Playground })));

export function App() {
  return (
    <ThemeProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Landing />} />
          <Route
            path="/playground"
            element={
              <Suspense fallback={null}>
                <Playground />
              </Suspense>
            }
          />

          <Route path="/docs" element={<DocsLayout />}>
            <Route index element={<Navigate to="/docs/introduction" replace />} />
            <Route path="introduction" element={<Introduction />} />
            <Route path="installation" element={<Installation />} />
            <Route path="getting-started" element={<GettingStarted />} />
            <Route path="language/core-concepts" element={<CoreConcepts />} />
            <Route path="language/syntax-reference" element={<SyntaxReference />} />
            <Route path="language/type-system" element={<TypeSystem />} />
            <Route path="language/operations-reference" element={<OperationsReference />} />
            <Route path="tour" element={<Tour />} />
            <Route path="errors" element={<ErrorCatalogue />} />
            <Route path="diagnostics" element={<Navigate to="/docs/errors" replace />} />
            <Route path="architecture/compiler" element={<CompilerArchitecture />} />
            <Route path="architecture/jvm-backend" element={<JvmBackend />} />
            <Route path="cli-reference" element={<CliReference />} />
            <Route path="examples" element={<Examples />} />
            <Route path="glossary" element={<Glossary />} />
          </Route>

          <Route path="*" element={<NotFound />} />
        </Routes>
      </BrowserRouter>
    </ThemeProvider>
  );
}
