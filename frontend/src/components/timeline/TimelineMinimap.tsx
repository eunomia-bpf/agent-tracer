'use client';

import { ProcessedEvent } from '@/types/event';

interface TimelineMinimapProps {
  events: ProcessedEvent[];
  fullTimeRange: { start: number; end: number };
  visibleTimeRange: { start: number; end: number };
  baseTimeSpan: number;
  timeSpan: number;
  scrollOffset: number;
}

interface TimelineGroup {
  source: string;
  events: ProcessedEvent[];
  color: string;
}

export function TimelineMinimap({
  events,
  fullTimeRange,
  visibleTimeRange,
  baseTimeSpan,
  timeSpan,
  scrollOffset
}: TimelineMinimapProps) {
  // Group events by source for minimap
  const timelineGroups: TimelineGroup[] = [];
  const grouped: { [source: string]: ProcessedEvent[] } = {};
  
  events.forEach(event => {
    if (!grouped[event.source]) {
      grouped[event.source] = [];
    }
    grouped[event.source].push(event);
  });

  Object.entries(grouped).forEach(([source, sourceEvents]) => {
    timelineGroups.push({
      source,
      events: sourceEvents,
      color: sourceEvents[0]?.sourceColor || '#6B7280'
    });
  });

  return (
    <div className="mb-4">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs text-gray-600">Timeline Overview</span>
        <span className="text-xs text-gray-500">
          {Math.round((scrollOffset / (baseTimeSpan - timeSpan)) * 100)}% scrolled
        </span>
      </div>
      <div className="relative h-4 bg-gray-100 rounded-sm">
        {/* Full timeline background */}
        <div className="absolute inset-0 bg-gray-200 rounded-sm" />
        
        {/* Visible range indicator */}
        <div
          className="absolute top-0 h-full bg-blue-300 rounded-sm opacity-50"
          style={{
            left: `${(scrollOffset / baseTimeSpan) * 100}%`,
            width: `${(timeSpan / baseTimeSpan) * 100}%`
          }}
        />
        
        {/* Events dots in minimap */}
        {timelineGroups.map((group) => 
          group.events.map((event) => {
            const position = ((event.timestamp - fullTimeRange.start) / baseTimeSpan) * 100;
            return (
              <div
                key={event.id}
                className="absolute top-1 w-0.5 h-2 opacity-60"
                style={{
                  left: `${position}%`,
                  backgroundColor: group.color
                }}
              />
            );
          })
        )}
      </div>
    </div>
  );
}