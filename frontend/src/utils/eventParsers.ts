import { Event } from '@/types/event';

export interface ProcessNode {
  pid: number;
  comm: string;
  ppid?: number;
  children: ProcessNode[];
  events: ParsedEvent[];
  isExpanded: boolean;
}

export interface ParsedEvent {
  id: string;
  timestamp: number;
  type: 'prompt' | 'response' | 'ssl' | 'file' | 'process';
  title: string;
  content: string;
  metadata: Record<string, any>;
  isExpanded: boolean;
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
  const extractor = new DataExtractor(data);
  
  if (extractor.get('method') === 'POST' && extractor.get('message_type') === 'request') {
    const path = extractor.get('path', '');
    const host = extractor.get('host') || extractor.get('headers.host', '');
    
    const aiEndpoints = ['api.openai.com', 'api.anthropic.com', 'api.claude.ai'];
    const aiPaths = ['/v1/chat/completions', '/v1/completions', '/v1/messages'];
    
    if (aiEndpoints.some(endpoint => host.includes(endpoint)) || 
        aiPaths.some(apiPath => path.includes(apiPath))) {
      return true;
    }
    
    const promptData = extractor.getPromptData();
    return !!(promptData.model || promptData.messages || promptData.prompt);
  }
  
  return false;
}

function isResponseEvent(source: string, data: any): boolean {
  const extractor = new DataExtractor(data);
  
  if (source === 'sse_processor' && extractor.get('sse_events')) {
    return true;
  }
  
  if (extractor.get('message_type') === 'response' && extractor.get('status_code')) {
    const host = extractor.get('host') || extractor.get('headers.host', '');
    const aiEndpoints = ['api.openai.com', 'api.anthropic.com', 'api.claude.ai'];
    return aiEndpoints.some(endpoint => host.includes(endpoint));
  }
  
  return false;
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
  const extractor = new DataExtractor(event.data);
  const promptData = extractor.getPromptData();
  
  const model = promptData.model || 'Unknown Model';
  const method = extractor.get('method', 'POST');
  
  const sections: string[] = [];
  let hasStructuredData = false;
  
  // System messages
  if (promptData.system?.length > 0) {
    hasStructuredData = true;
    sections.push('=== SYSTEM ===');
    promptData.system.forEach((s: any) => {
      if (typeof s === 'object' && s.text) {
        sections.push(s.text);
        if (s.cache_control) {
          sections.push(`[Cache: ${extractor.toString(s.cache_control)}]`);
        }
      } else {
        sections.push(extractor.toString(s));
      }
    });
    sections.push('');
  }
  
  // Messages
  if (promptData.messages?.length > 0) {
    hasStructuredData = true;
    promptData.messages.forEach((msg: any) => {
      sections.push(`=== ${msg.role.toUpperCase()} ===`);
      sections.push(extractor.toString(msg.content));
      sections.push('');
    });
  }
  
  // Parameters
  const params: string[] = [];
  if (promptData.temperature !== undefined) params.push(`temp: ${promptData.temperature}`);
  if (promptData.max_tokens !== undefined) params.push(`max_tokens: ${promptData.max_tokens}`);
  if (promptData.stream !== undefined) params.push(`stream: ${promptData.stream}`);
  if (promptData.metadata?.user_id) params.push(`user: ${promptData.metadata.user_id.slice(0, 8)}...`);
  
  if (params.length > 0) {
    hasStructuredData = true;
    sections.push('=== PARAMETERS ===');
    sections.push(params.join(', '));
  }

  // If we couldn't parse structured data but this looks like AI data, show raw data
  if (!hasStructuredData || extractor.isUnparsedAiData()) {
    sections.push('=== RAW DATA ===');
    sections.push(extractor.getRawData());
  }

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'prompt',
    title: `${method} ${model}`,
    content: sections.join('\n'),
    metadata: { model, method, url: `${extractor.get('host', '')}${extractor.get('path', '')}`, ...promptData },
    isExpanded: false
  };
}

