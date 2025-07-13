'use client';

import { useState } from 'react';

interface TimelineMetadata {
  timestamp: string;
  source_file: string;
  total_timeline_entries: number;
  total_requests: number;
  total_responses: number;
  sse_responses: number;
}

interface TimelineData {
  analysis_metadata: TimelineMetadata;
  simple_timeline: string[];
}

interface ParsedEvent {
  id: string;
  index: number;
  type: 'request' | 'response';
  method?: string;
  path?: string;
  host?: string;
  protocol?: string;
  status?: string;
  statusCode?: number;
  contentType?: string;
  raw: string;
  timestamp?: number;
}

export default function TimelinePage() {
  const [jsonInput, setJsonInput] = useState('');
  const [timelineData, setTimelineData] = useState<TimelineData | null>(null);
  const [parsedEvents, setParsedEvents] = useState<ParsedEvent[]>([]);
  const [selectedEvent, setSelectedEvent] = useState<ParsedEvent | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const parseHttpLine = (line: string, index: number): ParsedEvent => {
    const event: ParsedEvent = {
      id: `event-${index}`,
      index,
      type: 'request',
      raw: line
    };

    // Extract timestamp from URL parameters if present
    const timestampMatch = line.match(/[&?]t=(\d+)/);
    if (timestampMatch) {
      event.timestamp = parseInt(timestampMatch[1]);
    }

    // Check if it's a response
    if (line.startsWith('HTTP/')) {
      event.type = 'response';
      
      // Parse HTTP response
      const responseMatch = line.match(/^HTTP\/(\d+\.\d+)\s+(\d+)\s+([^]+?)(?:\s+content-type:\s*([^]+?))?(?:\s+host:\s*([^]+?))?$/i);
      if (responseMatch) {
        event.protocol = `HTTP/${responseMatch[1]}`;
        event.statusCode = parseInt(responseMatch[2]);
        event.status = responseMatch[3];
        event.contentType = responseMatch[4];
        event.host = responseMatch[5];
      }
    } else {
      // Parse HTTP request
      const requestMatch = line.match(/^([A-Z]+)\s+([^]+?)\s+HTTP\/(\d+\.\d+)(?:\s+host:\s*([^]+?))?$/i);
      if (requestMatch) {
        event.method = requestMatch[1];
        event.path = requestMatch[2];
        event.protocol = `HTTP/${requestMatch[3]}`;
        event.host = requestMatch[4];
      }
    }

    return event;
  };

  const handleParseTimeline = () => {
    if (!jsonInput.trim()) {
      setError('Please enter JSON data');
      return;
    }

    setLoading(true);
    setError('');

    try {
      const data: TimelineData = JSON.parse(jsonInput);
      
      if (!data.simple_timeline || !Array.isArray(data.simple_timeline)) {
        throw new Error('Invalid JSON format: expected simple_timeline array');
      }

      const events = data.simple_timeline.map(parseHttpLine);
      
      setTimelineData(data);
      setParsedEvents(events);
      setError('');
    } catch (err) {
      setError(`Failed to parse JSON: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const getEventColor = (event: ParsedEvent) => {
    if (event.type === 'response') {
      if (event.statusCode) {
        if (event.statusCode >= 200 && event.statusCode < 300) return 'bg-green-500';
        if (event.statusCode >= 300 && event.statusCode < 400) return 'bg-blue-500';
        if (event.statusCode >= 400 && event.statusCode < 500) return 'bg-yellow-500';
        if (event.statusCode >= 500) return 'bg-red-500';
      }
      return 'bg-gray-500';
    }

    // Request colors based on method
    switch (event.method) {
      case 'GET': return 'bg-green-400';
      case 'POST': return 'bg-blue-400';
      case 'PUT': return 'bg-yellow-400';
      case 'DELETE': return 'bg-red-400';
      case 'HEAD': return 'bg-purple-400';
      default: return 'bg-gray-400';
    }
  };

  const getEventTextColor = (event: ParsedEvent) => {
    if (event.type === 'response') {
      if (event.statusCode) {
        if (event.statusCode >= 200 && event.statusCode < 300) return 'text-green-700';
        if (event.statusCode >= 300 && event.statusCode < 400) return 'text-blue-700';
        if (event.statusCode >= 400 && event.statusCode < 500) return 'text-yellow-700';
        if (event.statusCode >= 500) return 'text-red-700';
      }
      return 'text-gray-700';
    }

    switch (event.method) {
      case 'GET': return 'text-green-700';
      case 'POST': return 'text-blue-700';
      case 'PUT': return 'text-yellow-700';
      case 'DELETE': return 'text-red-700';
      case 'HEAD': return 'text-purple-700';
      default: return 'text-gray-700';
    }
  };

  const getEventLabel = (event: ParsedEvent) => {
    if (event.type === 'response') {
      return `${event.statusCode} ${event.status}`;
    }
    
    const pathShort = event.path && event.path.length > 40 
      ? event.path.substring(0, 40) + '...' 
      : event.path;
    
    return `${event.method} ${pathShort}`;
  };

  const getEventDescription = (event: ParsedEvent) => {
    if (event.type === 'response') {
      return `${event.protocol || 'HTTP/1.1'} ${event.statusCode} ${event.status}${event.contentType ? ` • ${event.contentType}` : ''}`;
    }
    
    return `${event.method} ${event.path} ${event.protocol || 'HTTP/1.1'}${event.host ? ` • ${event.host}` : ''}`;
  };

  const groupEventsByHost = (events: ParsedEvent[]) => {
    const grouped: { [host: string]: ParsedEvent[] } = {};
    
    events.forEach(event => {
      const host = event.host || 'unknown-host';
      if (!grouped[host]) {
        grouped[host] = [];
      }
      grouped[host].push(event);
    });

    return grouped;
  };

  const sampleJson = `{
  "analysis_metadata": {
    "timestamp": "2025-07-13T09:06:11.589259",
    "source_file": "collector/ssl.log",
    "total_timeline_entries": 1043,
    "total_requests": 525,
    "total_responses": 518,
    "sse_responses": 62
  },
  "simple_timeline": [
    "POST /v1/rgstr?k=client-RRNS7R65EAtReO5XA4xDC3eU6ZdJQi6lLEP6b5j32Me&st=javascript-client&sv=3.12.1&t=1752375607735&sid=2d1e2d68-5883-4437-9dc4-18e6787f79d5&ec=1 HTTP/1.1 host: statsig.anthropic.com",
    "POST /v1/messages?beta=true HTTP/1.1 host: api.anthropic.com",
    "HTTP/1.1 202 Accepted content-type: application/json",
    "HTTP/1.1 200 OK content-type: text/event-stream; charset=utf-8",
    "GET / HTTP/1.1 host: www.google.com",
    "HTTP/1.1 200 OK content-type: text/html; charset=ISO-8859-1"
  ]
}`;

  return (
    <div className="min-h-screen bg-gray-50 py-6">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            SSL Timeline Visualizer
          </h1>
          <p className="text-gray-600">
            Visualize eBPF SSL/HTTP traffic traces in a timeline format
          </p>
        </div>

        {/* JSON Input Section */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-semibold text-gray-900">JSON Input</h2>
            <button
              onClick={() => setJsonInput(sampleJson)}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              Load Sample
            </button>
          </div>
          
          <textarea
            value={jsonInput}
            onChange={(e) => setJsonInput(e.target.value)}
            placeholder="Paste your SSL timeline JSON here..."
            className="w-full h-40 p-3 border border-gray-300 rounded-md font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
          
          <div className="flex justify-between items-center mt-4">
            <button
              onClick={handleParseTimeline}
              disabled={loading}
              className="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed flex items-center"
            >
              {loading ? (
                <>
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
                  Parsing...
                </>
              ) : (
                'Parse Timeline'
              )}
            </button>
            
            {error && (
              <div className="text-red-600 text-sm bg-red-50 px-3 py-2 rounded-md">
                {error}
              </div>
            )}
          </div>
        </div>

        {/* Timeline Visualization */}
        {timelineData && parsedEvents.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6">
            {/* Metadata */}
            <div className="mb-6 p-4 bg-gray-50 rounded-lg">
              <h3 className="font-semibold text-gray-900 mb-2">Timeline Metadata</h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                <div>
                  <span className="font-medium text-gray-700">Total Entries:</span>
                  <span className="ml-2 text-gray-600">{timelineData.analysis_metadata.total_timeline_entries}</span>
                </div>
                <div>
                  <span className="font-medium text-gray-700">Requests:</span>
                  <span className="ml-2 text-gray-600">{timelineData.analysis_metadata.total_requests}</span>
                </div>
                <div>
                  <span className="font-medium text-gray-700">Responses:</span>
                  <span className="ml-2 text-gray-600">{timelineData.analysis_metadata.total_responses}</span>
                </div>
                <div>
                  <span className="font-medium text-gray-700">SSE Responses:</span>
                  <span className="ml-2 text-gray-600">{timelineData.analysis_metadata.sse_responses}</span>
                </div>
              </div>
            </div>

            {/* Legend */}
            <div className="mb-6 p-4 bg-gray-50 rounded-lg">
              <h3 className="font-semibold text-gray-900 mb-3">Legend</h3>
              <div className="flex flex-wrap gap-4 text-sm">
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-green-400 rounded mr-2"></div>
                  <span>GET Request</span>
                </div>
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-blue-400 rounded mr-2"></div>
                  <span>POST Request</span>
                </div>
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-purple-400 rounded mr-2"></div>
                  <span>HEAD Request</span>
                </div>
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-green-500 rounded mr-2"></div>
                  <span>2xx Response</span>
                </div>
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-yellow-500 rounded mr-2"></div>
                  <span>4xx Response</span>
                </div>
                <div className="flex items-center">
                  <div className="w-4 h-4 bg-red-500 rounded mr-2"></div>
                  <span>5xx Response</span>
                </div>
              </div>
            </div>

            {/* Timeline */}
            <div className="overflow-x-auto">
              <div className="min-w-[800px]">
                <h3 className="font-semibold text-gray-900 mb-4">HTTP Traffic Timeline</h3>
                
                {/* Events by host */}
                {Object.entries(groupEventsByHost(parsedEvents)).map(([host, events]) => (
                  <div key={host} className="mb-6">
                    <h4 className="font-medium text-gray-700 mb-3 sticky left-0 bg-gray-100 px-3 py-1 rounded">
                      {host}
                    </h4>
                    
                    <div className="space-y-1">
                      {events.map((event, index) => (
                        <div
                          key={event.id}
                          className="flex items-center cursor-pointer hover:bg-gray-50 rounded p-2"
                          onClick={() => setSelectedEvent(event)}
                        >
                          {/* Sequence number */}
                          <div className="w-12 text-xs text-gray-500 font-mono">
                            {event.index + 1}
                          </div>
                          
                          {/* Event type indicator */}
                          <div className={`w-3 h-3 rounded-full ${getEventColor(event)} mr-3 flex-shrink-0`}></div>
                          
                          {/* Event bar */}
                          <div className="flex-1 min-w-0">
                            <div className={`${getEventColor(event)} bg-opacity-20 rounded px-3 py-1 border-l-4 ${getEventColor(event).replace('bg-', 'border-')}`}>
                              <div className="flex items-center justify-between">
                                <div className="flex items-center space-x-2 min-w-0">
                                  <span className={`text-xs font-medium px-2 py-1 rounded ${getEventTextColor(event)} bg-white bg-opacity-80`}>
                                    {event.type.toUpperCase()}
                                  </span>
                                  <span className="font-medium text-gray-900 truncate">
                                    {getEventLabel(event)}
                                  </span>
                                </div>
                                {event.timestamp && (
                                  <span className="text-xs text-gray-500 font-mono">
                                    {new Date(event.timestamp).toLocaleTimeString()}
                                  </span>
                                )}
                              </div>
                              <div className="text-xs text-gray-600 mt-1 truncate">
                                {getEventDescription(event)}
                              </div>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Event Details Modal */}
        {selectedEvent && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
            <div className="bg-white rounded-lg max-w-4xl w-full max-h-96 overflow-y-auto">
              <div className="p-6">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-xl font-bold text-gray-900">
                    Event Details #{selectedEvent.index + 1}
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
                  <div>
                    <span className="font-medium text-gray-700">Type:</span>
                    <span className={`ml-2 px-2 py-1 rounded text-xs font-medium ${getEventTextColor(selectedEvent)} bg-opacity-20`}>
                      {selectedEvent.type.toUpperCase()}
                    </span>
                  </div>
                  
                  {selectedEvent.method && (
                    <div>
                      <span className="font-medium text-gray-700">Method:</span>
                      <span className="ml-2 text-gray-600">{selectedEvent.method}</span>
                    </div>
                  )}
                  
                  {selectedEvent.path && (
                    <div>
                      <span className="font-medium text-gray-700">Path:</span>
                      <span className="ml-2 text-gray-600 font-mono text-sm break-all">{selectedEvent.path}</span>
                    </div>
                  )}
                  
                  {selectedEvent.statusCode && (
                    <div>
                      <span className="font-medium text-gray-700">Status:</span>
                      <span className="ml-2 text-gray-600">{selectedEvent.statusCode} {selectedEvent.status}</span>
                    </div>
                  )}
                  
                  {selectedEvent.host && (
                    <div>
                      <span className="font-medium text-gray-700">Host:</span>
                      <span className="ml-2 text-gray-600">{selectedEvent.host}</span>
                    </div>
                  )}
                  
                  {selectedEvent.contentType && (
                    <div>
                      <span className="font-medium text-gray-700">Content Type:</span>
                      <span className="ml-2 text-gray-600">{selectedEvent.contentType}</span>
                    </div>
                  )}
                  
                  {selectedEvent.timestamp && (
                    <div>
                      <span className="font-medium text-gray-700">Timestamp:</span>
                      <span className="ml-2 text-gray-600">{new Date(selectedEvent.timestamp).toLocaleString()}</span>
                    </div>
                  )}
                  
                  <div>
                    <span className="font-medium text-gray-700">Raw Data:</span>
                    <div className="mt-2 bg-gray-50 rounded-md p-3">
                      <pre className="text-sm text-gray-800 font-mono whitespace-pre-wrap break-all">
                        {selectedEvent.raw}
                      </pre>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
} 