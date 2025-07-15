'use client';

interface TimelineAxisProps {
  startTime: number;
  endTime: number;
  timeSpan: number;
}

export function TimelineAxis({ startTime, endTime, timeSpan }: TimelineAxisProps) {
  return (
    <div className="relative h-8 border-b border-gray-200">
      <div className="absolute left-0 top-0 text-xs text-gray-500">
        {new Date(startTime).toLocaleTimeString()}
      </div>
      <div className="absolute right-0 top-0 text-xs text-gray-500">
        {new Date(endTime).toLocaleTimeString()}
      </div>
      {/* Time markers */}
      {Array.from({ length: 5 }, (_, i) => {
        const position = (i / 4) * 100;
        const time = startTime + (timeSpan * i / 4);
        return (
          <div
            key={i}
            className="absolute top-4 w-px h-4 bg-gray-200"
            style={{ left: `${position}%` }}
          >
            <div className="absolute top-5 text-xs text-gray-400 transform -translate-x-1/2">
              {new Date(time).toLocaleTimeString('en-US', {
                hour12: false,
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit'
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}