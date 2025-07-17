import { Event } from '@/types/event';
import { ProcessNode, parseEventData } from './eventParsers';
import { ProcessTreeFilters } from '@/components/process-tree/ProcessTreeFilters';

// Extract unique filter options from events
export function extractFilterOptions(events: Event[]) {
  const eventTypes = new Set<string>();
  const models = new Set<string>();
  const sources = new Set<string>();
  const commands = new Set<string>();

  events.forEach(event => {
    // Parse the event to get structured data
    const parsedEvent = parseEventData(event);
    
    // Event types
    eventTypes.add(parsedEvent.type);
    
    // Sources
    sources.add(event.source);
    
    // Commands
    if (event.comm) {
      commands.add(event.comm);
    }
    
    // Models (extract from different event types)
    if (parsedEvent.type === 'prompt' || parsedEvent.type === 'response') {
      const model = parsedEvent.metadata?.model;
      if (model && model !== 'Unknown Model') {
        models.add(model);
      }
    }
  });

  return {
    eventTypes: Array.from(eventTypes).sort(),
    models: Array.from(models).sort(),
    sources: Array.from(sources).sort(),
    commands: Array.from(commands).sort()
  };
}

// Check if an event matches the filters
export function eventMatchesFilters(event: Event, filters: ProcessTreeFilters): boolean {
  const parsedEvent = parseEventData(event);
  
  // Event type filter
  if (filters.eventTypes.length > 0 && !filters.eventTypes.includes(parsedEvent.type)) {
    return false;
  }
  
  // Source filter
  if (filters.sources.length > 0 && !filters.sources.includes(event.source)) {
    return false;
  }
  
  // Command filter
  if (filters.commands.length > 0 && (!event.comm || !filters.commands.includes(event.comm))) {
    return false;
  }
  
  // Model filter
  if (filters.models.length > 0) {
    const model = parsedEvent.metadata?.model;
    if (!model || !filters.models.includes(model)) {
      return false;
    }
  }
  
  // Time range filter
  if (filters.timeRange.start && event.timestamp < filters.timeRange.start) {
    return false;
  }
  
  if (filters.timeRange.end && event.timestamp > filters.timeRange.end) {
    return false;
  }
  
  // Search text filter
  if (filters.searchText) {
    const searchLower = filters.searchText.toLowerCase();
    const searchableText = [
      parsedEvent.title,
      parsedEvent.content,
      event.comm,
      event.source,
      parsedEvent.metadata?.model,
      JSON.stringify(event.data)
    ].filter(Boolean).join(' ').toLowerCase();
    
    if (!searchableText.includes(searchLower)) {
      return false;
    }
  }
  
  return true;
}

// Filter events and return filtered events
export function filterEvents(events: Event[], filters: ProcessTreeFilters): Event[] {
  return events.filter(event => eventMatchesFilters(event, filters));
}

// Filter process tree by applying filters to events within each process
export function filterProcessTree(processTree: ProcessNode[], filters: ProcessTreeFilters): ProcessNode[] {
  return processTree.map(process => {
    // Filter events within this process
    const filteredEvents = process.events.filter(event => {
      // Create a mock Event object from ParsedEvent for filtering
      const mockEvent: Event = {
        id: event.id,
        timestamp: event.timestamp,
        source: '', // We'll need to reconstruct this from metadata
        pid: process.pid,
        comm: process.comm,
        data: event.metadata
      };
      
      // Try to reconstruct source from event metadata
      if (event.metadata?.original_source) {
        mockEvent.source = event.metadata.original_source;
      } else if (event.type === 'prompt' || event.type === 'response') {
        mockEvent.source = 'http_parser';
      } else if (event.type === 'file') {
        mockEvent.source = 'process';
      } else if (event.type === 'process') {
        mockEvent.source = 'process';
      } else {
        mockEvent.source = 'ssl';
      }
      
      return eventMatchesFilters(mockEvent, filters);
    });
    
    // Recursively filter children
    const filteredChildren = filterProcessTree(process.children, filters);
    
    // Return process if it has filtered events or filtered children
    if (filteredEvents.length > 0 || filteredChildren.length > 0) {
      return {
        ...process,
        events: filteredEvents,
        children: filteredChildren
      };
    }
    
    return null;
  }).filter((process): process is ProcessNode => process !== null);
}

// Get total event count from process tree
export function getTotalEventCount(processTree: ProcessNode[]): number {
  return processTree.reduce((total, process) => {
    return total + process.events.length + getTotalEventCount(process.children);
  }, 0);
}

// Create default filters
export function createDefaultFilters(): ProcessTreeFilters {
  return {
    eventTypes: [],
    models: [],
    sources: [],
    commands: [],
    searchText: '',
    timeRange: {}
  };
}