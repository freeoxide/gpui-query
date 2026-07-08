/**
 * Swizzled from @docusaurus/theme-classic's <Logo>.
 *
 * Why this override exists
 * ------------------------
 * The docs site is served under baseUrl `/docs/` (see docusaurus.config.ts),
 * but the navbar logo must take the user back to the marketing homepage at the
 * domain root `/`. That homepage is a *separate* app (the TanStack `web/` site)
 * mounted outside Docusaurus; the docs SPA can't route to it.
 *
 * The stock <Logo> renders its link with Docusaurus's <Link>, which always
 * re-appends baseUrl to any root-absolute href. Even the `pathname://` escape
 * hatch does this — in core/.../Link.js it strips the protocol but still calls
 * maybeAddBaseUrl(), with the comment "we want baseUrl to be appended". So a
 * configured `href: '/'` resolves to `/docs/` and the logo just points back at
 * the docs root.
 *
 * Rendering a plain <a> (instead of <Link>) bypasses baseUrl entirely, giving a
 * correct root-relative link that works in both local preview and production.
 * Everything else mirrors the upstream component.
 */
import React, {type ReactNode} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {useThemeConfig} from '@docusaurus/theme-common';
import ThemedImage from '@theme/ThemedImage';
import type {Props} from '@theme/Logo';

type NavbarLogo = NonNullable<
  ReturnType<typeof useThemeConfig>['navbar']['logo']
>;

function LogoThemedImage({
  logo,
  alt,
  imageClassName,
}: {
  logo: NavbarLogo;
  alt: string;
  imageClassName?: string;
}) {
  const sources = {
    light: useBaseUrl(logo.src),
    dark: useBaseUrl(logo.srcDark || logo.src),
  };
  const themedImage = (
    <ThemedImage
      className={logo.className}
      sources={sources}
      height={logo.height}
      width={logo.width}
      alt={alt}
      style={logo.style}
    />
  );
  return imageClassName ? (
    <div className={imageClassName}>{themedImage}</div>
  ) : (
    themedImage
  );
}

export default function Logo(props: Props): ReactNode {
  const {
    siteConfig: {title},
  } = useDocusaurusContext();
  const {
    navbar: {title: navbarTitle, logo},
  } = useThemeConfig();
  const {imageClassName, titleClassName, ...propsRest} = props;

  // If a visible title is shown, fallback alt text should be an empty string to
  // mark the logo as decorative.
  const fallbackAlt = navbarTitle ? '' : title;
  const alt = logo?.alt ?? fallbackAlt;

  // Use the configured href verbatim (defaulting to the site root) on a plain
  // <a>, so it escapes the docs `/docs/` baseUrl. See the file header.
  const href = logo?.href ?? '/';

  return (
    <a
      href={href}
      {...propsRest}
      {...(logo?.target && {target: logo.target})}>
      {logo && (
        <LogoThemedImage logo={logo} alt={alt} imageClassName={imageClassName} />
      )}
      {navbarTitle != null && <b className={titleClassName}>{navbarTitle}</b>}
    </a>
  );
}
