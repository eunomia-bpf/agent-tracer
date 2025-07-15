'use client';

import { useState, useMemo } from 'react';
import { Event, GroupedEvents, ProcessedEvent } from '@/types/event';

interface TimelineViewProps {
  events: Event[];
}

interface TimelineGroup {
  source: string;
  events: ProcessedEvent[];
  color: string;
}

export function TimelineView({ events }: TimelineViewProps) {
  const [selectedEvent, setSelectedEvent] = useState<ProcessedEvent | null>(null);
  const [timeRange, setTimeRange] = useState<{ start: number; end: number } | null>(null);

  // Process events with additional metadata
  const processedEvents: ProcessedEvent[] = useMemo(() => {
    const sourceColors = [
      '#3B82F6', // blue
      '#10B981', // green
      '#F59E0B', // yellow
      '#8B5CF6', // purple
      '#EF4444', // red
      '#6366F1', // indigo
      '#EC4899', // pink
      '#6B7280'  // gray
    ];

    const sourceColorMap = new Map<string, string>();
    let colorIndex = 0;

    return events.map(event => {
      const datetime = new Date(event.timestamp);
      const formattedTime = datetime.toLocaleTimeString('en-US', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      }) + '.' + datetime.getMilliseconds().toString().padStart(3, '0');

      // Assign colors to sources
      if (!sourceColorMap.has(event.source)) {
        sourceColorMap.set(event.source, sourceColors[colorIndex % sourceColors.length]);
        colorIndex++;
      }

      return {
        ...event,
        datetime,
        formattedTime,
        sourceColor: sourceColorMap.get(event.source) || sourceColors[0]
      };
    });
  }, [events]);

  // Group events by source
  const timelineGroups: TimelineGroup[] = useMemo(() => {
    const grouped: { [source: string]: ProcessedEvent[] } = {};
    processedEvents.forEach(event => {
      if (!grouped[event.source]) {
        grouped[event.source] = [];
      }
      grouped[event.source].push(event);
    });

    return Object.entries(grouped).map(([source, events]) => ({
      source,
      events: events.sort((a, b) => a.timestamp - b.timestamp),
      color: events[0]?.sourceColor || '#6B7280'
    }));
  }, [processedEvents]);

  // Calculate time range
  const fullTimeRange = useMemo(() => {
    if (processedEvents.length === 0) return { start: 0, end: 0 };
    
    const timestamps = processedEvents.map(e => e.timestamp);
    return {
      start: Math.min(...timestamps),
      end: Math.max(...timestamps)
    };
  }, [processedEvents]);

  const visibleTimeRange = timeRange || fullTimeRange;
  const timeSpan = visibleTimeRange.end - visibleTimeRange.start;

  // Calculate position for an event in the timeline
  const getEventPosition = (timestamp: number) => {
    if (timeSpan === 0) return 0;
    return ((timestamp - visibleTimeRange.start) / timeSpan) * 100;
  };

  // Format duration
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  // Format event label for timeline
  const formatEventLabel = (event: ProcessedEvent) => {
    return `${event.comm} (${event.pid})`;
  };

  // Get event color based on source
  const getEventColor = (event: ProcessedEvent) => {
    return event.sourceColor;
  };

  return (
    <div className="bg-white rounded-lg shadow-md">
      {/* Timeline Header */}
      <div className="border-b border-gray-200 p-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-gray-900">Timeline View</h2>
          <div className="text-sm text-gray-600">
            Duration: {formatDuration(timeSpan)} • {processedEvents.length} events
          </div>
        </div>
      </div>

      {/* Timeline */}
      <div className="p-4">
        {timelineGroups.length === 0 ? (
          <div className="text-center text-gray-500 py-8">
            No events to display
          </div>
        ) : (
          <div className="space-y-6">
            {/* Time axis */}
            <div className="relative h-8 border-b border-gray-200">
              <div className="absolute left-0 top-0 text-xs text-gray-500">
                {new Date(visibleTimeRange.start).toLocaleTimeString()}
              </div>
              <div className="absolute right-0 top-0 text-xs text-gray-500">
                {new Date(visibleTimeRange.end).toLocaleTimeString()}
              </div>
              {/* Time markers */}
              {Array.from({ length: 5 }, (_, i) => {
                const position = (i / 4) * 100;
                const time = visibleTimeRange.start + (timeSpan * i / 4);
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

            {/* Timeline Groups */}
            {timelineGroups.map((group) => (
              <div key={group.source} className="relative">
                {/* Source label */}
                <div className="flex items-center mb-2">
                  <div 
                    className="w-4 h-4 rounded-full mr-3"
                    style={{ backgroundColor: group.color }}
                  />
                  <span className="font-medium text-gray-900 text-sm">
                    {group.source}
                  </span>
                  <span className="ml-2 text-xs text-gray-500">
                    ({group.events.length} events)
                  </span>
                </div>

                {/* Timeline bar */}
                <div className="relative h-8 bg-gray-50 rounded-md mb-2">
                  {group.events.map((event, index) => {
                    const position = getEventPosition(event.timestamp);
                    const isVisible = position >= 0 && position <= 100;
                    
                    if (!isVisible) return null;

                    return (
                      <div
                        key={event.id}
                        className="absolute top-1 h-6 cursor-pointer transform -translate-x-1/2 group"
                        style={{ left: `${position}%` }}
                        onClick={() => setSelectedEvent(event)}
                      >
                        <div
                          className="w-2 h-6 rounded-sm shadow-sm hover:shadow-md transition-shadow"
                          style={{ backgroundColor: getEventColor(event) }}
                        />
                        
                        {/* Tooltip */}
                        <div className="absolute bottom-8 left-1/2 transform -translate-x-1/2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                          <div className="bg-black text-white text-xs rounded px-2 py-1 whitespace-nowrap">
                            {formatEventLabel(event)}
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
            ))}
          </div>
        )}
      </div>

      {/* Event Details Modal */}
      {selectedEvent && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg max-w-4xl w-full max-h-[80vh] overflow-y-auto">
            <div className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold text-gray-900">
                  Timeline Event Details
                </h2>
                <button
                  onClick={() => setSelectedEvent(null)}
                  className="text-gray-500 hover:text-gray-700"
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Process</label>
                    <div className="text-sm text-gray-900">{selectedEvent.comm}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">PID</label>
                    <div className="text-sm text-gray-900 font-mono">{selectedEvent.pid}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Source</label>
                    <span className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${selectedEvent.sourceColor}`}>
                      {selectedEvent.source}
                    </span>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Time</label>
                    <div className="text-sm text-gray-900">{selectedEvent.formattedTime}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Timestamp</label>
                    <div className="text-sm text-gray-900">{selectedEvent.datetime.toLocaleString()}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Unix Timestamp</label>
                    <div className="text-sm text-gray-900 font-mono">{selectedEvent.timestamp}</div>
                  </div>
                </div>


                {/* Raw Data */}
                <div className="border-t pt-4">
                  <h3 className="font-medium text-gray-900 mb-2">Raw Data</h3>
                  <div className="bg-gray-50 rounded-md p-3 max-h-64 overflow-y-auto">
                    <pre className="text-sm text-gray-800 font-mono whitespace-pre-wrap">
                      {JSON.stringify(selectedEvent.data, null, 2)}
                    </pre>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}