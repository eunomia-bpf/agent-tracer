'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Event, ProcessedEvent } from '@/types/event';
import { processEvents, filterEvents, formatDuration } from '@/utils/eventProcessing';
import { EventFilters } from '@/components/common/EventFilters';
import { EventModal } from '@/components/common/EventModal';
import { ZoomControls } from './ZoomControls';
import { TimelineAxis } from './TimelineAxis';
import { TimelineMinimap } from './TimelineMinimap';
import { TimelineGroup } from './TimelineGroup';
import { TimelineScrollBar } from './TimelineScrollBar';

interface TimelineProps {
  events: Event[];
}

interface TimelineGroupData {
  source: string;
  events: ProcessedEvent[];
  color: string;
}

export function Timeline({ events }: TimelineProps) {
  const [selectedEvent, setSelectedEvent] = useState<ProcessedEvent | null>(null);
  const [timeRange, setTimeRange] = useState<{ start: number; end: number } | null>(null);
  const [selectedSource, setSelectedSource] = useState<string>('');
  const [selectedComm, setSelectedComm] = useState<string>('');
  const [selectedPid, setSelectedPid] = useState<string>('');
  const [zoomLevel, setZoomLevel] = useState<number>(1);
  const [scrollOffset, setScrollOffset] = useState<number>(0);

  // Process events with additional metadata
  const processedEvents = useMemo(() => processEvents(events), [events]);

  // Filter events based on selected filters
  const filteredEvents = useMemo(() => {
    return filterEvents(processedEvents, {
      source: selectedSource,
      comm: selectedComm,
      pid: selectedPid
    });
  }, [processedEvents, selectedSource, selectedComm, selectedPid]);

  // Group filtered events by source
  const timelineGroups: TimelineGroupData[] = useMemo(() => {
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
  const zoomIn = useCallback(() => {
    setZoomLevel(prev => {
      const newZoom = Math.min(prev * 1.5, 10);
      // Adjust scroll offset to maintain center position when zooming
      const currentCenter = scrollOffset + (baseTimeSpan / prev) / 2;
      const newZoomedSpan = baseTimeSpan / newZoom;
      const newOffset = Math.max(0, Math.min(baseTimeSpan - newZoomedSpan, currentCenter - newZoomedSpan / 2));
      setScrollOffset(newOffset);
      return newZoom;
    });
  }, [scrollOffset, baseTimeSpan]);

  const zoomOut = useCallback(() => {
    setZoomLevel(prev => {
      const newZoom = Math.max(prev / 1.5, 0.1);
      if (newZoom === 1) {
        setScrollOffset(0);
      } else {
        // Adjust scroll offset to maintain center position when zooming
        const currentCenter = scrollOffset + (baseTimeSpan / prev) / 2;
        const newZoomedSpan = baseTimeSpan / newZoom;
        const newOffset = Math.max(0, Math.min(baseTimeSpan - newZoomedSpan, currentCenter - newZoomedSpan / 2));
        setScrollOffset(newOffset);
      }
      return newZoom;
    });
  }, [scrollOffset, baseTimeSpan]);

  const resetZoom = () => {
    setZoomLevel(1);
    setScrollOffset(0);
    setTimeRange(null);
  };

  // Scroll functions
  const scrollLeft = useCallback(() => {
    const zoomedSpan = baseTimeSpan / zoomLevel;
    const scrollStep = zoomedSpan * 0.1; // 10% of visible range
    setScrollOffset(prev => Math.max(0, prev - scrollStep));
  }, [baseTimeSpan, zoomLevel]);

  const scrollRight = useCallback(() => {
    const zoomedSpan = baseTimeSpan / zoomLevel;
    const scrollStep = zoomedSpan * 0.1; // 10% of visible range
    const maxOffset = baseTimeSpan - zoomedSpan;
    setScrollOffset(prev => Math.min(maxOffset, prev + scrollStep));
  }, [baseTimeSpan, zoomLevel]);

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
      const scrollStep = zoomedSpan * 0.1; // 10% of visible range for smoother scrolling
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
  }, [zoomLevel, scrollLeft, scrollRight, zoomIn, zoomOut]);

  return (
    <div className="bg-white rounded-lg shadow-md">
      {/* Timeline Header */}
      <div className="border-b border-gray-200 p-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Timeline View</h2>
          <div className="flex items-center gap-4">
            <ZoomControls
              zoomLevel={zoomLevel}
              onZoomIn={zoomIn}
              onZoomOut={zoomOut}
              onReset={resetZoom}
              onScrollLeft={scrollLeft}
              onScrollRight={scrollRight}
            />
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
        <EventFilters
          events={processedEvents}
          selectedSource={selectedSource}
          selectedComm={selectedComm}
          selectedPid={selectedPid}
          onSourceChange={setSelectedSource}
          onCommChange={setSelectedComm}
          onPidChange={setSelectedPid}
        />
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
            <TimelineAxis
              startTime={visibleTimeRange.start}
              endTime={visibleTimeRange.end}
              timeSpan={timeSpan}
            />

            {/* Scroll bar - Only show when zoomed */}
            <TimelineScrollBar
              zoomLevel={zoomLevel}
              scrollOffset={scrollOffset}
              baseTimeSpan={baseTimeSpan}
              onScrollChange={setScrollOffset}
            />

            {/* Scroll indicator/minimap - Only show when zoomed */}
            {zoomLevel > 1 && (
              <TimelineMinimap
                events={filteredEvents}
                fullTimeRange={fullTimeRange}
                visibleTimeRange={visibleTimeRange}
                baseTimeSpan={baseTimeSpan}
                timeSpan={timeSpan}
                scrollOffset={scrollOffset}
                onScrollChange={setScrollOffset}
              />
            )}

            {/* Timeline Groups */}
            {timelineGroups.map((group) => (
              <TimelineGroup
                key={group.source}
                source={group.source}
                events={group.events}
                color={group.color}
                getEventPosition={getEventPosition}
                onEventClick={setSelectedEvent}
              />
            ))}
          </div>
        )}
      </div>

      {/* Event Details Modal */}
      <EventModal
        event={selectedEvent}
        onClose={() => setSelectedEvent(null)}
        title="Timeline Event Details"
      />
    </div>
  );
}