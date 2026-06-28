import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'gpui-query Docs',
  tagline: 'Zero-boilerplate async state management for GPUI',
  url: 'https://gpui-query.hmziq.xyz',
  baseUrl: '/docs/',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: '/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
      defaultMode: 'light',
    },
    navbar: {
      title: 'gpui-query',
      logo: {
        alt: 'gpui-query',
        src: 'img/logo.svg',
        // The docs site is served under baseUrl `/docs/`, but the homepage is
        // the separate marketing app at the domain root `/`. A plain `href: '/'`
        // gets run through useBaseUrl() and becomes `/docs/`, so the logo just
        // points back at the docs root. The `pathname://` escape hatch renders a
        // regular anchor with no baseUrl prepending and no SPA interception,
        // sending the user to the real homepage at `/`.
        href: 'pathname:///',
        target: '_self',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/hmziqrs/gpui-query',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Getting Started',
              to: '/getting-started/installation',
            },
            {
              label: 'API Reference',
              to: '/api/queries',
            },
            {
              label: 'Guides',
              to: '/guides/caching',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/hmziqrs/gpui-query',
            },
          ],
        },
      ],
      copyright: 'Copyright &copy; 2024-2026 hmziqrs.',
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
