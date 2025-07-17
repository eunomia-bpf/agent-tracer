'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon, ChatBubbleLeftEllipsisIcon } from '@heroicons/react/24/outline';
import { ParsedEvent } from '@/utils/eventParsers';

interface ResponseBlockProps {
  event: ParsedEvent;
  onToggle: (eventId: string) => void;
}

export function ResponseBlock({ event, onToggle }: ResponseBlockProps) {
  const { id, title, content, metadata, isExpanded } = event;
  
  const truncatedContent = content.length > 200 
    ? content.substring(0, 200) + '...' 
    : content;

  const shouldShowExpandButton = content.length > 200;

  // Extract streaming info
  const isStreaming = metadata.event_count > 0;
  const duration = metadata.duration_ns ? `${(metadata.duration_ns / 1000000).toFixed(1)}ms` : null;

  return (
    <div className="mb-1">
      <div
        className="relative p-2 bg-gradient-to-r from-green-50 via-emerald-50 to-teal-50 border-l-4 border-green-400 rounded-lg cursor-pointer hover:from-green-100 hover:via-emerald-100 hover:to-teal-100 transition-all duration-200 shadow-sm hover:shadow-md"
        onClick={() => shouldShowExpandButton && onToggle(id)}
      >
        {/* Header with icon and expand button */}
        <div className="flex items-start space-x-3">
          <div className="flex-shrink-0">
            <ChatBubbleLeftEllipsisIcon className="h-4 w-4 text-green-600" />
          </div>
          
          <div className="flex-1 min-w-0">
            {/* Compact header */}
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className="px-2 py-1 bg-green-100 text-green-800 text-xs font-bold rounded uppercase">
                  AI RESPONSE
                </span>
                {isStreaming && (
                  <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded font-mono">
                    STREAMING
                  </span>
                )}
                {metadata.model && (
                  <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded font-mono">
                    {metadata.model}
                  </span>
                )}
                {duration && (
                  <span className="px-2 py-1 bg-yellow-100 text-yellow-700 text-xs rounded font-mono">
                    {duration}
                  </span>
                )}
                {!isExpanded && content.trim() && (
                  <span className="text-sm text-gray-600 truncate max-w-xs">
                    {truncatedContent}
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
                      <ChevronDownIcon className="h-4 w-4 text-green-600" />
                    ) : (
                      <ChevronRightIcon className="h-4 w-4 text-green-600" />
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Expanded content */}
        {isExpanded && (
          <div className="mt-2 pt-2 border-t border-green-200">
            {/* Message ID if available */}
            {metadata.message_id && (
              <div className="text-xs text-gray-500 font-mono mb-2">
                ID: {metadata.message_id}
              </div>
            )}
            
            {/* Content */}
            <div className="text-sm text-gray-800">
              <div className="font-medium text-gray-900 mb-1">{title}</div>
              <div className="bg-white/50 p-2 rounded border border-green-200">
                {content.trim() ? (
                  <div className="whitespace-pre-wrap font-mono text-xs leading-relaxed">
                    {content}
                  </div>
                ) : (
                  <div className="text-gray-500 italic text-xs">
                    Response received (no text content)
                  </div>
                )}
              </div>
            </div>
            
            {/* Usage statistics */}
            {metadata.usage && (
              <div className="mt-2 flex items-center space-x-4 text-xs text-gray-500">
                {metadata.usage.input_tokens && (
                  <span>Input: {metadata.usage.input_tokens} tokens</span>
                )}
                {metadata.usage.output_tokens && (
                  <span>Output: {metadata.usage.output_tokens} tokens</span>
                )}
                {metadata.event_count && (
                  <span>Events: {metadata.event_count}</span>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}