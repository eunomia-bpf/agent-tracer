'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/outline';
import React from 'react';

// Clean unified block interface

export interface UnifiedBlockData {
  id: string;
  type: 'prompt' | 'response' | 'ssl' | 'file' | 'process';
  timestamp: number;
  tags: string[];
  bgGradient: string;
  borderColor: string;
  iconColor: string;
  icon: React.ComponentType<{ className?: string }>;
  foldContent: string; // What to show when collapsed
  expandedContent: string; // What to show when expanded
}

interface UnifiedBlockProps {
  data: UnifiedBlockData;
  isExpanded: boolean;
  onToggle: () => void;
}

// Simplified unified block - no complex field rendering needed

export function UnifiedBlock({ data, isExpanded, onToggle }: UnifiedBlockProps) {
  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  const shouldShowExpandButton = data.expandedContent.length > 300;

  // Get gradient hover classes
  const getHoverGradient = (bgGradient: string) => {
    if (bgGradient.includes('blue')) return 'hover:from-blue-100 hover:via-purple-100 hover:to-pink-100';
    if (bgGradient.includes('cyan')) return 'hover:from-cyan-100 hover:via-sky-100 hover:to-blue-100';
    if (bgGradient.includes('green')) return 'hover:from-green-100 hover:via-emerald-100 hover:to-teal-100';
    if (bgGradient.includes('orange')) return 'hover:from-orange-100 hover:via-amber-100 hover:to-yellow-100';
    if (bgGradient.includes('purple')) return 'hover:from-purple-100 hover:via-violet-100 hover:to-indigo-100';
    return 'hover:bg-gray-100';
  };

  return (
    <div className="mb-1">
      <div
        className={`relative p-2 ${data.bgGradient} border-l-4 ${data.borderColor} rounded-lg cursor-pointer ${getHoverGradient(data.bgGradient)} transition-all duration-200 shadow-sm hover:shadow-md`}
        onClick={() => shouldShowExpandButton && onToggle()}
      >
        {/* Single line header */}
        <div className="flex items-center space-x-3">
          <div className="flex-shrink-0">
            <data.icon className={`h-4 w-4 ${data.iconColor}`} />
          </div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2 flex-1 min-w-0">
                {/* Tags */}
                {data.tags.map((tag, index) => {
                  if (index === 0) {
                    // First tag uses primary color scheme
                    const bgColor = data.iconColor.replace('text-', 'bg-').replace('-600', '-100').replace('-700', '-100');
                    const textColor = data.iconColor.replace('-600', '-800').replace('-700', '-800');
                    return (
                      <span key={tag} className={`px-2 py-1 text-xs font-bold rounded uppercase ${bgColor} ${textColor}`}>
                        {tag}
                      </span>
                    );
                  } else {
                    // Other tags use gray
                    return (
                      <span key={tag} className="px-2 py-1 bg-gray-100 text-gray-800 text-xs font-bold rounded uppercase">
                        {tag}
                      </span>
                    );
                  }
                })}
                
                {/* Content when not expanded */}
                {!isExpanded && (
                  <span className="text-sm text-gray-600 truncate">
                    {data.foldContent}
                  </span>
                )}
              </div>
              
              <div className="flex items-center space-x-2 flex-shrink-0">
                <span className="text-xs text-gray-500">
                  {formatTimestamp(data.timestamp)}
                </span>
                {shouldShowExpandButton && (
                  <div className="flex-shrink-0">
                    {isExpanded ? (
                      <ChevronDownIcon className={`h-4 w-4 ${data.iconColor}`} />
                    ) : (
                      <ChevronRightIcon className={`h-4 w-4 ${data.iconColor}`} />
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Expanded content */}
        {isExpanded && (
          <div className={`mt-2 pt-2 border-t ${data.borderColor.replace('border-', 'border-').replace('-400', '-200')}`}>
            <div className="bg-white/50 p-2 rounded border">
              <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed text-gray-800">
                {data.expandedContent}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}