import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-r from-blue-500 to-purple-600">
      <div className="text-center p-8 bg-white bg-opacity-10 rounded-lg backdrop-blur-md shadow-lg">
        <h1 className="text-6xl font-bold text-white mb-4">
          Agent Tracer
        </h1>
        <p className="text-xl text-white opacity-90 mb-8">
          eBPF-based system event tracing and visualization
        </p>
        <div className="space-y-4">
          <div className="inline-block px-6 py-3 bg-white bg-opacity-20 rounded-full text-white font-semibold">
            Built with Next.js + TypeScript + Tailwind CSS
          </div>
          <div className="mt-8">
            <Link
              href="/timeline"
              className="inline-block px-8 py-4 bg-white bg-opacity-20 hover:bg-opacity-30 rounded-lg text-white font-semibold transition-all duration-200 hover:scale-105"
            >
              View Timeline →
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}