// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 36 个色彩主题定义（id/名称/描述/分组/预览色板），与 codex2api useTheme.tsx 一一对应。
// 实际 CSS 变量映射在 themes.css（.theme-<id> / .dark.theme-<id>），此处仅供页面展示与 hook 校验。

export type ColorTheme =
  | 'default'
  | 'glass'
  | 'graphite'
  | 'tokyo-night'
  | 'mono'
  | 'github'
  | 'vercel'
  | 'google'
  | 'apple'
  | 'notion'
  | 'stripe'
  | 'spotify'
  | 'ocean'
  | 'forest'
  | 'lavender'
  | 'amber'
  | 'espresso'
  | 'sakura'
  | 'mint'
  | 'midnight'
  | 'claude'
  | 'chatgpt'
  | 'deepseek'
  | 'aurora'
  | 'rose'
  | 'solarized-light'
  | 'quiet-light'
  | 'ayu-light'
  | 'noctis-lux'
  | 'one-dark-pro'
  | 'github-dimmed'
  | 'dracula'
  | 'monokai-pro'
  | 'nord'
  | 'catppuccin'
  | 'gruvbox'

export type ThemeGroup = 'recommended' | 'website' | 'light' | 'dark' | 'editor'

export interface ThemePreviewSwatch {
  sidebar?: string
  primary: string
  bg: string
  surface: string
  muted: string
}

export interface ColorThemeDef {
  id: ColorTheme
  /** i18n key：common.theme.<nameKey> */
  nameKey: string
  /** i18n key：themeSettings.themeDesc.<nameKey> */
  descriptionKey: string
  group: ThemeGroup
  recommended?: boolean
  previewLight: ThemePreviewSwatch
  previewDark: ThemePreviewSwatch
}

