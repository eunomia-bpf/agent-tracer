'use client';

import { useState, useEffect } from 'react';
import { LogView } from '@/components/LogView';
import { TimelineView } from '@/components/TimelineView';
import { ProcessTreeView } from '@/components/ProcessTreeView';
import { UploadPanel } from '@/components/UploadPanel';
import { Event } from '@/types/event';

type ViewMode = 'log' | 'timeline' | 'process-tree';

export default function Home() {
  const [file, setFile] = useState<File | null>(null);
  const [logContent, setLogContent] = useState<string>('');
  const [events, setEvents] = useState<Event[]>([]);
  const [viewMode, setViewMode] = useState<ViewMode>('process-tree');
  const [loading, setLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string>('');
  const [isParsed, setIsParsed] = useState(false);
  const [showUploadPanel, setShowUploadPanel] = useState(false);

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
      setShowUploadPanel(false); // Hide upload panel after successful parse
      
      // Save to localStorage
      localStorage.setItem('agent-tracer-log', content);
      localStorage.setItem('agent-tracer-events', JSON.stringify(sortedEvents));
      
    } catch (err) {
      setError(`Failed to parse log content: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const syncData = async () => {
    setSyncing(true);
    setError('');

    try {
      const response = await fetch('/api/events');
      
      if (!response.ok) {
        throw new Error(`Failed to fetch data: ${response.status} ${response.statusText}`);
      }

      const content = await response.text();
      
      if (!content.trim()) {
        setError('No data received from server');
        return;
      }

      setLogContent(content);
      parseLogContent(content);
      
    } catch (err) {
      setError(`Failed to sync data: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setSyncing(false);
    }
  };

  // Load data from localStorage on component mount
  useEffect(() => {
    const savedContent = localStorage.getItem('agent-tracer-log');
    const savedEvents = localStorage.getItem('agent-tracer-events');
    
    if (savedContent && savedEvents) {
      setLogContent(savedContent);
      setEvents(JSON.parse(savedEvents));
      setIsParsed(true);
    }
    // Auto-sync disabled - user must manually sync data
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

  const clearData = () => {
    setFile(null);
    setLogContent('');
    setEvents([]);
    setError('');
    setIsParsed(false);
    localStorage.removeItem('agent-tracer-log');
    localStorage.removeItem('agent-tracer-events');
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            AgentSight Analyzer
          </h1>
          <p className="text-gray-600">
            Upload and analyze eBPF agent trace logs with dual view modes
          </p>
        </div>

        {/* Upload Panel */}
        {showUploadPanel && (
          <UploadPanel
            logContent={logContent}
            loading={loading}
            error={error}
            onFileUpload={handleFileUpload}
            onTextPaste={handleTextPaste}
            onParseLog={() => parseLogContent(logContent)}
          />
        )}

        {/* Main Content - Always show */}
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
                
                {syncing && (
                  <div className="flex items-center text-sm text-blue-600">
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2"></div>
                    Syncing...
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
                  <button
                    onClick={() => setViewMode('process-tree')}
                    className={`px-3 py-1 text-sm rounded-md transition-colors ${
                      viewMode === 'process-tree'
                        ? 'bg-blue-600 text-white'
                        : 'text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    Process Tree
                  </button>
                </div>
                
                {/* Action Buttons */}
                <button
                  onClick={() => setShowUploadPanel(!showUploadPanel)}
                  className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-md transition-colors border border-gray-300"
                >
                  {showUploadPanel ? 'Hide' : 'Upload'} Log
                </button>
                
                <button
                  onClick={syncData}
                  disabled={syncing}
                  className="px-4 py-2 text-sm text-blue-600 hover:text-blue-700 hover:bg-blue-50 rounded-md transition-colors border border-blue-300 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Sync Data
                </button>
                
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
          {events.length > 0 ? (
            viewMode === 'log' ? (
              <LogView events={events} />
            ) : viewMode === 'timeline' ? (
              <TimelineView events={events} />
            ) : (
              <ProcessTreeView events={events} />
            )
          ) : (
            <div className="bg-white rounded-lg shadow-md p-12 text-center">
              <div className="text-gray-500">
                {syncing ? (
                  <div className="flex flex-col items-center">
                    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mb-4"></div>
                    <p className="text-lg">Loading events from server...</p>
                  </div>
                ) : (
                  <>
                    <p className="text-lg mb-4">No events loaded</p>
                    <div className="space-x-4">
                      <button
                        onClick={syncData}
                        className="px-6 py-3 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition-colors"
                      >
                        Sync Data from Server
                      </button>
                      <button
                        onClick={() => setShowUploadPanel(true)}
                        className="px-6 py-3 bg-gray-600 text-white font-semibold rounded-lg hover:bg-gray-700 transition-colors"
                      >
                        Upload Log File
                      </button>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}