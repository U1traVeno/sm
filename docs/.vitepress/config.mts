import { defineConfig } from 'vitepress'

const repository = 'https://github.com/U1traVeno/sm'

const englishSidebar = [
  {
    text: 'Guide',
    items: [
      { text: 'Getting Started', link: '/getting-started' },
      { text: 'Profiles and Activation', link: '/profiles' },
      { text: 'Targets and Templates', link: '/targets' },
      { text: 'Isolated Shells', link: '/shell' },
      { text: 'Project Import and Export', link: '/project-skills' },
    ],
  },
  {
    text: 'Reference',
    items: [
      { text: 'Command Reference', link: '/command-reference' },
      { text: 'Filesystem Layout', link: '/filesystem-layout' },
    ],
  },
]

const chineseSidebar = [
  {
    text: '指南',
    items: [
      { text: '快速开始', link: '/zh-cn/getting-started' },
      { text: 'Profiles 与激活', link: '/zh-cn/profiles' },
      { text: 'Targets 与模板', link: '/zh-cn/targets' },
      { text: '隔离 Shell', link: '/zh-cn/shell' },
      { text: '项目 Skill 的导入与导出', link: '/zh-cn/project-skills' },
    ],
  },
  {
    text: '参考',
    items: [
      { text: '命令参考', link: '/zh-cn/command-reference' },
      { text: '文件系统布局', link: '/zh-cn/filesystem-layout' },
    ],
  },
]

export default defineConfig({
  base: process.env.BASE_PATH || '/',
  cleanUrls: true,
  lastUpdated: true,
  title: 'sm',
  description: 'Activate skill profiles on demand.',
  head: [['meta', { name: 'theme-color', content: '#3c6e71' }]],
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      title: 'sm',
      description: 'Activate skill profiles on demand.',
    },
    'zh-cn': {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh-cn/',
      title: 'sm',
      description: '按需激活 Skill Profiles。',
    },
  },
  themeConfig: {
    search: { provider: 'local' },
    socialLinks: [{ icon: 'github', link: repository }],
    locales: {
      root: {
        nav: [
          { text: 'Guide', link: '/getting-started' },
          { text: 'Commands', link: '/command-reference' },
        ],
        sidebar: englishSidebar,
        outline: { label: 'On this page', level: [2, 3] },
        editLink: {
          pattern: `${repository}/edit/main/docs/:path`,
          text: 'Edit this page on GitHub',
        },
        docFooter: { prev: 'Previous page', next: 'Next page' },
        lastUpdated: { text: 'Last updated' },
      },
      'zh-cn': {
        nav: [
          { text: '指南', link: '/zh-cn/getting-started' },
          { text: '命令', link: '/zh-cn/command-reference' },
        ],
        sidebar: chineseSidebar,
        outline: { label: '本页目录', level: [2, 3] },
        editLink: {
          pattern: `${repository}/edit/main/docs/:path`,
          text: '在 GitHub 上编辑此页',
        },
        docFooter: { prev: '上一页', next: '下一页' },
        lastUpdated: { text: '最后更新' },
      },
    },
  },
})
