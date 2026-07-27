import { useEffect } from "react";

export function FirebaseAnalytics() {
  useEffect(() => {
    const logAfterNavigation = () => {
      window.setTimeout(() => {
        // Import lazily: a static import would pull every @firebase/*
        // package (and its module-scope initializeApp) into the entry
        // chunk, ahead of hydration.
        void import("#/lib/firebase").then(({ logFirebasePageView }) => logFirebasePageView());
      }, 0);
    };

    const originalPushState = window.history.pushState;
    const originalReplaceState = window.history.replaceState;

    window.history.pushState = function pushState(...args) {
      const result = originalPushState.apply(this, args);
      logAfterNavigation();
      return result;
    };

    window.history.replaceState = function replaceState(...args) {
      const result = originalReplaceState.apply(this, args);
      logAfterNavigation();
      return result;
    };

    window.addEventListener("popstate", logAfterNavigation);
    logAfterNavigation();

    return () => {
      window.history.pushState = originalPushState;
      window.history.replaceState = originalReplaceState;
      window.removeEventListener("popstate", logAfterNavigation);
    };
  }, []);

  // Return an empty Fragment instead of `null`. Under Astro 7 dev, react-refresh
  // (@vitejs/plugin-react) only treats a hooks-using module as a refresh
  // boundary when it contains JSX; a component that calls hooks but returns
  // `null` is skipped, so its hooks run without React's dispatcher and throw
  // "Invalid hook call" during dev SSR. An empty Fragment renders nothing yet
  // gives react-refresh the JSX it needs. Build/prod SSR is unaffected.
  return <></>;
}
