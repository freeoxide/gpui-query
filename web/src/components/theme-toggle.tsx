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

export function ThemeToggle() {
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    if (typeof window === "undefined") return "light";
    const stored = localStorage.getItem("theme");
    if (stored === "dark" || stored === "light") return stored;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });

  const [mounted, setMounted] = useState(false);
  const iconRef = useRef<HTMLSpanElement>(null);

  // Apply theme class, persist, and update meta
  useEffect(() => {
    const root = document.documentElement;
    const isDark = theme === "dark";

    if (isDark) {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
    localStorage.setItem("theme", theme);
    updateThemeColor(isDark);
    setMounted(true);
  }, [theme]);

  // Listen for system theme changes
  useEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      // Only follow system if user hasn't explicitly stored a preference
      if (!localStorage.getItem("theme")) {
        setTheme(e.matches ? "dark" : "light");
      }
    };
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);

  const toggleTheme = () => {
    if (iconRef.current) {
      iconRef.current.classList.add("theme-icon-exit");
      iconRef.current.addEventListener(
        "animationend",
        () => {
          setTheme((prev) => (prev === "dark" ? "light" : "dark"));
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
      setTheme((prev) => (prev === "dark" ? "light" : "dark"));
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
