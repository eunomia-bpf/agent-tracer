export default function Home() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-r from-blue-500 to-purple-600">
      <div className="text-center p-8 bg-white bg-opacity-10 rounded-lg backdrop-blur-md shadow-lg">
        <h1 className="text-6xl font-bold text-white mb-4">
          Hello World
        </h1>
        <p className="text-xl text-white opacity-90">
          Welcome to Agent Tracer Frontend
        </p>
        <div className="mt-8">
          <div className="inline-block px-6 py-3 bg-white bg-opacity-20 rounded-full text-white font-semibold">
            Built with Next.js + TypeScript + Tailwind CSS
          </div>
        </div>
      </div>
    </div>
  )
}