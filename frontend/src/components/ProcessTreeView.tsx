'use client';

import { useState, useMemo } from 'react';
import { Event } from '@/types/event';
import { buildProcessTree, ProcessNode as ProcessNodeType } from '@/utils/eventParsers';
import { ProcessNode } from './process-tree/ProcessNode';

interface ProcessTreeViewProps {
  events: Event[];
}

export function ProcessTreeView({ events }: ProcessTreeViewProps) {
  const [expandedProcesses, setExpandedProcesses] = useState<Set<number>>(new Set());
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());

  // Build process hierarchy from events using the new parser
  const processTree = useMemo(() => {
    return buildProcessTree(events);
  }, [events]);

  const toggleProcessExpansion = (pid: number) => {
    const newExpanded = new Set(expandedProcesses);
    if (newExpanded.has(pid)) {
      newExpanded.delete(pid);
    } else {
      newExpanded.add(pid);
    }
    setExpandedProcesses(newExpanded);
  };

  const toggleEventExpansion = (eventId: string) => {
    const newExpanded = new Set(expandedEvents);
    if (newExpanded.has(eventId)) {
      newExpanded.delete(eventId);
    } else {
      newExpanded.add(eventId);
    }
    setExpandedEvents(newExpanded);
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
          <div className="space-y-2">
            {processTree.map(process => (
              <ProcessNode
                key={process.pid}
                process={process}
                depth={0}
                expandedProcesses={expandedProcesses}
                expandedEvents={expandedEvents}
                onToggleProcess={toggleProcessExpansion}
                onToggleEvent={toggleEventExpansion}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}