function parseResponseEvent(event: Event): ParsedEvent {
  const extractor = new DataExtractor(event.data);
  
  let responseText = '';
  let model = '';
  let usage: any = null;
  let hasStructuredData = false;
  
  if (extractor.get('sse_events')) {
    const textParts: string[] = [];
    
    extractor.get('sse_events', []).forEach((sseEvent: any) => {
      const parsed = sseEvent.parsed_data;
      if (parsed?.type === 'content_block_delta' && parsed.delta?.text) {
        textParts.push(parsed.delta.text);
        hasStructuredData = true;
      } else if (parsed?.type === 'message_start') {
        model = parsed.message?.model || '';
        usage = parsed.message?.usage;
        hasStructuredData = true;
      }
    });
    
    responseText = textParts.join('');
  } else {
    responseText = extractor.get('text_content', '');
    if (responseText) hasStructuredData = true;
  }
  
  const sections: string[] = [];
  
  if (responseText) {
    sections.push('=== RESPONSE ===');
    sections.push(responseText);
    sections.push('');
  }
  
  // Metadata
  const metadata: string[] = [];
  if (extractor.get('message_id')) metadata.push(`id: ${extractor.get('message_id')}`);
  if (extractor.get('duration_ns')) metadata.push(`duration: ${(extractor.get('duration_ns') / 1000000).toFixed(1)}ms`);
  if (extractor.get('event_count')) metadata.push(`events: ${extractor.get('event_count')}`);
  if (usage?.input_tokens) metadata.push(`in: ${usage.input_tokens}t`);
  if (usage?.output_tokens) metadata.push(`out: ${usage.output_tokens}t`);
  if (usage?.service_tier) metadata.push(`tier: ${usage.service_tier}`);
  
  if (metadata.length > 0) {
    hasStructuredData = true;
    sections.push('=== METADATA ===');
    sections.push(metadata.join(', '));
  }

  // Always include raw data for responses to show full SSE structure
  sections.push('=== RAW DATA ===');
  sections.push(extractor.getRawData());

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'response',
    title: model ? `Response from ${model}` : 'AI Response',
    content: sections.join('\n'),
    metadata: event.data,
    isExpanded: false
  };
}

function parseSSLEvent(event: Event): ParsedEvent {
  const extractor = new DataExtractor(event.data);
  
  const method = extractor.get('method', 'UNKNOWN');
  const host = extractor.get('host') || extractor.get('headers.host', 'unknown');
  const path = extractor.get('path', '/');
  const statusCode = extractor.get('status_code');
  
  let title = `${method} ${host}${path}`;
  if (statusCode) title += ` (${statusCode})`;
  
  const sections: string[] = [];
  
  // Show body if available
  const body = extractor.get('body');
  if (body) {
    sections.push('=== BODY ===');
    sections.push(extractor.toString(body));
    sections.push('');
  }
  
  // Always show full raw data for SSL events
  sections.push('=== RAW DATA ===');
  sections.push(extractor.getRawData());

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title,
    content: sections.join('\n'),
    metadata: event.data,
    isExpanded: false
  };
}

function parseFileEvent(event: Event): ParsedEvent {
  const extractor = new DataExtractor(event.data);
  
  const operation = extractor.get('operation') || extractor.get('event', 'file op');
  const path = extractor.get('path') || extractor.get('filepath', 'unknown');
  
  const sections: string[] = [];
  sections.push(`=== ${operation.toUpperCase()} ===`);
  sections.push(`Path: ${path}`);
  
  const metadata: string[] = [];
  if (extractor.get('size') !== undefined) metadata.push(`size: ${extractor.get('size')}`);
  if (extractor.get('fd') !== undefined) metadata.push(`fd: ${extractor.get('fd')}`);
  if (extractor.get('flags') !== undefined) metadata.push(`flags: ${extractor.get('flags')}`);
  if (extractor.get('permissions')) metadata.push(`perms: ${extractor.get('permissions')}`);
  
  if (metadata.length > 0) {
    sections.push('');
    sections.push('=== DETAILS ===');
    sections.push(metadata.join(', '));
  }

  // Show raw data for complete visibility
  sections.push('');
  sections.push('=== RAW DATA ===');
  sections.push(extractor.getRawData());

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'file',
    title: `${operation} ${path}`,
    content: sections.join('\n'),
    metadata: event.data,
    isExpanded: false
  };
}

function parseProcessEvent(event: Event): ParsedEvent {
  const extractor = new DataExtractor(event.data);
  
  const eventType = extractor.get('event', 'process');
  const filename = extractor.get('filename');
  const pid = extractor.get('pid');
  const ppid = extractor.get('ppid');
  
  const sections: string[] = [];
  sections.push(`=== ${eventType.toUpperCase()} ===`);
  
  if (filename) sections.push(`Executable: ${filename}`);
  if (pid) sections.push(`PID: ${pid}`);
  if (ppid) sections.push(`Parent PID: ${ppid}`);
  
  // Show raw data for complete visibility
  sections.push('');
  sections.push('=== RAW DATA ===');
  sections.push(extractor.getRawData());
  
  const title = filename ? `${eventType}: ${filename}` : `${eventType} event`;

  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'process',
    title,
    content: sections.join('\n'),
    metadata: event.data,
    isExpanded: false
  };
}

function parseGenericEvent(event: Event): ParsedEvent {
  const extractor = new DataExtractor(event.data);
  
  const sections: string[] = [];
  sections.push(`=== ${event.source.toUpperCase()} EVENT ===`);
  sections.push('=== RAW DATA ===');
  sections.push(extractor.getRawData());
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title: `${event.source} event`,
    content: sections.join('\n'),
    metadata: event.data,
    isExpanded: false
  };
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
  
  // Root processes are those without parents
  processMap.forEach((process, pid) => {
    if (!childProcesses.has(pid)) {
      rootProcesses.push(process);
    }
  });
  
  return rootProcesses.sort((a, b) => a.pid - b.pid);
}