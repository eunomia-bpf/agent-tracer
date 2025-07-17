/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  experimental: {
    appDir: true,
  },
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
}

module.exports = nextConfig