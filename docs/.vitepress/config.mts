import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'OxiDex',
  description: 'Modern, high-performance Rust implementation of ExifTool',
  lang: 'en-US',
  base: '/', // Custom domain (oxidex.net) serves from root
  outDir: '.vitepress/dist',
  cleanUrls: true,
  lastUpdated: true,

  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'OxiDex',

    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/' },
      { text: 'Reference', link: '/reference/' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'Performance', link: '/performance/' },
      {
        text: 'v1.2.1',
        items: [
          { text: 'Changelog', link: '/changelog' },
          { text: 'Contributing', link: '/contributing/' }
        ]
      }
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/' },
            { text: 'Installation', link: '/guide/getting-started' },
            { text: 'CLI Usage', link: '/guide/cli-usage' },
            { text: 'Library API', link: '/guide/library-api' },
            { text: 'MCP Integration', link: '/guide/mcp-integration' },
            { text: 'Troubleshooting', link: '/guide/troubleshooting' }
          ]
        }
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'Overview', link: '/reference/' },
            { text: 'Architecture', link: '/reference/architecture' },
            { text: 'API Reference', link: '/reference/api-reference' },
            { text: 'FFI API', link: '/reference/ffi-api' },
            { text: 'Tag Database', link: '/reference/tag-database' }
          ]
        },
        {
          text: 'Formats',
          items: [
            { text: 'Overview', link: '/reference/formats/' }
          ]
        },
        {
          text: 'API Documentation',
          items: [
            { text: 'Rust API', link: '/reference/api/' }
          ]
        },
        {
          text: 'Packaging',
          items: [
            { text: 'Distribution', link: '/reference/packaging/' }
          ]
        }
      ],
      '/architecture/': [
        {
          text: 'Architecture',
          items: [
            { text: 'Overview', link: '/architecture/' },
            { text: 'Hexagonal Architecture', link: '/architecture/hexagonal' },
            { text: 'Domain Model', link: '/architecture/domain-model' },
            { text: 'Parser Design', link: '/architecture/parser-design' }
          ]
        },
        {
          text: 'Diagrams',
          items: [
            { text: 'System Diagrams', link: '/diagrams/' }
          ]
        }
      ],
      '/performance/': [
        {
          text: 'Performance',
          items: [
            { text: 'Overview', link: '/performance/' },
            { text: 'Benchmarks', link: '/performance/benchmarks' },
            { text: 'Profiling', link: '/performance/profiling' },
            { text: 'Optimization Strategy', link: '/performance/optimization-strategy' }
          ]
        }
      ],
      '/contributing/': [
        {
          text: 'Contributing',
          items: [
            { text: 'Getting Started', link: '/contributing/' }
          ]
        },
        {
          text: 'Development',
          items: [
            { text: 'Development Guide', link: '/contributing/development/' },
            { text: 'Development Environment', link: '/contributing/development/environment' },
            { text: 'Code Style', link: '/contributing/development/code-style' }
          ]
        },
        {
          text: 'Testing',
          items: [
            { text: 'Testing Guide', link: '/contributing/testing/' },
            { text: 'Test Strategy', link: '/contributing/testing/strategy' },
            { text: 'Writing Tests', link: '/contributing/testing/writing-tests' }
          ]
        }
      ],
      '/tag-domains/': [
        {
          text: 'Tag Domains',
          items: [
            { text: 'Overview', link: '/tag-domains/' },
            { text: 'Domain Reference', link: '/tag-domains/reference' }
          ]
        }
      ],
      '/diagrams/': [
        {
          text: 'Diagrams',
          items: [
            { text: 'System Architecture', link: '/diagrams/' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/swack-tools/oxidex' }
    ],

    footer: {
      message: 'Released under the GPL-3.0 License.',
      copyright: 'Copyright © 2024 OxiDex Contributors'
    },

    editLink: {
      pattern: 'https://github.com/swack-tools/oxidex/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    },

    search: {
      provider: 'local'
    },

    outline: [2, 3]
  },

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/oxidex/logo.svg' }],
    ['meta', { name: 'theme-color', content: '#dd7732' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:locale', content: 'en' }],
    ['meta', { name: 'og:site_name', content: 'OxiDex' }]
  ],

  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark'
    },
    lineNumbers: true
  },

  ignoreDeadLinks: [
    // Benchmark reports - deployed separately by CI
    /^\/benchmarks\//
  ]
})
