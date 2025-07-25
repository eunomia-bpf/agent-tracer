import { Event } from '@/types/event';
import { comparePrompts } from './jsonDiff';

// Store prompt history per process for diff comparison
const promptHistoryByPid = new Map<number, ParsedEvent[]>();

export interface ProcessNode {
  pid: number;
  comm: string;
  ppid?: number;
  children: ProcessNode[];
  events: ParsedEvent[];
  timeline: TimelineItem[]; // Mixed events and child processes in chronological order
  isExpanded: boolean;
}

export interface TimelineItem {
  type: 'event' | 'process';
  timestamp: number;
  event?: ParsedEvent;
  process?: ProcessNode;
}

export interface ParsedEvent {
  id: string;
  timestamp: number;
  type: 'prompt' | 'response' | 'ssl' | 'file' | 'process';
  title: string;
  content: string;
  metadata: Record<string, any>;
  isExpanded: boolean;
  // For prompts, store diff with previous prompt
  promptDiff?: {
    diff: string;
    summary: string;
    hasChanges: boolean;
    previousPromptId?: string;
  };
}

export interface PromptData {
  model?: string;
  messages?: Array<{ 
    role: string; 
    content: string | Array<any> | any;
  }>;
  system?: Array<{ 
    type?: string; 
    text?: string; 
    cache_control?: any;
  } | string>;
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  metadata?: {
    user_id?: string;
    [key: string]: any;
  };
}

export interface ResponseData {
  message_id?: string;
  connection_id?: string;
  model?: string;
  role?: string;
  content?: string;
  duration_ns?: number;
  event_count?: number;
  function?: string;
  has_message_start?: boolean;
  start_time?: number;
  end_time?: number;
  usage?: {
    input_tokens?: number;
    output_tokens?: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
    service_tier?: string;
  };
  sse_events?: Array<{
    event: string;
    data: string;
    parsed_data?: any;
  }>;
  text_content?: string;
}

export interface SSLData {
  method?: string;
  path?: string;
  host?: string;
  headers?: Record<string, string>;
  body?: string;
  status_code?: number;
  content_length?: number;
  message_type?: 'request' | 'response';
}

export interface FileData {
  operation?: string;
  path?: string;
  filepath?: string;
  event?: string;
  size?: number;
  permissions?: string;
  fd?: number;
  flags?: number;
  count?: number;
  pid?: number;
  comm?: string;
}

// Utility class for safe data extraction
class DataExtractor {
  private data: any;

  constructor(data: any) {
    this.data = data;
  }

  // Safely get nested values
  get(path: string, defaultValue: any = undefined): any {
    return path.split('.').reduce((obj, key) => {
      return obj && obj[key] !== undefined ? obj[key] : defaultValue;
    }, this.data);
  }

  // Try to parse JSON strings safely
  parseJson(value: any): any {
    if (typeof value === 'string') {
      try {
        return JSON.parse(value);
      } catch {
        return value;
      }
    }
    return value;
  }

  // Convert any value to readable string, pretty printing JSON
  toString(value: any, indent = 2): string {
    if (value === null || value === undefined) return '';
    if (typeof value === 'string') return value;
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    if (typeof value === 'object') {
      try {
        return JSON.stringify(value, null, indent);
      } catch (error) {
        // Fallback for circular references or other JSON errors
        return String(value);
      }
    }
    return String(value);
  }

  // Get prompt data from various nested structures
  getPromptData(): any {
    const candidates = [
      this.parseJson(this.get('body')),
      this.parseJson(this.get('data.data')),
      this.get('data'),
      this.data
    ];

    for (const candidate of candidates) {
      if (candidate && (candidate.model || candidate.messages || candidate.prompt)) {
        return candidate;
      }
    }
    return this.data;
  }

  // Get raw data for debugging/full visibility
  getRawData(): string {
    return this.toString(this.data, 2);
  }

  // Check if data seems to be AI-related but couldn't be parsed properly
  isUnparsedAiData(): boolean {
    const raw = this.toString(this.data).toLowerCase();
    return raw.includes('model') || raw.includes('messages') || raw.includes('prompt') || 
           raw.includes('temperature') || raw.includes('max_tokens') || raw.includes('anthropic') ||
           raw.includes('openai') || raw.includes('claude');
  }
}

// Parse different types of events
export function parseEventData(event: Event): ParsedEvent {
  const eventType = determineEventType(event.source, event.data);
  
  switch (eventType) {
    case 'prompt':
      return parsePromptEvent(event);
    case 'response':
      return parseResponseEvent(event);
    case 'ssl':
      return parseSSLEvent(event);
    case 'file':
      return parseFileEvent(event);
    case 'process':
      return parseProcessEvent(event);
    default:
      return parseGenericEvent(event);
  }
}

