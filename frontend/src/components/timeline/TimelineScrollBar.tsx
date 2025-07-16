'use client';

import { useCallback } from 'react';

interface TimelineScrollBarProps {
  zoomLevel: number;
  scrollOffset: number;
  baseTimeSpan: number;
  onScrollChange: (offset: number) => void;
}

export function TimelineScrollBar({
  zoomLevel,
  scrollOffset,
  baseTimeSpan,
  onScrollChange
}: TimelineScrollBarProps) {
  const zoomedSpan = baseTimeSpan / zoomLevel;
  const maxOffset = baseTimeSpan - zoomedSpan;
  const scrollPercentage = maxOffset > 0 ? (scrollOffset / maxOffset) * 100 : 0;
  const visiblePercentage = (zoomedSpan / baseTimeSpan) * 100;

  const handleScrollBarClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const clickPosition = (e.clientX - rect.left) / rect.width;
    const newOffset = clickPosition * maxOffset;
    onScrollChange(Math.max(0, Math.min(maxOffset, newOffset)));
  }, [maxOffset, onScrollChange]);

  const handleThumbDrag = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const startX = e.clientX;
    const startOffset = scrollOffset;
    const scrollBarWidth = e.currentTarget.parentElement?.clientWidth || 0;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      const deltaPercentage = deltaX / scrollBarWidth;
      const deltaOffset = deltaPercentage * maxOffset;
      const newOffset = Math.max(0, Math.min(maxOffset, startOffset + deltaOffset));
      onScrollChange(newOffset);
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [scrollOffset, maxOffset, onScrollChange]);

  if (zoomLevel <= 1) return null;

  return (
    <div className="mt-2 mb-4">
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs text-gray-600">Scroll Position</span>
        <span className="text-xs text-gray-500">
          {Math.round(scrollPercentage)}% of timeline
        </span>
      </div>
      <div 
        className="relative h-3 bg-gray-200 rounded-sm cursor-pointer"
        onClick={handleScrollBarClick}
      >
        {/* Scroll thumb */}
        <div
          className="absolute top-0 h-full bg-blue-500 rounded-sm cursor-grab active:cursor-grabbing hover:bg-blue-600 transition-colors"
          style={{
            left: `${scrollPercentage}%`,
            width: `${Math.max(visiblePercentage, 5)}%` // Minimum 5% width for usability
          }}
          onMouseDown={handleThumbDrag}
        />
        
        {/* Scroll track indicators */}
        <div className="absolute inset-0 flex">
          {Array.from({ length: 11 }, (_, i) => (
            <div
              key={i}
              className="border-l border-gray-300 opacity-30"
              style={{ left: `${i * 10}%` }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}