'use client';

import { ProcessedEvent } from '@/types/event';
import { formatEventSummary } from '@/utils/eventProcessing';

interface LogListProps {
  events: ProcessedEvent[];
  onEventClick: (event: ProcessedEvent) => void;
}

export function LogList({ events, onEventClick }: LogListProps) {
  if (events.length === 0) {
    return (
      <div className="p-8 text-center text-gray-500">
        No events found matching the current filters.
      </div>
    );
  }

  return (
    <div className="divide-y divide-gray-200">
      {events.map((event) => (
        <div
          key={event.id}
          className="p-4 hover:bg-gray-50 cursor-pointer"
          onClick={() => onEventClick(event)}
        >
          <div className="flex items-start justify-between">
            <div className="flex-1 min-w-0">
              <div className="flex items-center space-x-3 mb-1">
                <span className="font-mono text-sm text-gray-500">
                  {event.formattedTime}
                </span>
                <span className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${event.sourceColorClass}`}>
                  {event.source}
                </span>
              </div>
              <div className="text-sm text-gray-900 mb-1">
                {formatEventSummary(event)}
              </div>
              <div className="text-xs text-gray-500 font-mono">
                ID: {event.id}
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}