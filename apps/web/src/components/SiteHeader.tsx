export type CurrentPage = "inventory" | "catalog" | "compatibility" | "recommend" | "diagnose";

export function SiteHeader({ current, wide = false }: { current: CurrentPage; wide?: boolean }) {
  return (
    <>
      <a className="skip-link" href="#main">
        {current === "catalog"
          ? "Skip to catalog"
          : current === "compatibility"
            ? "Skip to compatibility check"
            : current === "recommend"
              ? "Skip to recommendation"
              : current === "diagnose"
                ? "Skip to build diagnosis"
                : "Skip to inventory"}
      </a>
      <header className="site-header">
        <div className={`site-header__inner${wide ? " site-header__inner--wide" : ""}`}>
          <a className="brand" href="/inventory">
            RigCraft
          </a>
          <nav className="primary-nav" aria-label="Primary">
            <a href="/inventory" aria-current={current === "inventory" ? "page" : undefined}>
              Inventory
            </a>
            <a href="/catalog" aria-current={current === "catalog" ? "page" : undefined}>
              Catalog
            </a>
            <a
              href="/builds/check"
              aria-current={current === "compatibility" ? "page" : undefined}
            >
              Compatibility
            </a>
            <a
              href="/recommend"
              aria-current={current === "recommend" ? "page" : undefined}
            >
              Recommend
            </a>
            <a
              href="/diagnose"
              aria-current={current === "diagnose" ? "page" : undefined}
            >
              Diagnose
            </a>
          </nav>
        </div>
      </header>
    </>
  );
}
