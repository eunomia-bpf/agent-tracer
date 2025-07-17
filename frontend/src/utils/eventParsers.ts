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
  messages?: Array<{ role: string; content: string }>;
  system?: Array<{ type: string; text: string }>;
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
}

export interface ResponseData {
  message_id?: string;
  model?: string;
  role?: string;
  content?: string;
  usage?: {
    input_tokens?: number;
    output_tokens?: number;
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
  size?: number;
  permissions?: string;
  fd?: number;
}

// Parse different types of events
export function parseEventData(event: Event): ParsedEvent {
  const { id, timestamp, source, data } = event;
  
  // Determine event type based on source and data content
  const eventType = determineEventType(source, data);
  
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
  // Check for AI prompts (requests to AI APIs)
  if (isPromptEvent(source, data)) return 'prompt';
  
  // Check for AI responses (responses from AI APIs)
  if (isResponseEvent(source, data)) return 'response';
  
  // Check for file operations
  if (isFileEvent(source, data)) return 'file';
  
  // Check for process events
  if (isProcessEvent(source, data)) return 'process';
  
  // Default to SSL for other SSL-related events
  if (source.toLowerCase().includes('ssl') || source === 'http_parser') return 'ssl';
  
  return 'ssl';
}

function isPromptEvent(source: string, data: any): boolean {
  // Check if this is an HTTP request to AI API endpoints
  if (data.method === 'POST' && data.message_type === 'request') {
    const path = data.path || '';
    const host = data.host || data.headers?.host || '';
    
    // Common AI API endpoints
    const aiEndpoints = [
      'api.openai.com',
      'api.anthropic.com',
      'api.claude.ai',
      'api.gemini.google.com',
      'chat.googleapis.com'
    ];
    
    const aiPaths = [
      '/v1/chat/completions',
      '/v1/completions',
      '/v1/messages',
      '/chat/completions'
    ];
    
    if (aiEndpoints.some(endpoint => host.includes(endpoint)) || 
        aiPaths.some(apiPath => path.includes(apiPath))) {
      return true;
    }
    
    // Check body content for AI-related data
    if (data.body) {
      try {
        const bodyData = typeof data.body === 'string' ? JSON.parse(data.body) : data.body;
        return !!(bodyData.model || bodyData.messages || bodyData.prompt);
      } catch {
        return false;
      }
    }
  }
  
  return false;
}

function isResponseEvent(source: string, data: any): boolean {
  // Check for SSE processor events (streaming responses)
  if (source === 'sse_processor' && data.sse_events) {
    return true;
  }
  
  // Check for HTTP responses from AI APIs
  if (data.message_type === 'response' && data.status_code) {
    const host = data.host || data.headers?.host || '';
    const aiEndpoints = [
      'api.openai.com',
      'api.anthropic.com',
      'api.claude.ai',
      'api.gemini.google.com'
    ];
    
    return aiEndpoints.some(endpoint => host.includes(endpoint));
  }
  
  return false;
}

function isFileEvent(source: string, data: any): boolean {
  return source === 'file' || 
         (data.fd !== undefined) ||
         (data.operation && ['open', 'read', 'write', 'close'].includes(data.operation));
}

function isProcessEvent(source: string, data: any): boolean {
  return source === 'process' || 
         (data.exec !== undefined) ||
         (data.exit !== undefined) ||
         (data.ppid !== undefined);
}

function parsePromptEvent(event: Event): ParsedEvent {
  const { data } = event;
  let promptData: PromptData = {};
  let title = 'AI Prompt';
  let content = '';
  
  try {
    if (data.body) {
      promptData = typeof data.body === 'string' ? JSON.parse(data.body) : data.body;
    }
    
    // Extract model and method info
    const model = promptData.model || 'Unknown Model';
    const method = data.method || 'POST';
    const path = data.path || '';
    
    title = `${method} ${model}`;
    
    // Extract user message content
    if (promptData.messages && promptData.messages.length > 0) {
      const userMessages = promptData.messages
        .filter(msg => msg.role === 'user')
        .map(msg => msg.content)
        .join('\n');
      content = userMessages || 'No user message found';
    } else if (promptData.system && promptData.system.length > 0) {
      content = promptData.system.map(s => s.text || s).join('\n');
    } else {
      content = JSON.stringify(promptData, null, 2);
    }
  } catch (error) {
    content = typeof data.body === 'string' ? data.body : JSON.stringify(data);
  }
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'prompt',
    title,
    content,
    metadata: {
      model: promptData.model,
      temperature: promptData.temperature,
      max_tokens: promptData.max_tokens,
      url: `${data.host || ''}${data.path || ''}`,
      method: data.method,
      headers: data.headers
    },
    isExpanded: false
  };
}

