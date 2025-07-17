'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon, CommandLineIcon } from '@heroicons/react/24/outline';
import { ParsedEvent } from '@/utils/eventParsers';

interface ProcessBlockProps {
  event: ParsedEvent;
  onToggle: (eventId: string) => void;
}

export function ProcessBlock({ event, onToggle }: ProcessBlockProps) {
  const { id, title, content, metadata, isExpanded } = event;
  
  const truncatedContent = content.length > 120 
    ? content.substring(0, 120) + '...' 
    : content;

  const shouldShowExpandButton = content.length > 120;

  // Extract process event details
  const eventType = metadata.event || 'process';
  const filename = metadata.filename;
  const pid = metadata.pid;
  const ppid = metadata.ppid;

  // Color scheme based on event type
  const getEventColor = (event: string) => {
    const lowerEvent = event.toLowerCase();
    if (lowerEvent.includes('exec')) return 'text-green-600';
    if (lowerEvent.includes('exit')) return 'text-red-600';
    if (lowerEvent.includes('fork')) return 'text-blue-600';
    return 'text-purple-600';
  };

  const getBadgeColor = (event: string) => {
    const lowerEvent = event.toLowerCase();
    if (lowerEvent.includes('exec')) return 'bg-green-100 text-green-800';
    if (lowerEvent.includes('exit')) return 'bg-red-100 text-red-800';
    if (lowerEvent.includes('fork')) return 'bg-blue-100 text-blue-800';
    return 'bg-purple-100 text-purple-800';
  };

  const iconColor = getEventColor(eventType);
  const badgeColor = getBadgeColor(eventType);

  return (
    <div className="mb-1">
      <div
        className="relative p-2 bg-gradient-to-r from-violet-50 via-purple-50 to-indigo-50 border-l-4 border-violet-400 rounded-lg cursor-pointer hover:from-violet-100 hover:via-purple-100 hover:to-indigo-100 transition-all duration-200 shadow-sm hover:shadow-md"
        onClick={() => shouldShowExpandButton && onToggle(id)}
      >
        {/* Compact header */}
        <div className="flex items-center space-x-3">
          <div className="flex-shrink-0">
            <CommandLineIcon className={`h-4 w-4 ${iconColor}`} />
          </div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className={`px-2 py-1 ${badgeColor} text-xs font-bold rounded uppercase`}>
                  {eventType}
                </span>
                {pid && (
                  <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded font-mono">
                    PID {pid}
                  </span>
                )}
                {filename && (
                  <span className="text-sm text-gray-700 font-mono truncate max-w-xs">
                    {filename}
                  </span>
                )}
              </div>
              
              <div className="flex items-center space-x-2">
                <span className="text-xs text-gray-500">
                  {new Date(event.timestamp).toLocaleTimeString()}
                </span>
                {shouldShowExpandButton && (
                  <div className="flex-shrink-0">
                    {isExpanded ? (
                      <ChevronDownIcon className={`h-4 w-4 ${iconColor}`} />
                    ) : (
                      <ChevronRightIcon className={`h-4 w-4 ${iconColor}`} />
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Expanded content */}
        {isExpanded && (
          <div className="mt-2 pt-2 border-t border-violet-200">
            <div className="bg-white/50 p-2 rounded border border-violet-200">
              <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed max-h-32 overflow-y-auto">
                {content}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}