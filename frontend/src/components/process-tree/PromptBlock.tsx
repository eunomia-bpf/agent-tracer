'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon, SparklesIcon } from '@heroicons/react/24/outline';
import { ParsedEvent } from '@/utils/eventParsers';

interface PromptBlockProps {
  event: ParsedEvent;
  onToggle: (eventId: string) => void;
}

export function PromptBlock({ event, onToggle }: PromptBlockProps) {
  const { id, title, content, metadata, isExpanded } = event;
  
  const truncatedContent = content.length > 150 
    ? content.substring(0, 150) + '...' 
    : content;

  const shouldShowExpandButton = content.length > 150;

  return (
    <div className="mb-1">
      <div
        className="relative p-2 bg-gradient-to-r from-blue-50 via-purple-50 to-pink-50 border-l-4 border-blue-400 rounded-lg cursor-pointer hover:from-blue-100 hover:via-purple-100 hover:to-pink-100 transition-all duration-200 shadow-sm hover:shadow-md"
        onClick={() => shouldShowExpandButton && onToggle(id)}
      >
        {/* Header with icon and expand button */}
        <div className="flex items-start space-x-3">
          <div className="flex-shrink-0">
            <SparklesIcon className="h-4 w-4 text-blue-600" />
          </div>
          
          <div className="flex-1 min-w-0">
            {/* Compact header */}
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className="px-2 py-1 bg-blue-100 text-blue-800 text-xs font-bold rounded uppercase">
                  AI PROMPT
                </span>
                {metadata.model && (
                  <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded font-mono">
                    {metadata.model}
                  </span>
                )}
                {metadata.method && (
                  <span className="px-2 py-1 bg-green-100 text-green-700 text-xs rounded font-mono">
                    {metadata.method}
                  </span>
                )}
                {!isExpanded && content && (
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
                      <ChevronDownIcon className="h-4 w-4 text-blue-600" />
                    ) : (
                      <ChevronRightIcon className="h-4 w-4 text-blue-600" />
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Expanded content */}
        {isExpanded && (
          <div className="mt-2 pt-2 border-t border-blue-200">
            {/* URL if available */}
            {metadata.url && (
              <div className="text-sm text-blue-600 font-mono mb-2 break-all">
                ➜ {metadata.url}
              </div>
            )}
            
            {/* Content */}
            <div className="text-sm text-gray-800">
              <div className="font-medium text-gray-900 mb-1">{title}</div>
              <div className="bg-white/50 p-2 rounded border border-blue-200">
                <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed">
                  {content}
                </pre>
              </div>
            </div>
            
            {/* Additional metadata */}
            {(metadata.temperature || metadata.max_tokens) && (
              <div className="mt-2 flex items-center space-x-3 text-xs text-gray-500">
                {metadata.temperature && (
                  <span>temp: {metadata.temperature}</span>
                )}
                {metadata.max_tokens && (
                  <span>max_tokens: {metadata.max_tokens}</span>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}