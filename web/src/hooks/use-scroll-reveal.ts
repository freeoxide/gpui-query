import { useEffect, useRef } from "react";

/**
 * Lightweight scroll-reveal hook. Place the returned ref on a wrapper element.
 * All descendant elements with `[data-reveal]` will animate in when they enter
 * the viewport. Uses IntersectionObserver — no animation library required.
 *
 * Progressive enhancement: content is visible by default (SEO/no-JS safe).
 * The hook adds `js-reveal` to enable animations only when JS runs.
 * Respects `prefers-reduced-motion` by immediately showing all elements.
 */
export function useScrollReveal() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = ref.current;
    if (!container) return;

    // Enable reveal animations (content is visible without this class)
    container.classList.add("js-reveal");

    const elements = container.querySelectorAll("[data-reveal]");
    if (!elements.length) return;

    // Respect reduced motion — show everything immediately
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced) {
      elements.forEach((el) => el.classList.add("is-visible"));
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.15, rootMargin: "0px 0px -60px 0px" },
    );

    elements.forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return ref;
}
