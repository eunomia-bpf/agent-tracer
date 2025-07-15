'use client';

import { useState, useMemo, useEffect } from 'react';
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
  const [selectedSource, setSelectedSource] = useState<string>('');
  const [selectedComm, setSelectedComm] = useState<string>('');
  const [selectedPid, setSelectedPid] = useState<string>('');
  const [zoomLevel, setZoomLevel] = useState<number>(1);
  const [scrollOffset, setScrollOffset] = useState<number>(0);

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

  // Get unique values for filters
  const sources = useMemo(() => {
    const unique = new Set(processedEvents.map(event => event.source));
    return Array.from(unique).sort();
  }, [processedEvents]);

  const commValues = useMemo(() => {
    const unique = new Set(processedEvents.map(event => event.comm));
    return Array.from(unique).sort();
  }, [processedEvents]);

  const pidValues = useMemo(() => {
    const unique = new Set(processedEvents.map(event => event.pid.toString()));
    return Array.from(unique).sort((a, b) => parseInt(a) - parseInt(b));
  }, [processedEvents]);

  // Filter events based on selected filters
  const filteredEvents = useMemo(() => {
    let filtered = processedEvents;

    if (selectedSource) {
      filtered = filtered.filter(event => event.source === selectedSource);
    }

    if (selectedComm) {
      filtered = filtered.filter(event => event.comm === selectedComm);
    }

    if (selectedPid) {
      filtered = filtered.filter(event => event.pid.toString() === selectedPid);
    }

    return filtered;
  }, [processedEvents, selectedSource, selectedComm, selectedPid]);

  // Group filtered events by source
  const timelineGroups: TimelineGroup[] = useMemo(() => {
    const grouped: { [source: string]: ProcessedEvent[] } = {};
    filteredEvents.forEach(event => {
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
  }, [filteredEvents]);

  // Calculate time range
  const fullTimeRange = useMemo(() => {
    if (filteredEvents.length === 0) return { start: 0, end: 0 };
    
    const timestamps = filteredEvents.map(e => e.timestamp);
    return {
      start: Math.min(...timestamps),
      end: Math.max(...timestamps)
    };
  }, [filteredEvents]);

  const visibleTimeRange = useMemo(() => {
    if (timeRange) return timeRange;
    if (zoomLevel === 1) return fullTimeRange;
    
    // When zoomed, calculate the visible range based on zoom level and scroll offset
    const zoomedSpan = (fullTimeRange.end - fullTimeRange.start) / zoomLevel;
    const maxOffset = (fullTimeRange.end - fullTimeRange.start) - zoomedSpan;
    const clampedOffset = Math.max(0, Math.min(scrollOffset, maxOffset));
    
    return {
      start: fullTimeRange.start + clampedOffset,
      end: fullTimeRange.start + clampedOffset + zoomedSpan
    };
  }, [timeRange, fullTimeRange, zoomLevel, scrollOffset]);
  
  const baseTimeSpan = fullTimeRange.end - fullTimeRange.start;
  const timeSpan = visibleTimeRange.end - visibleTimeRange.start;

  // Calculate position for an event in the timeline
  const getEventPosition = (timestamp: number) => {
    if (timeSpan === 0) return 0;
    return ((timestamp - visibleTimeRange.start) / timeSpan) * 100;
  };

  // Zoom functions
  const zoomIn = () => {
    setZoomLevel(prev => Math.min(prev * 1.5, 10));
  };

  const zoomOut = () => {
    setZoomLevel(prev => Math.max(prev / 1.5, 0.1));
  };

  const resetZoom = () => {
    setZoomLevel(1);
    setScrollOffset(0);
    setTimeRange(null);
  };

  // Scroll functions
  const scrollLeft = () => {
    const zoomedSpan = baseTimeSpan / zoomLevel;
    const scrollStep = zoomedSpan * 0.1; // 10% of visible range
    setScrollOffset(prev => Math.max(0, prev - scrollStep));
  };

  const scrollRight = () => {
    const zoomedSpan = baseTimeSpan / zoomLevel;
    const scrollStep = zoomedSpan * 0.1; // 10% of visible range
    const maxOffset = baseTimeSpan - zoomedSpan;
    setScrollOffset(prev => Math.min(maxOffset, prev + scrollStep));
  };

  // Handle mouse wheel zoom and scroll
  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      // Zoom with Ctrl/Cmd + wheel
      e.preventDefault();
      const delta = e.deltaY;
      if (delta < 0) {
        zoomIn();
      } else {
        zoomOut();
      }
    } else if (zoomLevel > 1) {
      // Horizontal scroll when zoomed
      e.preventDefault();
      const delta = e.deltaY;
      const zoomedSpan = baseTimeSpan / zoomLevel;
      const scrollStep = zoomedSpan * 0.05; // 5% of visible range
      const maxOffset = baseTimeSpan - zoomedSpan;
      
      if (delta > 0) {
        setScrollOffset(prev => Math.min(maxOffset, prev + scrollStep));
      } else {
        setScrollOffset(prev => Math.max(0, prev - scrollStep));
      }
    }
  };

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === '=' || e.key === '+') {
          e.preventDefault();
          zoomIn();
        } else if (e.key === '-') {
          e.preventDefault();
          zoomOut();
        } else if (e.key === '0') {
          e.preventDefault();
          resetZoom();
        }
      } else if (zoomLevel > 1) {
        // Arrow keys for scrolling when zoomed
        if (e.key === 'ArrowLeft') {
          e.preventDefault();
          scrollLeft();
        } else if (e.key === 'ArrowRight') {
          e.preventDefault();
          scrollRight();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [zoomLevel, scrollLeft, scrollRight]);

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
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Timeline View</h2>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <button
                onClick={zoomOut}
                className="p-1 hover:bg-gray-100 rounded-md transition-colors"
                title="Zoom Out"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6" />
                </svg>
              </button>
              <span className="text-sm text-gray-600 font-mono min-w-[4rem] text-center">
                {Math.round(zoomLevel * 100)}%
              </span>
              <button
                onClick={zoomIn}
                className="p-1 hover:bg-gray-100 rounded-md transition-colors"
                title="Zoom In"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v6m3-3H9" />
                </svg>
              </button>
              <button
                onClick={resetZoom}
                className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded-md transition-colors"
                title="Reset Zoom"
              >
                Reset
              </button>
            </div>
            
            {/* Scroll Controls - Only show when zoomed */}
            {zoomLevel > 1 && (
              <div className="flex items-center gap-1 px-2 py-1 bg-gray-50 rounded-md">
                <button
                  onClick={scrollLeft}
                  className="p-1 hover:bg-gray-200 rounded-sm transition-colors"
                  title="Scroll Left"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                  </svg>
                </button>
                <span className="text-xs text-gray-600 px-2">Scroll</span>
                <button
                  onClick={scrollRight}
                  className="p-1 hover:bg-gray-200 rounded-sm transition-colors"
                  title="Scroll Right"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                  </svg>
                </button>
              </div>
            )}
            <div className="text-sm text-gray-600">
              Duration: {formatDuration(baseTimeSpan)} • {filteredEvents.length} events
            </div>
          </div>
        </div>
        
        {/* Zoom Help Text */}
        <div className="text-xs text-gray-500 mb-2">
          Use mouse wheel + Ctrl/Cmd to zoom, or Ctrl/Cmd + +/- keys. Press Ctrl/Cmd + 0 to reset.
          {zoomLevel > 1 && (
            <span className="ml-2 text-blue-600">
              Scroll with mouse wheel or arrow keys when zoomed.
            </span>
          )}
        </div>
        
        {/* Filters */}
        <div className="flex flex-col sm:flex-row gap-4">
          <div className="flex-1">
            <select
              value={selectedSource}
              onChange={(e) => setSelectedSource(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="">All Sources</option>
              {sources.map(source => (
                <option key={source} value={source}>
                  {source} ({processedEvents.filter(e => e.source === source).length})
                </option>
              ))}
            </select>
          </div>
          
          <div className="flex-1">
            <select
              value={selectedComm}
              onChange={(e) => setSelectedComm(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="">All Processes</option>
              {commValues.map(comm => (
                <option key={comm} value={comm}>
                  {comm} ({processedEvents.filter(e => e.comm === comm).length})
                </option>
              ))}
            </select>
          </div>
          
          <div className="flex-1">
            <select
              value={selectedPid}
              onChange={(e) => setSelectedPid(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="">All PIDs</option>
              {pidValues.map(pid => (
                <option key={pid} value={pid}>
                  PID {pid} ({processedEvents.filter(e => e.pid.toString() === pid).length})
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Timeline */}
      <div className="p-4" onWheel={handleWheel}>
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

            {/* Scroll indicator/minimap - Only show when zoomed */}
            {zoomLevel > 1 && (
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
            )}

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
                  {group.events.map((event) => {
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