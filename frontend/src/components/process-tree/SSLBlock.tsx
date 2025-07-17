'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon, LockClosedIcon } from '@heroicons/react/24/outline';
import { ParsedEvent } from '@/utils/eventParsers';

interface SSLBlockProps {
  event: ParsedEvent;
  onToggle: (eventId: string) => void;
}

export function SSLBlock({ event, onToggle }: SSLBlockProps) {
  const { id, title, content, metadata, isExpanded } = event;
  
  const truncatedContent = content.length > 150 
    ? content.substring(0, 150) + '...' 
    : content;

  const shouldShowExpandButton = content.length > 150;

  // Determine the type of SSL event
  const isRequest = metadata.message_type === 'request';
  const isResponse = metadata.message_type === 'response';
  const method = metadata.method || 'UNKNOWN';
  const statusCode = metadata.status_code;

  // Color scheme based on message type
  const colorClasses = isRequest 
    ? 'from-orange-50 via-amber-50 to-yellow-50 border-orange-400 hover:from-orange-100 hover:via-amber-100 hover:to-yellow-100'
    : isResponse
    ? 'from-purple-50 via-violet-50 to-indigo-50 border-purple-400 hover:from-purple-100 hover:via-violet-100 hover:to-indigo-100'
    : 'from-gray-50 via-slate-50 to-zinc-50 border-gray-400 hover:from-gray-100 hover:via-slate-100 hover:to-zinc-100';

  const iconColor = isRequest ? 'text-orange-600' : isResponse ? 'text-purple-600' : 'text-gray-600';
  const badgeColor = isRequest 
    ? 'bg-orange-100 text-orange-800' 
    : isResponse 
    ? 'bg-purple-100 text-purple-800' 
    : 'bg-gray-100 text-gray-800';

  return (
    <div className="mb-3">
      <div
        className={`relative p-4 bg-gradient-to-r ${colorClasses} border-l-4 rounded-lg cursor-pointer transition-all duration-200 shadow-sm hover:shadow-md`}
        onClick={() => shouldShowExpandButton && onToggle(id)}
      >
        {/* Header with icon and expand button */}
        <div className="flex items-start space-x-3">
          <div className="flex-shrink-0 mt-1">
            <LockClosedIcon className={`h-5 w-5 ${iconColor}`} />
          </div>
          
          <div className="flex-1 min-w-0">
            {/* Title and metadata row */}
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center space-x-2">
                <span className={`px-2 py-1 ${badgeColor} text-xs font-bold rounded uppercase`}>
                  SSL {metadata.message_type || 'DATA'}
                </span>
                {method && method !== 'UNKNOWN' && (
                  <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded font-mono">
                    {method}
                  </span>
                )}
                {statusCode && (
                  <span className={`px-2 py-1 text-xs rounded font-mono ${
                    statusCode >= 200 && statusCode < 300 
                      ? 'bg-green-100 text-green-700'
                      : statusCode >= 400
                      ? 'bg-red-100 text-red-700'
                      : 'bg-yellow-100 text-yellow-700'
                  }`}>
                    {statusCode}
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
            
            {/* URL/Path info */}
            {(metadata.host || metadata.path) && (
              <div className="text-sm text-gray-600 font-mono mb-2 break-all">
                ➜ {metadata.host || ''}{metadata.path || ''}
              </div>
            )}
            
            {/* Content */}
            <div className="text-sm text-gray-800">
              <div className="font-medium text-gray-900 mb-1">{title}</div>
              <div className="bg-white/50 p-3 rounded border border-gray-200">
                <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed max-h-40 overflow-y-auto">
                  {isExpanded ? content : truncatedContent}
                </pre>
              </div>
            </div>
            
            {/* Additional metadata */}
            <div className="mt-2 flex items-center space-x-3 text-xs text-gray-500">
              {metadata.content_length && (
                <span>Size: {metadata.content_length} bytes</span>
              )}
              {metadata.headers && Object.keys(metadata.headers).length > 0 && (
                <span>Headers: {Object.keys(metadata.headers).length}</span>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}