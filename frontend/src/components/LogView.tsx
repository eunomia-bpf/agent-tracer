'use client';

import { useState, useMemo } from 'react';
import { Event, GroupedEvents, ProcessedEvent } from '@/types/event';

interface LogViewProps {
  events: Event[];
}

export function LogView({ events }: LogViewProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedSource, setSelectedSource] = useState<string>('');
  const [selectedComm, setSelectedComm] = useState<string>('');
  const [selectedPid, setSelectedPid] = useState<string>('');
  const [selectedEvent, setSelectedEvent] = useState<ProcessedEvent | null>(null);

  // Process events with additional metadata
  const processedEvents: ProcessedEvent[] = useMemo(() => {
    const sourceColors = [
      'bg-blue-100 text-blue-800',
      'bg-green-100 text-green-800',
      'bg-yellow-100 text-yellow-800',
      'bg-purple-100 text-purple-800',
      'bg-red-100 text-red-800',
      'bg-indigo-100 text-indigo-800',
      'bg-pink-100 text-pink-800',
      'bg-gray-100 text-gray-800'
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
  const groupedEvents: GroupedEvents = useMemo(() => {
    const grouped: GroupedEvents = {};
    processedEvents.forEach(event => {
      if (!grouped[event.source]) {
        grouped[event.source] = [];
      }
      grouped[event.source].push(event);
    });
    return grouped;
  }, [processedEvents]);

  // Get unique comm values
  const commValues = useMemo(() => {
    const unique = new Set(processedEvents.map(event => event.comm));
    return Array.from(unique).sort();
  }, [processedEvents]);

  // Get unique pid values
  const pidValues = useMemo(() => {
    const unique = new Set(processedEvents.map(event => event.pid.toString()));
    return Array.from(unique).sort((a, b) => parseInt(a) - parseInt(b));
  }, [processedEvents]);

  // Filter events based on search, source, comm, and pid
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

    if (searchTerm) {
      const term = searchTerm.toLowerCase();
      filtered = filtered.filter(event => 
        event.source.toLowerCase().includes(term) ||
        event.id.toLowerCase().includes(term) ||
        event.comm.toLowerCase().includes(term) ||
        event.pid.toString().includes(term) ||
        JSON.stringify(event.data).toLowerCase().includes(term)
      );
    }

    return filtered;
  }, [processedEvents, searchTerm, selectedSource, selectedComm, selectedPid]);

  const sources = Object.keys(groupedEvents);

  const formatEventSummary = (event: ProcessedEvent) => {
    return `${event.comm} (${event.pid})`;
  };

  return (
    <div className="bg-white rounded-lg shadow-md">
      {/* Filters */}
      <div className="border-b border-gray-200 p-4">
        <div className="flex flex-col gap-4">
          <div className="flex-1">
            <input
              type="text"
              placeholder="Search events..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
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
                    {source} ({groupedEvents[source].length})
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
      </div>

      {/* Events List */}
      <div className="max-h-96 overflow-y-auto">
        {filteredEvents.length === 0 ? (
          <div className="p-8 text-center text-gray-500">
            No events found matching the current filters.
          </div>
        ) : (
          <div className="divide-y divide-gray-200">
            {filteredEvents.map((event) => (
              <div
                key={event.id}
                className="p-4 hover:bg-gray-50 cursor-pointer"
                onClick={() => setSelectedEvent(event)}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center space-x-3 mb-1">
                      <span className="font-mono text-sm text-gray-500">
                        {event.formattedTime}
                      </span>
                      <span className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${event.sourceColor}`}>
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
        )}
      </div>

      {/* Event Details Modal */}
      {selectedEvent && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg max-w-4xl w-full max-h-[80vh] overflow-y-auto">
            <div className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold text-gray-900">
                  Event Details
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
                    <label className="block text-sm font-medium text-gray-700 mb-1">ID</label>
                    <div className="text-sm text-gray-900 font-mono">{selectedEvent.id}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Source</label>
                    <span className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${selectedEvent.sourceColor}`}>
                      {selectedEvent.source}
                    </span>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Process</label>
                    <div className="text-sm text-gray-900">{selectedEvent.comm}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">PID</label>
                    <div className="text-sm text-gray-900 font-mono">{selectedEvent.pid}</div>
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