import { Moon, Sun } from "lucide-react";
import { Button } from "#/components/ui/button";
import { useEffect, useRef, useState } from "react";

const THEME_COLOR_LIGHT = "#1E6B5A";
const THEME_COLOR_DARK = "#0C1222";

function updateThemeColor(isDark: boolean) {
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", isDark ? THEME_COLOR_DARK : THEME_COLOR_LIGHT);
}

/** Apply a theme end-to-end: <html> class, persisted preference, meta color. */
function applyTheme(next: "light" | "dark") {
  document.documentElement.classList.toggle("dark", next === "dark");
  localStorage.setItem("theme", next);
  updateThemeColor(next === "dark");
}

export function ThemeToggle() {
  // SSR-safe: the server and the first client render both use "light", so
  // hydration matches (avoids React #418). The real theme — which the no-flash
  // <script> in BaseLayout already applied to <html> before paint — is read
  // from the DOM on mount, so the icon catches up without a flash and without
  // clobbering the user's stored preference.
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [mounted, setMounted] = useState(false);
  const iconRef = useRef<HTMLSpanElement>(null);

  // Catch the icon up to whatever the no-flash script applied.
  useEffect(() => {
    setTheme(document.documentElement.classList.contains("dark") ? "dark" : "light");
    setMounted(true);
  }, []);

  // Listen for system theme changes, but only when the user hasn't stored a
  // preference of their own.
  useEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem("theme")) {
        const next = e.matches ? "dark" : "light";
        applyTheme(next);
        setTheme(next);
      }
    };
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    if (iconRef.current) {
      iconRef.current.classList.add("theme-icon-exit");
      iconRef.current.addEventListener(
        "animationend",
        () => {
          applyTheme(next);
          setTheme(next);
          if (iconRef.current) {
            iconRef.current.classList.remove("theme-icon-exit");
            iconRef.current.classList.add("theme-icon-enter");
            iconRef.current.addEventListener(
              "animationend",
              () => {
                iconRef.current?.classList.remove("theme-icon-enter");
              },
              { once: true },
            );
          }
        },
        { once: true },
      );
    } else {
      applyTheme(next);
      setTheme(next);
    }
  };

  return (
    <>
      <style>{`
        @keyframes theme-icon-exit {
          0%   { transform: rotate(0deg) scale(1); opacity: 1; }
          100% { transform: rotate(90deg) scale(0.5); opacity: 0; }
        }
        @keyframes theme-icon-enter {
          0%   { transform: rotate(-90deg) scale(0.5); opacity: 0; }
          100% { transform: rotate(0deg) scale(1); opacity: 1; }
        }
        .theme-icon-exit { animation: theme-icon-exit 0.2s ease-in forwards; }
        .theme-icon-enter { animation: theme-icon-enter 0.2s ease-out forwards; }
      `}</style>
      <Button
        variant="ghost"
        size="icon"
        onClick={toggleTheme}
        aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
      >
        <span ref={iconRef} className="inline-flex" style={{ opacity: mounted ? 1 : 0 }}>
          {theme === "dark" ? <Sun className="h-5 w-5" /> : <Moon className="h-5 w-5" />}
        </span>
      </Button>
    </>
  );
}
