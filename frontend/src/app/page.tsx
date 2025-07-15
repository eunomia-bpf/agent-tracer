'use client';

import { useState, useEffect } from 'react';
import { LogView } from '@/components/LogView';
import { TimelineView } from '@/components/TimelineView';
import { Event } from '@/types/event';

type ViewMode = 'log' | 'timeline';

export default function Home() {
  const [file, setFile] = useState<File | null>(null);
  const [logContent, setLogContent] = useState<string>('');
  const [events, setEvents] = useState<Event[]>([]);
  const [viewMode, setViewMode] = useState<ViewMode>('log');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [isParsed, setIsParsed] = useState(false);

  // Load data from localStorage on component mount
  useEffect(() => {
    const savedContent = localStorage.getItem('agent-tracer-log');
    const savedEvents = localStorage.getItem('agent-tracer-events');
    
    if (savedContent && savedEvents) {
      setLogContent(savedContent);
      setEvents(JSON.parse(savedEvents));
      setIsParsed(true);
    }
  }, []);

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const uploadedFile = event.target.files?.[0];
    if (uploadedFile) {
      setFile(uploadedFile);
      setError('');
      setIsParsed(false);
      
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as string;
        setLogContent(content);
      };
      reader.readAsText(uploadedFile);
    }
  };

  const handleTextPaste = (content: string) => {
    setLogContent(content);
    setIsParsed(false);
    setError('');
  };

  const parseLogContent = (content: string) => {
    if (!content.trim()) {
      setError('Empty log content');
      return;
    }

    setLoading(true);
    setError('');

    try {
      const lines = content.split('\n').filter(line => line.trim());
      const parsedEvents: Event[] = [];
      const errors: string[] = [];

      lines.forEach((line, index) => {
        try {
          const event = JSON.parse(line.trim()) as Event;
          
          // Validate event structure - auto-generate id if missing
          if (event.timestamp && event.source && event.data) {
            if (!event.id) {
              event.id = `${event.source}-${event.timestamp}-${index}`;
            }
            parsedEvents.push(event);
          } else {
            // Track validation errors
            const missing = [];
            if (!event.timestamp) missing.push('timestamp');
            if (!event.source) missing.push('source');
            if (!event.data) missing.push('data');
            errors.push(`Line ${index + 1}: Missing required fields: ${missing.join(', ')}`);
          }
        } catch (err) {
          errors.push(`Line ${index + 1}: Invalid JSON - ${err instanceof Error ? err.message : 'Unknown error'}`);
        }
      });

      if (parsedEvents.length === 0) {
        const errorMsg = errors.length > 0 
          ? `No valid events found. Errors:\n${errors.slice(0, 10).join('\n')}${errors.length > 10 ? '\n...and ' + (errors.length - 10) + ' more errors' : ''}`
          : 'No valid events found in the log file';
        setError(errorMsg);
        return;
      }

      // Show warnings for partial parsing
      if (errors.length > 0) {
        console.warn(`Parsed ${parsedEvents.length} events with ${errors.length} errors:`, errors);
      }

      // Sort events by timestamp
      const sortedEvents = parsedEvents.sort((a, b) => a.timestamp - b.timestamp);
      
      setEvents(sortedEvents);
      setIsParsed(true);
      
      // Save to localStorage
      localStorage.setItem('agent-tracer-log', content);
      localStorage.setItem('agent-tracer-events', JSON.stringify(sortedEvents));
      
    } catch (err) {
      setError(`Failed to parse log content: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const clearData = () => {
    setFile(null);
    setLogContent('');
    setEvents([]);
    setError('');
    setIsParsed(false);
    localStorage.removeItem('agent-tracer-log');
    localStorage.removeItem('agent-tracer-events');
  };

  const sampleLogPath = '/home/yunwei37/agent-tracer/collector/ssl.log';

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            Agent Tracer Log Analyzer
          </h1>
          <p className="text-gray-600">
            Upload and analyze eBPF agent trace logs with dual view modes
          </p>
        </div>

        {/* File Upload Section */}
        {!isParsed && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">
              Upload Log File
            </h2>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Choose log file
                </label>
                <input
                  type="file"
                  accept=".log,.txt,.json"
                  onChange={handleFileUpload}
                  className="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
                />
              </div>
              
              <div className="text-center text-gray-500">
                <span>or</span>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Paste log content
                </label>
                <textarea
                  placeholder={`Paste log content here (e.g., from ${sampleLogPath})`}
                  className="w-full h-32 p-3 border border-gray-300 rounded-md font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  onChange={(e) => handleTextPaste(e.target.value)}
                />
              </div>
            </div>

            {/* Parse Button */}
            {logContent && !loading && (
              <div className="mt-4 flex justify-center">
                <button
                  onClick={() => parseLogContent(logContent)}
                  className="px-6 py-3 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition-colors focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                >
                  Parse Log
                </button>
              </div>
            )}

            {error && (
              <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-md">
                <div className="text-red-700 text-sm">{error}</div>
              </div>
            )}

            {loading && (
              <div className="mt-4 flex items-center justify-center">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
                <span className="ml-2 text-gray-600">Parsing log content...</span>
              </div>
            )}
          </div>
        )}

        {/* Main Content */}
        {isParsed && events.length > 0 && (
          <div className="space-y-6">
            {/* Controls */}
            <div className="bg-white rounded-lg shadow-md p-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-4">
                  <div className="text-sm text-gray-600">
                    <span className="font-medium">{events.length}</span> events loaded
                  </div>
                  
                  {file && (
                    <div className="text-sm text-gray-600">
                      File: <span className="font-medium">{file.name}</span>
                    </div>
                  )}
                </div>
                
                <div className="flex items-center space-x-4">
                  {/* View Mode Toggle */}
                  <div className="flex rounded-lg border border-gray-200 p-1">
                    <button
                      onClick={() => setViewMode('log')}
                      className={`px-3 py-1 text-sm rounded-md transition-colors ${
                        viewMode === 'log'
                          ? 'bg-blue-600 text-white'
                          : 'text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      Log View
                    </button>
                    <button
                      onClick={() => setViewMode('timeline')}
                      className={`px-3 py-1 text-sm rounded-md transition-colors ${
                        viewMode === 'timeline'
                          ? 'bg-blue-600 text-white'
                          : 'text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      Timeline View
                    </button>
                  </div>
                  
                  <button
                    onClick={clearData}
                    className="px-4 py-2 text-sm text-red-600 hover:text-red-700 hover:bg-red-50 rounded-md transition-colors"
                  >
                    Clear Data
                  </button>
                </div>
              </div>
            </div>

            {/* View Content */}
            {viewMode === 'log' ? (
              <LogView events={events} />
            ) : (
              <TimelineView events={events} />
            )}
          </div>
        )}
      </div>
    </div>
  );
}