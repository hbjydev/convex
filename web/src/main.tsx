import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Link, Route, Routes } from "react-router";
import { SystemPage } from "./system-page";
import "./styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("Missing application root");

createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <div className="shell">
        <header>
          <Link className="brand" to="/">
            Convex
            <span className="brand-dot" />
          </Link>
          <span className="header-note">Application foundation</span>
        </header>
        <main>
          <Routes>
            <Route path="/" element={<SystemPage />} />
            <Route
              path="*"
              element={
                <>
                  <h1>Page not found</h1>
                  <p>This page does not exist.</p>
                  <Link to="/">Return to system status</Link>
                </>
              }
            />
          </Routes>
        </main>
        <footer>Self-hosted issue tracking · Development preview</footer>
      </div>
    </BrowserRouter>
  </StrictMode>,
);
