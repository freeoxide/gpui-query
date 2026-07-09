import { initializeApp } from "firebase/app";
import {
  initializeAnalytics,
  isSupported,
  logEvent,
  type Analytics,
} from "firebase/analytics";

const firebaseConfig = {
  apiKey: "AIzaSyCux3FsCYpaoEyhKoLQQ-9X3NUxao6Uj1U",
  authDomain: "gpui-query.firebaseapp.com",
  projectId: "gpui-query",
  storageBucket: "gpui-query.firebasestorage.app",
  messagingSenderId: "260261054618",
  appId: "1:260261054618:web:8bbc954396f4cbac523723",
  measurementId: "G-V2TY39PKSH",
};

const app = initializeApp(firebaseConfig);
let analyticsPromise: Promise<Analytics | null> | undefined;
let lastPageLocation: string | undefined;

async function getFirebaseAnalytics() {
  if (typeof window === "undefined") {
    return null;
  }

  analyticsPromise ??= isSupported()
    .then((supported) =>
      supported
        ? initializeAnalytics(app, {
            config: {
              send_page_view: false,
            },
          })
        : null,
    )
    .catch(() => null);

  return analyticsPromise;
}

export async function logFirebasePageView() {
  const analytics = await getFirebaseAnalytics();
  if (!analytics || typeof window === "undefined") {
    return;
  }

  const pageLocation = window.location.href;
  if (pageLocation === lastPageLocation) {
    return;
  }

  lastPageLocation = pageLocation;
  logEvent(analytics, "page_view", {
    page_title: document.title,
    page_location: pageLocation,
    page_path: `${window.location.pathname}${window.location.search}`,
  });
}