function determineEventType(source: string, data: any): ParsedEvent['type'] {
  if (isPromptEvent(source, data)) return 'prompt';
  if (isResponseEvent(source, data)) return 'response';
  if (isFileEvent(source, data)) return 'file';
  if (isProcessEvent(source, data)) return 'process';
  if (source.toLowerCase().includes('ssl') || source === 'http_parser') return 'ssl';
  return 'ssl';
}

function isPromptEvent(source: string, data: any): boolean {
  // Simple heuristics for AI request detection
  const hasAIRequestIndicators = 
    data.model || 
    data.messages || 
    data.prompt || 
    data.inputs ||
    data.query ||
    (data.method === 'POST' && data.message_type === 'request' && 
     (data.path?.includes('/v1/') || data.path?.includes('/api/')));
    
  return !!hasAIRequestIndicators;
}

function isResponseEvent(source: string, data: any): boolean {
  // Simple heuristics for AI response detection
  const hasAIResponseIndicators = 
    data.choices ||
    data.completion ||
    data.response ||
    data.sse_events ||
    data.delta ||
    data.content_block ||
    (source === 'sse_processor' && data.sse_events) ||
    (data.message_type === 'response' && (data.model || data.usage));
    
  return !!hasAIResponseIndicators;
}

function isFileEvent(source: string, data: any): boolean {
  const extractor = new DataExtractor(data);
  return source === 'file' || 
         extractor.get('fd') !== undefined ||
         (extractor.get('operation') && ['open', 'read', 'write', 'close'].includes(extractor.get('operation'))) ||
         (extractor.get('event', '').includes('FILE_')) ||
         extractor.get('filepath') !== undefined;
}

function isProcessEvent(source: string, data: any): boolean {
  const extractor = new DataExtractor(data);
  return (source === 'process' && !extractor.get('event', '').includes('FILE_')) || 
         extractor.get('exec') !== undefined ||
         extractor.get('exit') !== undefined ||
         extractor.get('event') === 'EXEC' ||
         extractor.get('event') === 'EXIT' ||
         (extractor.get('ppid') !== undefined && !extractor.get('event', '').includes('FILE_'));
}

function parsePromptEvent(event: Event): ParsedEvent {
  const data = event.data;
  let model = data.model || 'AI Request';
  const method = data.method || 'POST';
  
  // For http_parser events, parse the body field if it exists
  let displayData = data;
  if (data.body && typeof data.body === 'string') {
    try {
      const parsedBody = JSON.parse(data.body);
      // Extract model from parsed body if available
      if (parsedBody.model) {
        model = parsedBody.model;
      }
      // Use parsed body as display data
      displayData = { ...data, body: parsedBody };
    } catch (e) {
      // Keep original data if parsing fails
    }
  }
  
  // Simply show the JSON data as-is
  const content = JSON.stringify(displayData, null, 2);
  
  const parsedEvent: ParsedEvent = {
    id: event.id,
    timestamp: event.timestamp,
    type: 'prompt',
    title: `${method} ${model}`,
    content: content,
    metadata: { model, method, url: `${data.host || ''}${data.path || ''}`, raw: data },
    isExpanded: false
  };
  
  // Get prompt history for this process
  const pid = event.pid;
  if (!promptHistoryByPid.has(pid)) {
    promptHistoryByPid.set(pid, []);
  }
  
  const history = promptHistoryByPid.get(pid)!;
  
  // If there's a previous prompt, generate diff
  if (history.length > 0) {
    const previousPrompt = history[history.length - 1];
    const diffResult = comparePrompts(previousPrompt.metadata.raw, data);
    
    parsedEvent.promptDiff = {
      ...diffResult,
      previousPromptId: previousPrompt.id
    };
  }
  
  // Add this prompt to history
  history.push(parsedEvent);
  
  // Keep only last 10 prompts per process to avoid memory issues
  if (history.length > 10) {
    history.shift();
  }
  
  return parsedEvent;
}

function parseResponseEvent(event: Event): ParsedEvent {
  const data = event.data;
  let model = data.model || 'AI Response';
  
  // For sse_processor events, extract model and enhance display
  let displayData = data;
  if (data.sse_events && Array.isArray(data.sse_events)) {
    // Look for model in SSE events
    for (const sseEvent of data.sse_events) {
      if (sseEvent.parsed_data?.message?.model) {
        model = sseEvent.parsed_data.message.model;
        break;
      }
    }
  }
  
  // Simply show the JSON data as-is
  const content = JSON.stringify(displayData, null, 2);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'response',
    title: model,
    content: content,
    metadata: { model, raw: data },
    isExpanded: false
  };
}

