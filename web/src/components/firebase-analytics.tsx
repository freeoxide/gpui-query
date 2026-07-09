import { useEffect } from "react";
import { logFirebasePageView } from "#/lib/firebase";

export function FirebaseAnalytics() {
  useEffect(() => {
    const logAfterNavigation = () => {
      window.setTimeout(() => {
        void logFirebasePageView();
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

  return null;
}
