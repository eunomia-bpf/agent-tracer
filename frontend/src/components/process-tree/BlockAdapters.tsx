import { ParsedEvent } from '@/utils/eventParsers';
import { UnifiedBlockData } from './UnifiedBlock';
import { 
  SparklesIcon, 
  CheckCircleIcon, 
  DocumentIcon, 
  CpuChipIcon, 
  LockClosedIcon 
} from '@heroicons/react/24/outline';

// Simplified - no longer need these helper functions

export function adaptPromptEvent(event: ParsedEvent): UnifiedBlockData {
  const metadata = event.metadata || {};
  
  // Update tags to include diff info
  const tags = ['AI PROMPT', metadata.model, metadata.method].filter(Boolean);
  if (event.promptDiff?.hasChanges) {
    tags.push('CHANGED');
  }
  
  // Fold content: show diff summary if available
  let foldContent = event.content && event.content.length > 0 
    ? event.content.substring(0, 100) + (event.content.length > 100 ? '...' : '')
    : metadata.url || '';
    
  if (event.promptDiff?.summary) {
    foldContent = `📝 ${event.promptDiff.summary}`;
  }

  // Expanded content: include diff if available
  let expandedContent = event.content || JSON.stringify(event.metadata, null, 2);
  
  if (event.promptDiff?.diff) {
    expandedContent = `=== CHANGES FROM PREVIOUS PROMPT ===\n${event.promptDiff.diff}\n\n=== FULL CONTENT ===\n${expandedContent}`;
  }

  return {
    id: event.id,
    type: 'prompt',
    timestamp: event.timestamp,
    tags,
    bgGradient: event.promptDiff?.hasChanges 
      ? 'bg-gradient-to-r from-yellow-50 via-orange-50 to-red-50'
      : 'bg-gradient-to-r from-blue-50 via-purple-50 to-pink-50',
    borderColor: event.promptDiff?.hasChanges 
      ? 'border-yellow-400'
      : 'border-blue-400',
    iconColor: event.promptDiff?.hasChanges 
      ? 'text-yellow-600'
      : 'text-blue-600',
    icon: SparklesIcon,
    foldContent,
    expandedContent
  };
}

export function adaptResponseEvent(event: ParsedEvent): UnifiedBlockData {
  const metadata = event.metadata || {};
  
  // Fold content: short preview
  const foldContent = event.content && event.content.length > 0 
    ? event.content.substring(0, 100) + (event.content.length > 100 ? '...' : '')
    : '';

  // Expanded content: everything
  const expandedContent = event.content || JSON.stringify(event.metadata, null, 2);

  return {
    id: event.id,
    type: 'response',
    timestamp: event.timestamp,
    tags: ['AI RESPONSE', metadata.model].filter(Boolean),
    bgGradient: 'bg-gradient-to-r from-green-50 via-emerald-50 to-teal-50',
    borderColor: 'border-green-400',
    iconColor: 'text-green-600',
    icon: CheckCircleIcon,
    foldContent,
    expandedContent
  };
}

// Helper function to format file sizes
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export function adaptFileEvent(event: ParsedEvent): UnifiedBlockData {
  const metadata = event.metadata || {};
  
  const operation = metadata.operation || metadata.event || 'file';
  const filepath = metadata.path || metadata.filepath || '';
  
  // Color scheme based on operation type - matching old FileBlock
  const getOperationColors = (op: string) => {
    const lowerOp = op.toLowerCase();
    if (lowerOp.includes('read')) return 'text-blue-600';
    if (lowerOp.includes('write')) return 'text-green-600';
    if (lowerOp.includes('open')) return 'text-purple-600';
    if (lowerOp.includes('close')) return 'text-gray-600';
    if (lowerOp.includes('delete') || lowerOp.includes('unlink')) return 'text-red-600';
    return 'text-indigo-600';
  };

  // Build tags for header
  const tags = [operation.toUpperCase()];
  if (metadata.fd !== undefined) tags.push(`FD ${metadata.fd}`);
  if (metadata.size !== undefined) tags.push(formatFileSize(metadata.size));

  // Fold content: file path
  const foldContent = filepath;

  // Expanded content: everything
  const expandedContent = event.content || JSON.stringify(event.metadata, null, 2);

  return {
    id: event.id,
    type: 'file',
    timestamp: event.timestamp,
    tags,
    bgGradient: 'bg-gradient-to-r from-cyan-50 via-sky-50 to-blue-50',
    borderColor: 'border-cyan-400',
    iconColor: getOperationColors(operation),
    icon: DocumentIcon,
    foldContent,
    expandedContent
  };
}

export function adaptProcessEvent(event: ParsedEvent): UnifiedBlockData {
  const metadata = event.metadata || {};
  
  const eventType = metadata.event || 'process';
  const comm = metadata.comm || '';
  const pid = metadata.pid || '';

  // Styling based on event type
  const getProcessColors = (eventType: string) => {
    const lowerEvent = eventType.toLowerCase();
    if (lowerEvent.includes('exec')) return { 
      icon: 'text-green-700',
      gradient: 'bg-gradient-to-r from-green-50 via-emerald-50 to-teal-50',
      border: 'border-green-400'
    };
    if (lowerEvent.includes('exit')) return { 
      icon: 'text-red-700',
      gradient: 'bg-gradient-to-r from-red-50 via-rose-50 to-pink-50',
      border: 'border-red-400'
    };
    return { 
      icon: 'text-gray-700',
      gradient: 'bg-gradient-to-r from-gray-50 via-slate-50 to-zinc-50',
      border: 'border-gray-400'
    };
  };

  const colors = getProcessColors(eventType);
  const tags = [eventType.toUpperCase()];
  if (pid) tags.push(`PID ${pid}`);

  // Fold content: command and PID
  const foldContent = comm && pid ? `${comm} (PID: ${pid})` : comm || `PID: ${pid}`;

  // Expanded content: everything
  const expandedContent = event.content || JSON.stringify(event.metadata, null, 2);

  return {
    id: event.id,
    type: 'process',
    timestamp: event.timestamp,
    tags,
    bgGradient: colors.gradient,
    borderColor: colors.border,
    iconColor: colors.icon,
    icon: CpuChipIcon,
    foldContent,
    expandedContent
  };
}

export function adaptSSLEvent(event: ParsedEvent): UnifiedBlockData {
  const metadata = event.metadata || {};
  
  const direction = metadata.direction || '';
  const size = metadata.data_size || metadata.size || 0;
  const comm = metadata.comm || '';

  // Fold content: size and command
  const foldContent = comm ? `${size} bytes - ${comm}` : `${size} bytes`;

  // Expanded content: everything
  const expandedContent = event.content || JSON.stringify(event.metadata, null, 2);

  return {
    id: event.id,
    type: 'ssl',
    timestamp: event.timestamp,
    tags: ['SSL', direction.toUpperCase(), `${size} bytes`].filter(Boolean),
    bgGradient: 'bg-gradient-to-r from-orange-50 via-amber-50 to-yellow-50',
    borderColor: 'border-orange-400',
    iconColor: 'text-orange-600',
    icon: LockClosedIcon,
    foldContent,
    expandedContent
  };
}

// Main adapter function
export function adaptEventToUnifiedBlock(event: ParsedEvent): UnifiedBlockData {
  switch (event.type) {
    case 'prompt':
      return adaptPromptEvent(event);
    case 'response':
      return adaptResponseEvent(event);
    case 'file':
      return adaptFileEvent(event);
    case 'process':
      return adaptProcessEvent(event);
    case 'ssl':
    default:
      return adaptSSLEvent(event);
  }
}