'use client';

import { ProcessedEvent } from '@/types/event';
import { formatEventSummary } from '@/utils/eventProcessing';

interface TimelineGroupProps {
  source: string;
  events: ProcessedEvent[];
  color: string;
  getEventPosition: (timestamp: number) => number;
  onEventClick: (event: ProcessedEvent) => void;
}

export function TimelineGroup({
  source,
  events,
  color,
  getEventPosition,
  onEventClick
}: TimelineGroupProps) {
  return (
    <div className="relative">
      {/* Source label */}
      <div className="flex items-center mb-2">
        <div 
          className="w-4 h-4 rounded-full mr-3"
          style={{ backgroundColor: color }}
        />
        <span className="font-medium text-gray-900 text-sm">
          {source}
        </span>
        <span className="ml-2 text-xs text-gray-500">
          ({events.length} events)
        </span>
      </div>

      {/* Timeline bar */}
      <div className="relative h-8 bg-gray-50 rounded-md mb-2">
        {events.map((event) => {
          const position = getEventPosition(event.timestamp);
          const isVisible = position >= 0 && position <= 100;
          
          if (!isVisible) return null;

          return (
            <div
              key={event.id}
              className="absolute top-1 h-6 cursor-pointer transform -translate-x-1/2 group"
              style={{ left: `${position}%` }}
              onClick={() => onEventClick(event)}
            >
              <div
                className="w-2 h-6 rounded-sm shadow-sm hover:shadow-md transition-shadow"
                style={{ backgroundColor: color }}
              />
              
              {/* Tooltip */}
              <div className="absolute bottom-8 left-1/2 transform -translate-x-1/2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                <div className="bg-black text-white text-xs rounded px-2 py-1 whitespace-nowrap">
                  {formatEventSummary(event)}
                  <div className="text-gray-300">
                    {event.formattedTime}
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}