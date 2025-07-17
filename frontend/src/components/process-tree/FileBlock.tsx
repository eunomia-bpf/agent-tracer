'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon, DocumentIcon } from '@heroicons/react/24/outline';
import { ParsedEvent } from '@/utils/eventParsers';

interface FileBlockProps {
  event: ParsedEvent;
  onToggle: (eventId: string) => void;
}

export function FileBlock({ event, onToggle }: FileBlockProps) {
  const { id, title, content, metadata, isExpanded } = event;
  
  const truncatedContent = content.length > 120 
    ? content.substring(0, 120) + '...' 
    : content;

  const shouldShowExpandButton = content.length > 120;

  // Extract file operation details
  const operation = metadata.operation || 'file operation';
  const path = metadata.path || 'unknown';
  const size = metadata.size;
  const fd = metadata.fd;

  // Color scheme based on operation type
  const getOperationColor = (op: string) => {
    const lowerOp = op.toLowerCase();
    if (lowerOp.includes('read')) return 'text-blue-600';
    if (lowerOp.includes('write')) return 'text-green-600';
    if (lowerOp.includes('open')) return 'text-purple-600';
    if (lowerOp.includes('close')) return 'text-gray-600';
    if (lowerOp.includes('delete') || lowerOp.includes('unlink')) return 'text-red-600';
    return 'text-indigo-600';
  };

  const getBadgeColor = (op: string) => {
    const lowerOp = op.toLowerCase();
    if (lowerOp.includes('read')) return 'bg-blue-100 text-blue-800';
    if (lowerOp.includes('write')) return 'bg-green-100 text-green-800';
    if (lowerOp.includes('open')) return 'bg-purple-100 text-purple-800';
    if (lowerOp.includes('close')) return 'bg-gray-100 text-gray-800';
    if (lowerOp.includes('delete') || lowerOp.includes('unlink')) return 'bg-red-100 text-red-800';
    return 'bg-indigo-100 text-indigo-800';
  };

  const iconColor = getOperationColor(operation);
  const badgeColor = getBadgeColor(operation);

  return (
    <div className="mb-1">
      <div
        className="relative p-2 bg-gradient-to-r from-cyan-50 via-sky-50 to-blue-50 border-l-4 border-cyan-400 rounded-lg cursor-pointer hover:from-cyan-100 hover:via-sky-100 hover:to-blue-100 transition-all duration-200 shadow-sm hover:shadow-md"
        onClick={() => shouldShowExpandButton && onToggle(id)}
      >
        {/* Header with icon and expand button */}
        <div className="flex items-start space-x-3">
          <div className="flex-shrink-0">
            <DocumentIcon className={`h-4 w-4 ${iconColor}`} />
          </div>
          
          <div className="flex-1 min-w-0">
            {/* Compact header */}
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className={`px-2 py-1 ${badgeColor} text-xs font-bold rounded uppercase`}>
                  {operation}
                </span>
                {fd !== undefined && (
                  <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded font-mono">
                    FD {fd}
                  </span>
                )}
                {size !== undefined && (
                  <span className="px-2 py-1 bg-yellow-100 text-yellow-700 text-xs rounded font-mono">
                    {formatFileSize(size)}
                  </span>
                )}
                {path && path !== 'unknown' && (
                  <span className="text-sm text-gray-700 font-mono truncate max-w-xs">
                    {path}
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
          <div className="mt-2 pt-2 border-t border-cyan-200">
            {/* File path */}
            {path && path !== 'unknown' && (
              <div className="text-sm text-gray-700 font-mono mb-2 break-all bg-white/50 px-2 py-1 rounded border">
                📁 {path}
              </div>
            )}
            
            {/* Content */}
            <div className="text-sm text-gray-800">
              <div className="font-medium text-gray-900 mb-1">{title}</div>
              {content.trim() && content !== '{}' && (
                <div className="bg-white/50 p-2 rounded border border-cyan-200">
                  <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed max-h-32 overflow-y-auto">
                    {content}
                  </pre>
                </div>
              )}
            </div>
            
            {/* Additional metadata */}
            {metadata.permissions && (
              <div className="mt-2 text-xs text-gray-500">
                <span>Permissions: {metadata.permissions}</span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// Helper function to format file sizes
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}