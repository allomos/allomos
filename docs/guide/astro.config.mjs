// The Allomos user guide, published on GitHub Pages (Task-A07-10).
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://allomos.github.io',
  base: '/allomos',
  integrations: [
    starlight({
      title: {
        en: 'Allomos Guide',
        'zh-CN': '换肩 Allomos 使用指南',
        ja: '肩代 Allomos ガイド',
      },
      logo: { src: './src/assets/logo.svg', alt: '' },
      favicon: '/favicon.svg',
      defaultLocale: 'root',
      locales: {
        root: { label: 'English', lang: 'en' },
        'zh-cn': { label: '简体中文', lang: 'zh-CN' },
        ja: { label: '日本語', lang: 'ja' },
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/allomos/allomos' },
      ],
      sidebar: [
        {
          label: 'Start here',
          translations: { 'zh-CN': '从这里开始', ja: 'はじめに' },
          items: [{ autogenerate: { directory: 'start' } }],
        },
      ],
    }),
  ],
});
