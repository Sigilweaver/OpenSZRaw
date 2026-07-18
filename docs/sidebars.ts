import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    'install',
    'quickstart',
    {
      type: 'category',
      label: 'Guide',
      collapsed: false,
      items: [
        'guide/reader',
        'guide/scan-data',
        'guide/format-variants',
        'guide/mzml-export',
        'guide/python-api',
      ],
    },
    {
      type: 'category',
      label: 'Format Specification',
      link: { type: 'doc', id: 'format/overview' },
      items: [
        'format/overview',
        'format/ole2-container',
        'format/qgd-gcms',
        'format/lcd-ittof',
        'format/lcd-qtof',
        'format/known-limitations',
      ],
    },
    'changelog',
    'license',
  ],
};

export default sidebars;
