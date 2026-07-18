import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'OpenSZRaw',
  tagline: 'Rust reader for Shimadzu LabSolutions .lcd / .qgd mass spectrometry files',
  favicon: 'img/favicon.ico',

  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },
  plugins: ['docusaurus-plugin-llms-txt'],
  themes: ['@docusaurus/theme-mermaid'],

  url: 'https://sigilweaver.app',
  baseUrl: '/openszraw/docs/',

  organizationName: 'Sigilweaver',
  projectName: 'OpenSZRaw',

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          routeBasePath: '/',
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/Sigilweaver/OpenSZRaw/tree/main/docs/',
        },
        blog: false,
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          filename: 'sitemap.xml',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    metadata: [
      { name: 'keywords', content: 'OpenSZRaw, Shimadzu, LabSolutions, lcd, qgd, mass spectrometry, IT-TOF, QTOF, GC-MS, Rust' },
      { name: 'description', content: 'OpenSZRaw is a Rust reader for Shimadzu LabSolutions .lcd / .qgd mass spectrometry files.' },
    ],
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Sigilweaver',
      logo: {
        alt: 'Sigilweaver logo',
        src: 'img/logo.svg',
        href: 'https://sigilweaver.app',
        target: '_self',
      },
      items: [
        {
          type: 'dropdown',
          label: 'OpenSZRaw',
          position: 'left',
          items: [
            { label: 'OpenMassSpec', href: 'https://sigilweaver.app/openmassspec/docs/' },
            { label: 'OpenTFRaw (Thermo)', href: 'https://sigilweaver.app/opentfraw/docs/' },
            { label: 'OpenWRaw (Waters)', href: 'https://sigilweaver.app/openwraw/docs/' },
            { label: 'OpenTimsTDF (Bruker)', href: 'https://sigilweaver.app/opentimstdf/docs/' },
            { label: 'OpenARaw (Agilent)', href: 'https://sigilweaver.app/openaraw/docs/' },
            { label: 'OpenSXRaw (SCIEX)', href: 'https://sigilweaver.app/opensxraw/docs/' },
          ],
        },
        {
          href: 'https://github.com/Sigilweaver/OpenSZRaw',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Project',
          items: [
            { label: 'GitHub', href: 'https://github.com/Sigilweaver/OpenSZRaw' },
            { label: 'Issues', href: 'https://github.com/Sigilweaver/OpenSZRaw/issues' },
          ],
        },
        {
          title: 'Legal',
          items: [
            { label: 'Terms of Use', href: 'https://sigilweaver.app/terms' },
            { label: 'Privacy Policy', href: 'https://sigilweaver.app/privacy' },
          ],
        },
      ],
      copyright: `Copyright ${new Date().getFullYear()} Sigilweaver Holdings LLC. OpenSZRaw is Apache-2.0 licensed. Documentation licensed under <a href="https://creativecommons.org/licenses/by-sa/4.0/" target="_blank" rel="noopener noreferrer">CC-BY-SA 4.0</a>.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash', 'python'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
