'use client';

import { useState } from 'react';
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/outline';
import React from 'react';

// Simplified interface - no longer needed but keeping for compatibility
export interface BlockField {
  key: string;
  label: string;
  content: any;
  type: 'text' | 'json' | 'code' | 'metadata' | 'raw';
  isExpandable?: boolean;
  isExpanded?: boolean;
  children?: BlockField[];
}

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

function renderFieldContent(field: BlockField, depth = 0): JSX.Element {
  const indentClass = depth > 0 ? `ml-${Math.min(depth * 4, 12)}` : '';
  
  if (field.type === 'json' && typeof field.content === 'object') {
    // Recursively render JSON objects as nested fields
    if (Array.isArray(field.content)) {
      return (
        <div className={indentClass}>
          <div className="text-xs text-gray-500 mb-1">[Array with {field.content.length} items]</div>
          {field.content.map((item, index) => (
            <div key={index} className="mb-1">
              <span className="text-xs text-gray-400">[{index}]</span>
              <div className="ml-4">
                {typeof item === 'object' ? 
                  renderFieldContent({
                    key: `${field.key}_${index}`,
                    label: `Item ${index}`,
                    content: item,
                    type: 'json'
                  }, depth + 1) :
                  <span className="text-sm font-mono">{String(item)}</span>
                }
              </div>
            </div>
          ))}
        </div>
      );
    } else if (field.content && typeof field.content === 'object') {
      return (
        <div className={indentClass}>
          {Object.entries(field.content).map(([key, value]) => (
            <div key={key} className="mb-1">
              <span className="text-xs font-medium text-gray-600">{key}:</span>
              <div className="ml-4">
                {typeof value === 'object' ? 
                  renderFieldContent({
                    key: `${field.key}_${key}`,
                    label: key,
                    content: value,
                    type: 'json'
                  }, depth + 1) :
                  <span className="text-sm font-mono">{String(value)}</span>
                }
              </div>
            </div>
          ))}
        </div>
      );
    }
  }
  
  // Handle other field types as text
  let displayContent = field.content;
  if (typeof field.content === 'object') {
    try {
      displayContent = JSON.stringify(field.content, null, 2);
    } catch {
      displayContent = String(field.content);
    }
  }
  
  const textClasses = {
    text: 'text-sm text-gray-800',
    code: 'text-sm font-mono bg-gray-50 p-2 rounded border',
    metadata: 'text-xs text-gray-600',
    raw: 'text-xs font-mono bg-gray-100 p-2 rounded border text-gray-700',
    json: 'text-sm font-mono bg-gray-50 p-2 rounded border'
  };
  
  const className = `${textClasses[field.type]} ${indentClass}`;
  
  if (field.type === 'code' || field.type === 'raw' || field.type === 'json') {
    return (
      <pre className={className}>
        {displayContent}
      </pre>
    );
  }
  
  return (
    <div className={className}>
      {displayContent}
    </div>
  );
}

function FieldRenderer({ field, depth = 0 }: { field: BlockField; depth?: number }) {
  const [isExpanded, setIsExpanded] = useState(field.isExpanded || false);
  const hasChildren = field.children && field.children.length > 0;
  const isExpandable = field.isExpandable || hasChildren || 
    (field.type === 'json' && typeof field.content === 'object');
  
  const shouldTruncate = !isExpanded && 
    typeof field.content === 'string' && 
    field.content.length > 300;
  
  const displayContent = shouldTruncate ? 
    field.content.substring(0, 300) + '...' : 
    field.content;
  
  return (
    <div className="mb-1">
      <div className="flex items-start space-x-2">
        {isExpandable && (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex-shrink-0 mt-1 text-gray-400 hover:text-gray-600"
          >
            {isExpanded ? (
              <ChevronDownIcon className="h-3 w-3" />
            ) : (
              <ChevronRightIcon className="h-3 w-3" />
            )}
          </button>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center space-x-2 mb-1">
            <span className="text-xs font-medium text-gray-700">{field.label}</span>
            {field.type !== 'text' && (
              <span className="text-xs text-gray-400 bg-gray-100 px-1 rounded">
                {field.type}
              </span>
            )}
          </div>
          
          {isExpanded ? (
            <div>
              {renderFieldContent({ ...field, content: field.content }, depth)}
              {hasChildren && (
                <div className="mt-2 ml-4">
                  {field.children!.map((child, index) => (
                    <FieldRenderer key={`${child.key}_${index}`} field={child} depth={depth + 1} />
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div>
              {renderFieldContent({ ...field, content: displayContent }, depth)}
              {shouldTruncate && (
                <button
                  onClick={() => setIsExpanded(true)}
                  className="text-xs text-blue-600 hover:text-blue-800 mt-1"
                >
                  Show more
                </button>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

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