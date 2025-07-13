export interface LogEntry {
  level: string;
  time: string;
  message?: string;
  AppName?: string;
  HomePage?: string;
  Repository?: string;
  Author?: string;
  Description?: string;
  Version?: string;
  Listen?: string;
  logger?: string;
  eventCollector?: string;
  listen?: string;
  Pid?: number;
  'Kernel Info'?: string;
  TruncateSize?: number;
  Unit?: string;
  btfMode?: number;
  keylogger?: string;
  eBPFProgramType?: string;
  moduleName?: string;
  isReload?: boolean;
  error?: string;
  soPath?: string;
  imported?: string;
  [key: string]: any;
}

export interface TimelineEvent {
  id: string;
  timestamp: string;
  level: string;
  message: string;
  details?: Record<string, any>;
}

export function parseLogEntry(logLine: string): LogEntry | null {
  try {
    const entry = JSON.parse(logLine.trim());
    return entry as LogEntry;
  } catch (error) {
    console.warn('Failed to parse log line:', logLine, error);
    return null;
  }
}

export function convertLogToTimelineEvent(entry: LogEntry, index: number): TimelineEvent {
  const { level, time, message, ...details } = entry;
  
  // Generate a meaningful message from the log entry
  let eventMessage = message || 'System event';
  
  // Customize messages based on available data
  if (entry.AppName) {
    eventMessage = `${entry.AppName} application started`;
  } else if (entry.Repository) {
    eventMessage = 'Repository information loaded';
  } else if (entry.Listen || entry.listen) {
    eventMessage = `Server listening on ${entry.Listen || entry.listen}`;
  } else if (entry.moduleName) {
    eventMessage = `Module ${entry.moduleName} ${entry.isReload ? 'reloaded' : 'initialized'}`;
  } else if (entry.error) {
    eventMessage = entry.error;
  } else if (entry.soPath) {
    eventMessage = `Library detected: ${entry.soPath}`;
  } else if (entry['Kernel Info']) {
    eventMessage = `Kernel info detected: ${entry['Kernel Info']}`;
  } else if (entry.btfMode !== undefined) {
    eventMessage = 'BTF bytecode mode configured';
  } else if (entry.logger) {
    eventMessage = 'Logger initialized';
  } else if (entry.Version) {
    eventMessage = `Version: ${entry.Version}`;
  }

  // Remove fields that are already in the message
  const cleanDetails = { ...details };
  delete cleanDetails.time;
  delete cleanDetails.level;
  
  return {
    id: `event-${index}`,
    timestamp: time,
    level: level,
    message: eventMessage,
    details: Object.keys(cleanDetails).length > 0 ? cleanDetails : undefined
  };
}

export function parseLogFile(logContent: string): TimelineEvent[] {
  const lines = logContent.split('\n').filter(line => line.trim());
  const events: TimelineEvent[] = [];
  
  lines.forEach((line, index) => {
    const entry = parseLogEntry(line);
    if (entry) {
      events.push(convertLogToTimelineEvent(entry, index));
    }
  });
  
  return events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
} 