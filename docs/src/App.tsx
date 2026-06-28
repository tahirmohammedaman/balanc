import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ThemeProvider } from "./context/ThemeContext";
import { Landing } from "./pages/Landing";
import { DocsLayout } from "./components/DocsLayout";
import { Introduction } from "./pages/docs/Introduction";
import { GettingStarted } from "./pages/docs/GettingStarted";
import { CoreConcepts } from "./pages/docs/CoreConcepts";
import { SyntaxReference } from "./pages/docs/SyntaxReference";
import { TypeSystem } from "./pages/docs/TypeSystem";
import { OperationsReference } from "./pages/docs/OperationsReference";
import { Diagnostics } from "./pages/docs/Diagnostics";
import { CompilerArchitecture } from "./pages/docs/CompilerArchitecture";
import { JvmBackend } from "./pages/docs/JvmBackend";
import { CliReference } from "./pages/docs/CliReference";
import { Examples } from "./pages/docs/Examples";
import { Glossary } from "./pages/docs/Glossary";
import { NotFound } from "./pages/NotFound";

export function App() {
  return (
    <ThemeProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Landing />} />

          <Route path="/docs" element={<DocsLayout />}>
            <Route index element={<Navigate to="/docs/introduction" replace />} />
            <Route path="introduction" element={<Introduction />} />
            <Route path="getting-started" element={<GettingStarted />} />
            <Route path="language/core-concepts" element={<CoreConcepts />} />
            <Route path="language/syntax-reference" element={<SyntaxReference />} />
            <Route path="language/type-system" element={<TypeSystem />} />
            <Route path="language/operations-reference" element={<OperationsReference />} />
            <Route path="diagnostics" element={<Diagnostics />} />
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
