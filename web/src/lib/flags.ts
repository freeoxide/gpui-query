/**
 * Master switch for the /v1–/v4 landing design previews. While false, the
 * preview routes throw notFound(), they are skipped at prerender time (see
 * vite.config.ts), and the floating VersionSwitcher renders nothing. Flip to
 * true to bring the previews back while iterating on landing designs.
 *
 * V4 won and is rendered at `/` — see src/routes/index.tsx.
 */
export const LANDING_PREVIEWS_ENABLED = false;
