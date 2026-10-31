/**
 * Master switch for the /v1–/v4 landing design previews. While false, the
 * preview routes throw notFound() and the floating VersionSwitcher renders
 * nothing. Flip to true to bring the previews back while iterating on landing
 * designs.
 */
export const LANDING_PREVIEWS_ENABLED = false;