export const COLOR_THEMES: ColorThemeDef[] = [
  {
    id: 'default',
    nameKey: 'default',
    descriptionKey: 'default',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(174 81% 26%)',
      bg: 'hsl(178 20% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(174 22% 91%)',
    },
    previewDark: {
      primary: 'hsl(172 62% 45%)',
      bg: 'hsl(180 14% 12%)',
      surface: 'hsl(180 12% 15%)',
      muted: 'hsl(180 9% 22%)',
    },
  },
  {
    id: 'glass',
    nameKey: 'glass',
    descriptionKey: 'glass',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(32 92% 48%)',
      bg: 'hsl(210 33% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(214 22% 90%)',
      sidebar: 'hsl(210 40% 98%)',
    },
    previewDark: {
      primary: 'hsl(43 96% 76%)',
      bg: 'hsl(222 47% 6%)',
      surface: 'hsl(217 33% 14%)',
      muted: 'hsl(215 19% 27%)',
      sidebar: 'hsl(222 41% 8%)',
    },
  },
  {
    id: 'graphite',
    nameKey: 'graphite',
    descriptionKey: 'graphite',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(194 72% 38%)',
      bg: 'hsl(216 18% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(216 12% 90%)',
    },
    previewDark: {
      primary: 'hsl(190 72% 48%)',
      bg: 'hsl(220 13% 10%)',
      surface: 'hsl(220 12% 14%)',
      muted: 'hsl(220 10% 21%)',
    },
  },
  {
    id: 'tokyo-night',
    nameKey: 'tokyoNight',
    descriptionKey: 'tokyoNight',
    group: 'dark',
    recommended: true,
    previewLight: {
      primary: 'hsl(214 82% 55%)',
      bg: 'hsl(229 11% 95%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(225 19% 88%)',
    },
    previewDark: {
      primary: 'hsl(217 92% 73%)',
      bg: 'hsl(230 24% 16%)',
      surface: 'hsl(229 24% 19%)',
      muted: 'hsl(229 17% 28%)',
    },
  },
  {
    id: 'mono',
    nameKey: 'mono',
    descriptionKey: 'mono',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(222 10% 18%)',
      bg: 'hsl(0 0% 98%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(0 0% 91%)',
    },
    previewDark: {
      primary: 'hsl(0 0% 92%)',
      bg: 'hsl(0 0% 8%)',
      surface: 'hsl(0 0% 12%)',
      muted: 'hsl(0 0% 19%)',
    },
  },
  {
    id: 'github',
    nameKey: 'github',
    descriptionKey: 'github',
    group: 'website',
    recommended: true,
    previewLight: {
      primary: '#1f883d',
      bg: '#f6f8fa',
      surface: '#ffffff',
      muted: '#eaeef2',
      sidebar: '#f6f8fa',
    },
    previewDark: {
      primary: '#3fb950',
      bg: '#0d1117',
      surface: '#161b22',
      muted: '#21262d',
      sidebar: '#0d1117',
    },
  },
  {
    id: 'vercel',
    nameKey: 'vercel',
    descriptionKey: 'vercel',
    group: 'website',
    recommended: true,
    previewLight: {
      primary: '#171717',
      bg: '#fafafa',
      surface: '#ffffff',
      muted: '#ededed',
      sidebar: '#ffffff',
    },
    previewDark: {
      primary: '#ededed',
      bg: '#000000',
      surface: '#0a0a0a',
      muted: '#1a1a1a',
      sidebar: '#000000',
    },
  },
  {
    id: 'google',
    nameKey: 'google',
    descriptionKey: 'google',
    group: 'website',
    previewLight: {
      primary: '#0b57d0',
      bg: '#f8fafd',
      surface: '#ffffff',
      muted: '#e9eef6',
      sidebar: '#edf2fa',
    },
    previewDark: {
      primary: '#a8c7fa',
      bg: '#131314',
      surface: '#1e1f20',
      muted: '#282a2c',
      sidebar: '#1a1b1e',
    },
  },
  {
    id: 'apple',
    nameKey: 'apple',
    descriptionKey: 'apple',
    group: 'website',
    previewLight: {
      primary: '#0071e3',
      bg: '#f5f5f7',
      surface: '#ffffff',
      muted: '#eaeaec',
      sidebar: '#f5f5f7',
    },
    previewDark: {
      primary: '#2997ff',
      bg: '#000000',
      surface: '#1d1d1f',
      muted: '#2b2b2d',
      sidebar: '#111113',
    },
  },
  {
    id: 'notion',
    nameKey: 'notion',
    descriptionKey: 'notion',
    group: 'website',
    previewLight: {
      primary: '#37352f',
      bg: '#ffffff',
      surface: '#ffffff',
      muted: '#f1f1ef',
      sidebar: '#f7f7f5',
    },
    previewDark: {
      primary: '#e3e2de',
      bg: '#191919',
      surface: '#202020',
      muted: '#2c2c2c',
      sidebar: '#202020',
    },
  },
  {
    id: 'stripe',
    nameKey: 'stripe',
    descriptionKey: 'stripe',
    group: 'website',
    previewLight: {
      primary: '#635bff',
      bg: '#f6f9fc',
      surface: '#ffffff',
      muted: '#e9eef5',
      sidebar: '#f3f6fb',
    },
    previewDark: {
      primary: '#a59bff',
      bg: '#0a1020',
      surface: '#141e32',
      muted: '#202c43',
      sidebar: '#10172a',
    },
  },
  {
    id: 'spotify',
    nameKey: 'spotify',
    descriptionKey: 'spotify',
    group: 'website',
    previewLight: {
      primary: '#087a35',
      bg: '#f6f8f6',
      surface: '#ffffff',
      muted: '#e6eee8',
      sidebar: '#edf3ef',
    },
    previewDark: {
      primary: '#1ed760',
      bg: '#121212',
      surface: '#181818',
      muted: '#282828',
      sidebar: '#000000',
    },
  },
  {
    id: 'ocean',
    nameKey: 'ocean',
    descriptionKey: 'ocean',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(202 82% 35%)',
      bg: 'hsl(202 50% 96%)',
      surface: 'hsl(202 35% 99%)',
      muted: 'hsl(202 32% 91%)',
    },
    previewDark: {
      primary: 'hsl(195 78% 64%)',
      bg: 'hsl(207 44% 10%)',
      surface: 'hsl(207 38% 14%)',
      muted: 'hsl(207 28% 20%)',
    },
  },
  {
    id: 'forest',
    nameKey: 'forest',
    descriptionKey: 'forest',
    group: 'light',
    recommended: true,
    previewLight: {
      primary: 'hsl(145 40% 30%)',
      bg: 'hsl(90 24% 95%)',
      surface: 'hsl(90 25% 98%)',
      muted: 'hsl(108 18% 89%)',
    },
    previewDark: {
      primary: 'hsl(139 35% 66%)',
      bg: 'hsl(145 22% 10%)',
      surface: 'hsl(145 19% 14%)',
      muted: 'hsl(143 16% 21%)',
    },
  },
  {
    id: 'lavender',
    nameKey: 'lavender',
    descriptionKey: 'lavender',
    group: 'light',
    previewLight: {
      primary: 'hsl(267 42% 44%)',
      bg: 'hsl(266 40% 97%)',
      surface: 'hsl(270 40% 99%)',
      muted: 'hsl(269 28% 92%)',
    },
    previewDark: {
      primary: 'hsl(269 65% 77%)',
      bg: 'hsl(266 24% 12%)',
      surface: 'hsl(267 22% 17%)',
      muted: 'hsl(267 18% 24%)',
    },
  },
  {
    id: 'amber',
    nameKey: 'amber',
    descriptionKey: 'amber',
    group: 'light',
    previewLight: {
      primary: 'hsl(35 85% 31%)',
      bg: 'hsl(43 48% 96%)',
      surface: 'hsl(42 50% 99%)',
      muted: 'hsl(42 31% 89%)',
    },
    previewDark: {
      primary: 'hsl(42 85% 63%)',
      bg: 'hsl(35 17% 11%)',
      surface: 'hsl(35 16% 15%)',
      muted: 'hsl(35 14% 22%)',
    },
  },
  {
    id: 'espresso',
    nameKey: 'espresso',
    descriptionKey: 'espresso',
    group: 'dark',
    previewLight: {
      primary: 'hsl(22 38% 35%)',
      bg: 'hsl(26 26% 94%)',
      surface: 'hsl(30 27% 98%)',
      muted: 'hsl(28 20% 87%)',
    },
    previewDark: {
      primary: 'hsl(28 46% 70%)',
      bg: 'hsl(20 22% 10%)',
      surface: 'hsl(22 20% 14%)',
      muted: 'hsl(22 17% 21%)',
    },
  },
  {
    id: 'sakura',
    nameKey: 'sakura',
    descriptionKey: 'sakura',
    group: 'light',
    previewLight: {
      primary: 'hsl(345 54% 43%)',
      bg: 'hsl(350 52% 97%)',
      surface: 'hsl(345 45% 99%)',
      muted: 'hsl(347 33% 92%)',
    },
    previewDark: {
      primary: 'hsl(346 68% 74%)',
      bg: 'hsl(340 23% 11%)',
      surface: 'hsl(340 22% 16%)',
      muted: 'hsl(339 18% 23%)',
    },
  },
  {
    id: 'mint',
    nameKey: 'mint',
    descriptionKey: 'mint',
    group: 'light',
    previewLight: {
      primary: 'hsl(164 67% 27%)',
      bg: 'hsl(157 38% 96%)',
      surface: 'hsl(155 38% 99%)',
      muted: 'hsl(157 27% 90%)',
    },
    previewDark: {
      primary: 'hsl(158 52% 65%)',
      bg: 'hsl(166 33% 9%)',
      surface: 'hsl(164 28% 13%)',
      muted: 'hsl(164 23% 20%)',
    },
  },
  {
    id: 'midnight',
    nameKey: 'midnight',
    descriptionKey: 'midnight',
    group: 'dark',
    previewLight: {
      primary: 'hsl(230 60% 45%)',
      bg: 'hsl(228 45% 96%)',
      surface: 'hsl(226 45% 99%)',
      muted: 'hsl(228 29% 91%)',
    },
    previewDark: {
      primary: 'hsl(218 84% 74%)',
      bg: 'hsl(228 45% 7%)',
      surface: 'hsl(228 37% 12%)',
      muted: 'hsl(228 30% 19%)',
    },
  },
  {
    id: 'claude',
    nameKey: 'claude',
    descriptionKey: 'claude',
    group: 'light',
    previewLight: {
      primary: 'hsl(16 76% 50%)',
      bg: 'hsl(30 20% 97%)',
      surface: 'hsl(30 17% 95%)',
      muted: 'hsl(30 12% 91%)',
    },
    previewDark: {
      primary: 'hsl(16 80% 55%)',
      bg: 'hsl(30 12% 10%)',
      surface: 'hsl(30 10% 13%)',
      muted: 'hsl(30 8% 18%)',
    },
  },
  {
    id: 'chatgpt',
    nameKey: 'chatgpt',
    descriptionKey: 'chatgpt',
    group: 'light',
    previewLight: {
      primary: 'hsl(160 84% 33%)',
      bg: 'hsl(0 0% 100%)',
      surface: 'hsl(0 0% 97%)',
      muted: 'hsl(0 0% 93%)',
    },
    previewDark: {
      primary: 'hsl(160 84% 35%)',
      bg: 'hsl(0 0% 13%)',
      surface: 'hsl(0 0% 9%)',
      muted: 'hsl(0 0% 18%)',
    },
  },
  {
    id: 'deepseek',
    nameKey: 'deepseek',
    descriptionKey: 'deepseek',
    group: 'light',
    previewLight: {
      primary: 'hsl(220 100% 50%)',
      bg: 'hsl(214 30% 97%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(214 20% 92%)',
    },
    previewDark: {
      primary: 'hsl(217 91% 60%)',
      bg: 'hsl(222 47% 7%)',
      surface: 'hsl(222 33% 13%)',
      muted: 'hsl(222 25% 20%)',
    },
  },
  {
    id: 'aurora',
    nameKey: 'aurora',
    descriptionKey: 'aurora',
    group: 'light',
    previewLight: {
      primary: 'hsl(173 78% 32%)',
      bg: 'hsl(166 33% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(192 35% 90%)',
    },
    previewDark: {
      primary: 'hsl(168 78% 46%)',
      bg: 'hsl(198 42% 8%)',
      surface: 'hsl(198 31% 12%)',
      muted: 'hsl(198 24% 18%)',
    },
  },
  {
    id: 'rose',
    nameKey: 'rose',
    descriptionKey: 'rose',
    group: 'light',
    previewLight: {
      primary: 'hsl(347 70% 48%)',
      bg: 'hsl(350 35% 97%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(342 28% 91%)',
    },
    previewDark: {
      primary: 'hsl(346 74% 61%)',
      bg: 'hsl(345 20% 10%)',
      surface: 'hsl(345 18% 13%)',
      muted: 'hsl(345 15% 20%)',
    },
  },
  {
    id: 'solarized-light',
    nameKey: 'solarizedLight',
    descriptionKey: 'solarizedLight',
    group: 'light',
    previewLight: {
      primary: 'hsl(205 69% 49%)',
      bg: 'hsl(44 87% 94%)',
      surface: 'hsl(46 42% 88%)',
      muted: 'hsl(180 7% 80%)',
    },
    previewDark: {
      primary: 'hsl(205 69% 55%)',
      bg: 'hsl(192 100% 11%)',
      surface: 'hsl(192 81% 14%)',
      muted: 'hsl(192 60% 18%)',
    },
  },
  {
    id: 'quiet-light',
    nameKey: 'quietLight',
    descriptionKey: 'quietLight',
    group: 'light',
    previewLight: {
      primary: 'hsl(283 35% 47%)',
      bg: 'hsl(0 0% 96%)',
      surface: 'hsl(0 0% 98%)',
      muted: 'hsl(0 0% 92%)',
    },
    previewDark: {
      primary: 'hsl(283 45% 65%)',
      bg: 'hsl(220 14% 14%)',
      surface: 'hsl(220 12% 17%)',
      muted: 'hsl(220 10% 22%)',
    },
  },
  {
    id: 'ayu-light',
    nameKey: 'ayuLight',
    descriptionKey: 'ayuLight',
    group: 'light',
    previewLight: {
      primary: 'hsl(28 100% 56%)',
      bg: 'hsl(0 0% 98%)',
      surface: 'hsl(0 0% 95%)',
      muted: 'hsl(210 9% 90%)',
    },
    previewDark: {
      primary: 'hsl(28 100% 63%)',
      bg: 'hsl(223 21% 16%)',
      surface: 'hsl(220 19% 18%)',
      muted: 'hsl(220 13% 24%)',
    },
  },
  {
    id: 'noctis-lux',
    nameKey: 'noctisLux',
    descriptionKey: 'noctisLux',
    group: 'light',
    previewLight: {
      primary: 'hsl(34 92% 44%)',
      bg: 'hsl(36 64% 88%)',
      surface: 'hsl(50 84% 93%)',
      muted: 'hsl(45 30% 84%)',
    },
    previewDark: {
      primary: 'hsl(34 92% 54%)',
      bg: 'hsl(189 70% 9%)',
      surface: 'hsl(190 100% 12%)',
      muted: 'hsl(190 60% 16%)',
    },
  },
  {
    id: 'one-dark-pro',
    nameKey: 'oneDarkPro',
    descriptionKey: 'oneDarkPro',
    group: 'editor',
    previewLight: {
      primary: 'hsl(221 87% 60%)',
      bg: 'hsl(0 0% 98%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(240 7% 92%)',
    },
    previewDark: {
      primary: 'hsl(207 82% 66%)',
      bg: 'hsl(220 13% 18%)',
      surface: 'hsl(220 12% 15%)',
      muted: 'hsl(220 11% 22%)',
    },
  },
  {
    id: 'github-dimmed',
    nameKey: 'githubDimmed',
    descriptionKey: 'githubDimmed',
    group: 'editor',
    previewLight: {
      primary: 'hsl(212 92% 45%)',
      bg: 'hsl(0 0% 100%)',
      surface: 'hsl(210 29% 97%)',
      muted: 'hsl(210 24% 93%)',
    },
    previewDark: {
      primary: 'hsl(212 89% 64%)',
      bg: 'hsl(213 13% 16%)',
      surface: 'hsl(216 13% 20%)',
      muted: 'hsl(213 11% 25%)',
    },
  },
  {
    id: 'dracula',
    nameKey: 'dracula',
    descriptionKey: 'dracula',
    group: 'editor',
    previewLight: {
      primary: 'hsl(265 60% 55%)',
      bg: 'hsl(60 30% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(264 14% 92%)',
    },
    previewDark: {
      primary: 'hsl(265 89% 78%)',
      bg: 'hsl(231 15% 18%)',
      surface: 'hsl(232 14% 23%)',
      muted: 'hsl(232 14% 31%)',
    },
  },
  {
    id: 'monokai-pro',
    nameKey: 'monokaiPro',
    descriptionKey: 'monokaiPro',
    group: 'editor',
    previewLight: {
      primary: 'hsl(341 75% 58%)',
      bg: 'hsl(13 33% 96%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(20 28% 92%)',
    },
    previewDark: {
      primary: 'hsl(349 100% 70%)',
      bg: 'hsl(290 6% 17%)',
      surface: 'hsl(285 4% 22%)',
      muted: 'hsl(285 3% 30%)',
    },
  },
  {
    id: 'nord',
    nameKey: 'nord',
    descriptionKey: 'nord',
    group: 'dark',
    previewLight: {
      primary: 'hsl(213 32% 52%)',
      bg: 'hsl(218 27% 94%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(218 27% 88%)',
    },
    previewDark: {
      primary: 'hsl(193 43% 67%)',
      bg: 'hsl(220 16% 22%)',
      surface: 'hsl(222 16% 28%)',
      muted: 'hsl(222 14% 31%)',
    },
  },
  {
    id: 'catppuccin',
    nameKey: 'catppuccin',
    descriptionKey: 'catppuccin',
    group: 'dark',
    previewLight: {
      primary: 'hsl(266 85% 58%)',
      bg: 'hsl(220 23% 95%)',
      surface: 'hsl(0 0% 100%)',
      muted: 'hsl(223 16% 88%)',
    },
    previewDark: {
      primary: 'hsl(267 84% 81%)',
      bg: 'hsl(240 21% 15%)',
      surface: 'hsl(240 21% 20%)',
      muted: 'hsl(234 13% 31%)',
    },
  },
  {
    id: 'gruvbox',
    nameKey: 'gruvbox',
    descriptionKey: 'gruvbox',
    group: 'dark',
    previewLight: {
      primary: 'hsl(35 80% 39%)',
      bg: 'hsl(48 87% 88%)',
      surface: 'hsl(53 73% 91%)',
      muted: 'hsl(43 59% 81%)',
    },
    previewDark: {
      primary: 'hsl(40 78% 50%)',
      bg: 'hsl(0 0% 16%)',
      surface: 'hsl(20 6% 22%)',
      muted: 'hsl(15 8% 30%)',
    },
  },
]

export const THEME_GROUP_ORDER: ThemeGroup[] = ['recommended', 'website', 'light', 'dark', 'editor']

export function getThemePreviewSwatch(item: ColorThemeDef, mode: 'light' | 'dark'): ThemePreviewSwatch {
  return mode === 'dark' ? item.previewDark : item.previewLight
}