function parseResponseEvent(event: Event): ParsedEvent {
  const { data } = event;
  let responseData: ResponseData = data;
  let title = 'AI Response';
  let content = '';
  
  try {
    // Extract response content from SSE events
    if (data.sse_events && Array.isArray(data.sse_events)) {
      const textParts: string[] = [];
      
      data.sse_events.forEach((sseEvent: any) => {
        if (sseEvent.parsed_data) {
          // Handle different SSE event types
          if (sseEvent.parsed_data.type === 'content_block_delta' && sseEvent.parsed_data.delta?.text) {
            textParts.push(sseEvent.parsed_data.delta.text);
          } else if (sseEvent.parsed_data.message?.content) {
            // Handle complete message content
            const msgContent = sseEvent.parsed_data.message.content;
            if (Array.isArray(msgContent)) {
              msgContent.forEach(item => {
                if (item.text) textParts.push(item.text);
              });
            }
          }
        }
      });
      
      content = textParts.join('') || data.text_content || 'Response received';
      
      // Get model info from message_start event
      const messageStart = data.sse_events.find((e: any) => e.event === 'message_start');
      if (messageStart?.parsed_data?.message?.model) {
        title = `Response from ${messageStart.parsed_data.message.model}`;
      }
    } else if (data.text_content) {
      content = data.text_content;
    } else {
      content = JSON.stringify(data, null, 2);
    }
  } catch (error) {
    content = JSON.stringify(data, null, 2);
  }
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'response',
    title,
    content,
    metadata: {
      message_id: data.message_id,
      duration_ns: data.duration_ns,
      event_count: data.event_count,
      model: responseData.model,
      usage: responseData.usage
    },
    isExpanded: false
  };
}

function parseSSLEvent(event: Event): ParsedEvent {
  const { data } = event;
  const sslData: SSLData = data;
  
  const method = sslData.method || 'UNKNOWN';
  const path = sslData.path || '/';
  const host = sslData.host || sslData.headers?.host || 'unknown';
  const statusCode = sslData.status_code;
  
  let title = `${method} ${host}${path}`;
  if (statusCode) {
    title += ` (${statusCode})`;
  }
  
  const content = sslData.body || JSON.stringify(data, null, 2);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title,
    content,
    metadata: {
      method: sslData.method,
      path: sslData.path,
      host: host,
      status_code: sslData.status_code,
      content_length: sslData.content_length,
      headers: sslData.headers,
      message_type: sslData.message_type
    },
    isExpanded: false
  };
}

function parseFileEvent(event: Event): ParsedEvent {
  const { data } = event;
  const fileData: FileData = data;
  
  const operation = fileData.operation || 'file operation';
  const path = fileData.path || 'unknown path';
  
  const title = `${operation} ${path}`;
  const content = JSON.stringify(data, null, 2);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'file',
    title,
    content,
    metadata: {
      operation: fileData.operation,
      path: fileData.path,
      size: fileData.size,
      fd: fileData.fd,
      permissions: fileData.permissions
    },
    isExpanded: false
  };
}

function parseProcessEvent(event: Event): ParsedEvent {
  const { data } = event;
  
  let title = 'Process Event';
  if (data.exec) {
    title = `exec: ${data.exec}`;
  } else if (data.exit) {
    title = `exit: code ${data.exit}`;
  } else if (data.ppid) {
    title = `child of PID ${data.ppid}`;
  }
  
  const content = JSON.stringify(data, null, 2);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'process',
    title,
    content,
    metadata: data,
    isExpanded: false
  };
}

function parseGenericEvent(event: Event): ParsedEvent {
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: 'ssl',
    title: `${event.source} event`,
    content: JSON.stringify(event.data, null, 2),
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