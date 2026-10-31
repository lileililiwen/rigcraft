import { useEffect } from "react";
import { CheckPage } from "./components/CheckPage";
import { InventoryPage } from "./components/InventoryPage";
import { RecommendPage } from "./components/RecommendPage";

/**
 * The Rust process serves one SPA shell for every non-API path, so the route
 * is resolved from the location before the first render.
 */
export default function App() {
  const path = typeof window === "undefined" ? "/" : window.location.pathname;
  const compatibility = path.startsWith("/builds/check");
  const recommendation = path.startsWith("/recommend");
  useEffect(() => {
    document.title = compatibility
      ? "RigCraft compatibility check"
      : recommendation
        ? "RigCraft build recommendation"
        : "RigCraft inventory";
  }, [compatibility, recommendation]);
  if (compatibility) {
    return <CheckPage />;
  }
  if (recommendation) {
    return <RecommendPage />;
  }
  return <InventoryPage />;
}
