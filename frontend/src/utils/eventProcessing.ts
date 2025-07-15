import { Event, ProcessedEvent } from '@/types/event';

export const SOURCE_COLORS = [
  '#3B82F6', // blue
  '#10B981', // green
  '#F59E0B', // yellow
  '#8B5CF6', // purple
  '#EF4444', // red
  '#6366F1', // indigo
  '#EC4899', // pink
  '#6B7280'  // gray
];

export const SOURCE_COLOR_CLASSES = [
  'bg-blue-100 text-blue-800',
  'bg-green-100 text-green-800',
  'bg-yellow-100 text-yellow-800',
  'bg-purple-100 text-purple-800',
  'bg-red-100 text-red-800',
  'bg-indigo-100 text-indigo-800',
  'bg-pink-100 text-pink-800',
  'bg-gray-100 text-gray-800'
];

export function processEvents(events: Event[]): ProcessedEvent[] {
  const sourceColorMap = new Map<string, string>();
  const sourceColorClassMap = new Map<string, string>();
  let colorIndex = 0;

  return events.map(event => {
    const datetime = new Date(event.timestamp);
    const formattedTime = datetime.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    }) + '.' + datetime.getMilliseconds().toString().padStart(3, '0');

    // Assign colors to sources
    if (!sourceColorMap.has(event.source)) {
      sourceColorMap.set(event.source, SOURCE_COLORS[colorIndex % SOURCE_COLORS.length]);
      sourceColorClassMap.set(event.source, SOURCE_COLOR_CLASSES[colorIndex % SOURCE_COLOR_CLASSES.length]);
      colorIndex++;
    }

    return {
      ...event,
      datetime,
      formattedTime,
      sourceColor: sourceColorMap.get(event.source) || SOURCE_COLORS[0],
      sourceColorClass: sourceColorClassMap.get(event.source) || SOURCE_COLOR_CLASSES[0]
    };
  });
}

export function getUniqueValues<T>(items: T[], key: keyof T): T[keyof T][] {
  const unique = new Set(items.map(item => item[key]));
  return Array.from(unique);
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

export function formatEventSummary(event: ProcessedEvent): string {
  return `${event.comm} (${event.pid})`;
}

export function filterEvents(
  events: ProcessedEvent[],
  filters: {
    source?: string;
    comm?: string;
    pid?: string;
    searchTerm?: string;
  }
): ProcessedEvent[] {
  let filtered = events;

  if (filters.source) {
    filtered = filtered.filter(event => event.source === filters.source);
  }

  if (filters.comm) {
    filtered = filtered.filter(event => event.comm === filters.comm);
  }

  if (filters.pid) {
    filtered = filtered.filter(event => event.pid.toString() === filters.pid);
  }

  if (filters.searchTerm) {
    const term = filters.searchTerm.toLowerCase();
    filtered = filtered.filter(event =>
      event.source.toLowerCase().includes(term) ||
      event.id.toLowerCase().includes(term) ||
      event.comm.toLowerCase().includes(term) ||
      event.pid.toString().includes(term) ||
      JSON.stringify(event.data).toLowerCase().includes(term)
    );
  }

  return filtered;
}