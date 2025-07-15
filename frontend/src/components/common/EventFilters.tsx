'use client';

import { ProcessedEvent } from '@/types/event';
import { getUniqueValues } from '@/utils/eventProcessing';
import { useMemo } from 'react';

interface EventFiltersProps {
  events: ProcessedEvent[];
  selectedSource: string;
  selectedComm: string;
  selectedPid: string;
  searchTerm?: string;
  onSourceChange: (source: string) => void;
  onCommChange: (comm: string) => void;
  onPidChange: (pid: string) => void;
  onSearchChange?: (term: string) => void;
  showSearch?: boolean;
}

export function EventFilters({
  events,
  selectedSource,
  selectedComm,
  selectedPid,
  searchTerm = '',
  onSourceChange,
  onCommChange,
  onPidChange,
  onSearchChange,
  showSearch = false
}: EventFiltersProps) {
  const sources = useMemo(() => {
    const unique = new Set(events.map(event => event.source));
    return Array.from(unique).sort();
  }, [events]);

  const commValues = useMemo(() => {
    const unique = getUniqueValues(events, 'comm') as string[];
    return unique.sort();
  }, [events]);

  const pidValues = useMemo(() => {
    const unique = getUniqueValues(events, 'pid') as number[];
    return unique.map(pid => pid.toString()).sort((a, b) => parseInt(a) - parseInt(b));
  }, [events]);

  return (
    <div className="flex flex-col gap-4">
      {showSearch && onSearchChange && (
        <div className="flex-1">
          <input
            type="text"
            placeholder="Search events..."
            value={searchTerm}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
      )}
      
      <div className="flex flex-col sm:flex-row gap-4">
        <div className="flex-1">
          <select
            value={selectedSource}
            onChange={(e) => onSourceChange(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="">All Sources</option>
            {sources.map(source => (
              <option key={source} value={source}>
                {source} ({events.filter(e => e.source === source).length})
              </option>
            ))}
          </select>
        </div>
        
        <div className="flex-1">
          <select
            value={selectedComm}
            onChange={(e) => onCommChange(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="">All Processes</option>
            {commValues.map(comm => (
              <option key={comm} value={comm}>
                {comm} ({events.filter(e => e.comm === comm).length})
              </option>
            ))}
          </select>
        </div>
        
        <div className="flex-1">
          <select
            value={selectedPid}
            onChange={(e) => onPidChange(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="">All PIDs</option>
            {pidValues.map(pid => (
              <option key={pid} value={pid}>
                PID {pid} ({events.filter(e => e.pid.toString() === pid).length})
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  );
}