function parseSSLEvent(event: Event): ParsedEvent {
  const data = event.data;
  
  const method = data.method || 'UNKNOWN';
  const host = data.host || data.headers?.host || 'unknown';
  const path = data.path || '/';
  const statusCode = data.status_code;
  
  let title = `${method} ${host}${path}`;
  if (statusCode) title += ` (${statusCode})`;
  
  // Simply show the JSON data as-is
  const content = JSON.stringify(data, null, 2);

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title,
    content: content,
    metadata: data,
    isExpanded: false
  };
}

function parseFileEvent(event: Event): ParsedEvent {
  const data = event.data;
  const operation = data.operation || data.event || 'file op';
  const path = data.path || data.filepath || 'unknown';
  
  // Simply show the JSON data as-is
  const content = JSON.stringify(data, null, 2);

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'file',
    title: `${operation} ${path}`,
    content: content,
    metadata: data,
    isExpanded: false
  };
}

function parseProcessEvent(event: Event): ParsedEvent {
  const data = event.data;
  const eventType = data.event || 'process';
  const filename = data.filename;
  const title = filename ? `${eventType}: ${filename}` : `${eventType} event`;
  
  // Simply show the JSON data as-is
  const content = JSON.stringify(data, null, 2);

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'process',
    title,
    content: content,
    metadata: data,
    isExpanded: false
  };
}

function parseGenericEvent(event: Event): ParsedEvent {
  // Simply show the JSON data as-is
  const content = JSON.stringify(event.data, null, 2);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title: `${event.source} event`,
    content: content,
    metadata: event.data,
    isExpanded: false
  };
}

// Helper function to get the earliest timestamp for a process
function getEarliestTimestamp(process: ProcessNode): number {
  let earliest = Infinity;
  
  // Check process events
  if (process.events.length > 0) {
    earliest = Math.min(earliest, process.events[0].timestamp);
  }
  
  // Check child processes recursively
  process.children.forEach(child => {
    earliest = Math.min(earliest, getEarliestTimestamp(child));
  });
  
  return earliest === Infinity ? 0 : earliest;
}

// Build process hierarchy from events
export function buildProcessTree(events: Event[]): ProcessNode[] {
  const processMap = new Map<number, ProcessNode>();
  const eventsByPid = new Map<number, ParsedEvent[]>();
  
  // First pass: create process nodes and parse events
  events.forEach(event => {
    const { pid, comm } = event;
    
    // Initialize process if not exists
    if (!processMap.has(pid)) {
      processMap.set(pid, {
        pid,
        comm: comm || 'unknown',
        children: [],
        events: [],
        timeline: [],
        isExpanded: false
      });
    }
    
    // Parse event and group by PID
    const parsedEvent = parseEventData(event);
    if (!eventsByPid.has(pid)) {
      eventsByPid.set(pid, []);
    }
    eventsByPid.get(pid)!.push(parsedEvent);
    
    // Extract parent PID if available
    if (event.source === 'process' && event.data.ppid) {
      const process = processMap.get(pid)!;
      process.ppid = event.data.ppid;
    }
  });
  
  // Assign events to processes
  eventsByPid.forEach((events, pid) => {
    const process = processMap.get(pid);
    if (process) {
      process.events = events.sort((a, b) => a.timestamp - b.timestamp);
    }
  });
  
  // Build tree structure
  const rootProcesses: ProcessNode[] = [];
  const childProcesses = new Set<number>();
  
  processMap.forEach((process, pid) => {
    if (process.ppid && processMap.has(process.ppid)) {
      const parent = processMap.get(process.ppid)!;
      parent.children.push(process);
      childProcesses.add(pid);
    }
  });
  
  // Build timeline for each process (mix events and child processes chronologically)
  processMap.forEach(process => {
    const timelineItems: TimelineItem[] = [];
    
    // Add all events as timeline items
    process.events.forEach(event => {
      timelineItems.push({
        type: 'event',
        timestamp: event.timestamp,
        event
      });
    });
    
    // Add child processes as timeline items (using their earliest timestamp)
    process.children.forEach(child => {
      timelineItems.push({
        type: 'process',
        timestamp: getEarliestTimestamp(child),
        process: child
      });
    });
    
    // Sort timeline by timestamp
    process.timeline = timelineItems.sort((a, b) => a.timestamp - b.timestamp);
  });
  
  // Root processes are those without parents
  processMap.forEach((process, pid) => {
    if (!childProcesses.has(pid)) {
      rootProcesses.push(process);
    }
  });
  
  // Sort root processes by their earliest timestamp
  return rootProcesses.sort((a, b) => getEarliestTimestamp(a) - getEarliestTimestamp(b));
}