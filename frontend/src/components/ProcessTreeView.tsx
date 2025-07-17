'use client';

import { useState, useMemo } from 'react';
import { Event } from '@/types/event';
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/outline';

interface ProcessTreeViewProps {
  events: Event[];
}

interface ProcessNode {
  pid: number;
  comm: string;
  ppid?: number;
  children: ProcessNode[];
  prompts: PromptEvent[];
  isExpanded: boolean;
}

interface PromptEvent {
  id: string;
  timestamp: number;
  type: 'SSL' | 'SSE';
  content: string;
  url?: string;
  method?: string;
}

export function ProcessTreeView({ events }: ProcessTreeViewProps) {
  const [expandedProcesses, setExpandedProcesses] = useState<Set<number>>(new Set());
  const [expandedPrompts, setExpandedPrompts] = useState<Set<string>>(new Set());

  // Build process hierarchy from events
  const processTree = useMemo(() => {
    const processMap = new Map<number, ProcessNode>();
    const promptsByPid = new Map<number, PromptEvent[]>();

    // First pass: collect all processes and prompts
    events.forEach(event => {
      const { pid, comm, source, data } = event;

      // Initialize process if not exists
      if (!processMap.has(pid)) {
        processMap.set(pid, {
          pid,
          comm: comm || 'unknown',
          children: [],
          prompts: [],
          isExpanded: expandedProcesses.has(pid)
        });
      }

      // Collect prompts (SSL/SSE events)
      if (source === 'SSL' || source === 'SSE' || source.toLowerCase().includes('ssl')) {
        const prompt: PromptEvent = {
          id: event.id,
          timestamp: event.timestamp,
          type: source.toUpperCase().includes('SSE') ? 'SSE' : 'SSL',
          content: extractPromptContent(data),
          url: extractUrl(data),
          method: extractMethod(data)
        };

        if (!promptsByPid.has(pid)) {
          promptsByPid.set(pid, []);
        }
        promptsByPid.get(pid)!.push(prompt);
      }

      // Try to extract parent PID from process events
      if (source === 'process' && data.ppid) {
        const process = processMap.get(pid)!;
        process.ppid = data.ppid;
      }
    });

    // Assign prompts to processes
    promptsByPid.forEach((prompts, pid) => {
      const process = processMap.get(pid);
      if (process) {
        process.prompts = prompts.sort((a, b) => a.timestamp - b.timestamp);
      }
    });

    // Second pass: build tree structure
    const rootProcesses: ProcessNode[] = [];
    const childProcesses = new Set<number>();

    processMap.forEach((process, pid) => {
      if (process.ppid && processMap.has(process.ppid)) {
        const parent = processMap.get(process.ppid)!;
        parent.children.push(process);
        childProcesses.add(pid);
      }
    });

    // Root processes are those without parents or whose parents don't exist
    processMap.forEach((process, pid) => {
      if (!childProcesses.has(pid)) {
        rootProcesses.push(process);
      }
    });

    return rootProcesses.sort((a, b) => a.pid - b.pid);
  }, [events, expandedProcesses]);

  const toggleProcessExpansion = (pid: number) => {
    const newExpanded = new Set(expandedProcesses);
    if (newExpanded.has(pid)) {
      newExpanded.delete(pid);
    } else {
      newExpanded.add(pid);
    }
    setExpandedProcesses(newExpanded);
  };

  const togglePromptExpansion = (promptId: string) => {
    const newExpanded = new Set(expandedPrompts);
    if (newExpanded.has(promptId)) {
      newExpanded.delete(promptId);
    } else {
      newExpanded.add(promptId);
    }
    setExpandedPrompts(newExpanded);
  };

  const renderProcessNode = (process: ProcessNode, depth: number = 0) => {
    const isExpanded = expandedProcesses.has(process.pid);
    const hasChildren = process.children.length > 0;
    const hasPrompts = process.prompts.length > 0;
    const indent = depth * 24;

    return (
      <div key={process.pid} className="select-none">
        {/* Process Header */}
        <div
          className="flex items-center py-2 px-3 hover:bg-gray-50 cursor-pointer border-l-2 border-blue-200"
          style={{ marginLeft: `${indent}px` }}
          onClick={() => toggleProcessExpansion(process.pid)}
        >
          <div className="flex items-center flex-1">
            {hasChildren || hasPrompts ? (
              isExpanded ? (
                <ChevronDownIcon className="h-4 w-4 text-gray-500 mr-2" />
              ) : (
                <ChevronRightIcon className="h-4 w-4 text-gray-500 mr-2" />
              )
            ) : (
              <div className="w-6 mr-2" />
            )}
            
            <div className="flex items-center space-x-2">
              <span className="text-xs text-gray-500 font-mono">PID {process.pid}</span>
              <span className="font-medium text-gray-900">[{process.comm}]</span>
              {hasPrompts && (
                <span className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded-full">
                  {process.prompts.length} prompt{process.prompts.length !== 1 ? 's' : ''}
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Expanded Content */}
        {isExpanded && (
          <div style={{ marginLeft: `${indent + 24}px` }}>
            {/* Prompts */}
            {process.prompts.map(prompt => renderPrompt(prompt, depth + 1))}
            
            {/* Child Processes */}
            {process.children.map(child => renderProcessNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  const renderPrompt = (prompt: PromptEvent, depth: number) => {
    const isExpanded = expandedPrompts.has(prompt.id);
    const truncatedContent = prompt.content.length > 100 
      ? prompt.content.substring(0, 100) + '...' 
      : prompt.content;

    return (
      <div key={prompt.id} className="mb-2">
        <div
          className="flex items-start p-3 bg-gradient-to-r from-purple-50 to-blue-50 border border-purple-200 rounded-lg cursor-pointer hover:from-purple-100 hover:to-blue-100 transition-colors"
          onClick={() => togglePromptExpansion(prompt.id)}
        >
          <div className="flex items-center flex-1">
            {prompt.content.length > 100 ? (
              isExpanded ? (
                <ChevronDownIcon className="h-4 w-4 text-purple-600 mr-2 flex-shrink-0" />
              ) : (
                <ChevronRightIcon className="h-4 w-4 text-purple-600 mr-2 flex-shrink-0" />
              )
            ) : (
              <div className="w-6 mr-2" />
            )}
            
            <div className="flex-1">
              <div className="flex items-center space-x-2 mb-1">
                <span className={`px-2 py-1 text-xs font-bold rounded ${
                  prompt.type === 'SSL' 
                    ? 'bg-green-100 text-green-800' 
                    : 'bg-blue-100 text-blue-800'
                }`}>
                  {prompt.type}
                </span>
                {prompt.method && (
                  <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded font-mono">
                    {prompt.method}
                  </span>
                )}
                <span className="text-xs text-gray-500">
                  {new Date(prompt.timestamp).toLocaleTimeString()}
                </span>
              </div>
              
              {prompt.url && (
                <div className="text-sm text-blue-600 font-mono mb-1 truncate">
                  ➜ {prompt.url}
                </div>
              )}
              
              <div className="text-sm text-gray-700">
                {isExpanded ? prompt.content : truncatedContent}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="bg-white rounded-lg shadow-md">
      <div className="border-b border-gray-200 p-4">
        <h2 className="text-lg font-semibold text-gray-900">Process Tree & AI Prompts</h2>
        <p className="text-sm text-gray-600 mt-1">
          Hierarchical view of processes with their AI prompts and API calls
        </p>
      </div>

      <div className="p-4">
        {processTree.length === 0 ? (
          <div className="text-center text-gray-500 py-8">
            No processes to display
          </div>
        ) : (
          <div className="space-y-1 font-mono text-sm">
            {processTree.map(process => renderProcessNode(process))}
          </div>
        )}
      </div>
    </div>
  );
}

// Helper functions to extract data from events
function extractPromptContent(data: any): string {
  if (typeof data === 'string') return data;
  if (data.content) return data.content;
  if (data.body) return data.body;
  if (data.message) return data.message;
  if (data.data) return JSON.stringify(data.data);
  return JSON.stringify(data);
}

function extractUrl(data: any): string | undefined {
  if (data.url) return data.url;
  if (data.uri) return data.uri;
  if (data.path) return data.path;
  if (data.host && data.path) return `${data.host}${data.path}`;
  return undefined;
}

function extractMethod(data: any): string | undefined {
  if (data.method) return data.method.toUpperCase();
  if (data.verb) return data.verb.toUpperCase();
  return undefined;
}