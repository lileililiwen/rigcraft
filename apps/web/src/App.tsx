import { useEffect } from "react";
import { CatalogPage } from "./components/CatalogPage";
import { CheckPage } from "./components/CheckPage";
import { DiagnosePage } from "./components/DiagnosePage";
import { InventoryPage } from "./components/InventoryPage";
import { RecommendPage } from "./components/RecommendPage";

/**
 * The Rust process serves the shell only for `/`, `/inventory`, `/catalog`,
 * `/builds/check`, `/recommend` and `/diagnose`; any other path is a server 404
 * document. The route is therefore resolved from the location before the first
 * render and never guessed from an unknown path.
 */
export default function App() {
  const path = typeof window === "undefined" ? "/" : window.location.pathname;
  const catalog = path.startsWith("/catalog");
  const compatibility = path.startsWith("/builds/check");
  const recommendation = path.startsWith("/recommend");
  const diagnosis = path.startsWith("/diagnose");
  useEffect(() => {
    document.title = catalog
      ? "RigCraft catalog"
      : compatibility
        ? "RigCraft compatibility check"
        : recommendation
          ? "RigCraft build recommendation"
          : diagnosis
            ? "RigCraft build diagnosis"
            : "RigCraft inventory";
  }, [catalog, compatibility, recommendation, diagnosis]);
  if (catalog) {
    return <CatalogPage />;
  }
  if (compatibility) {
    return <CheckPage />;
  }
  if (recommendation) {
    return <RecommendPage />;
  }
  if (diagnosis) {
    return <DiagnosePage />;
  }
  return <InventoryPage />;
}
