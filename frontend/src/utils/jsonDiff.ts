// JSON Diff utility for comparing AI prompts
import { diffLines, Change } from 'diff';

export interface JsonDiff {
  added: any[];
  removed: any[];
  modified: { path: string; oldValue: any; newValue: any }[];
  unchanged: number;
}

export interface DiffResult {
  type: 'added' | 'removed' | 'modified' | 'unchanged';
  path?: string;
  value?: any;
  oldValue?: any;
  newValue?: any;
  content?: string;
}

// Flatten JSON object into path-value pairs
function flattenJson(obj: any, prefix = ''): Map<string, any> {
  const flattened = new Map<string, any>();
  
  function flatten(current: any, path: string) {
    if (current === null || current === undefined) {
      flattened.set(path, current);
      return;
    }
    
    if (typeof current !== 'object' || current instanceof Date) {
      flattened.set(path, current);
      return;
    }
    
    if (Array.isArray(current)) {
      current.forEach((item, index) => {
        flatten(item, path ? `${path}[${index}]` : `[${index}]`);
      });
    } else {
      Object.keys(current).forEach(key => {
        const newPath = path ? `${path}.${key}` : key;
        flatten(current[key], newPath);
      });
    }
  }
  
  flatten(obj, prefix);
  return flattened;
}

// Compare two JSON objects and return differences
export function compareJson(oldObj: any, newObj: any): JsonDiff {
  const oldFlat = flattenJson(oldObj);
  const newFlat = flattenJson(newObj);
  
  const result: JsonDiff = {
    added: [],
    removed: [],
    modified: [],
    unchanged: 0
  };
  
  // Check for removed and modified
  oldFlat.forEach((value, key) => {
    if (!newFlat.has(key)) {
      result.removed.push({ path: key, value });
    } else {
      const newValue = newFlat.get(key);
      if (JSON.stringify(value) !== JSON.stringify(newValue)) {
        result.modified.push({ path: key, oldValue: value, newValue });
      } else {
        result.unchanged++;
      }
    }
  });
  
  // Check for added
  newFlat.forEach((value, key) => {
    if (!oldFlat.has(key)) {
      result.added.push({ path: key, value });
    }
  });
  
  return result;
}

// Format JSON for text-based diff
export function formatJsonForDiff(obj: any): string {
  if (!obj) return '';
  
  // Special handling for AI prompt messages
  if (obj.messages && Array.isArray(obj.messages)) {
    return obj.messages.map((msg: any, idx: number) => {
      const role = msg.role || 'unknown';
      const content = msg.content;
      
      if (Array.isArray(content)) {
        const textContent = content
          .filter(c => c.type === 'text')
          .map(c => c.text)
          .join('\n');
        return `[${idx}] ${role.toUpperCase()}:\n${textContent}`;
      } else if (typeof content === 'string') {
        return `[${idx}] ${role.toUpperCase()}:\n${content}`;
      }
      return `[${idx}] ${role.toUpperCase()}: ${JSON.stringify(content)}`;
    }).join('\n\n---\n\n');
  }
  
  return JSON.stringify(obj, null, 2);
}

// Generate text-based diff similar to git diff
export function generateTextDiff(oldObj: any, newObj: any): string {
  const oldText = formatJsonForDiff(oldObj);
  const newText = formatJsonForDiff(newObj);
  
  const changes = diffLines(oldText, newText);
  
  let result = '';
  let lineNum = 1;
  let newLineNum = 1;
  
  changes.forEach((change: Change) => {
    const lines = change.value.split('\n').filter(line => line !== '');
    
    if (change.added) {
      lines.forEach(line => {
        result += `+ ${line}\n`;
        newLineNum++;
      });
    } else if (change.removed) {
      lines.forEach(line => {
        result += `- ${line}\n`;
        lineNum++;
      });
    } else {
      // Context lines
      lines.slice(0, 3).forEach(line => {
        result += `  ${line}\n`;
        lineNum++;
        newLineNum++;
      });
      
      if (lines.length > 6) {
        result += `  ...\n`;
        const skipCount = lines.length - 6;
        lineNum += skipCount;
        newLineNum += skipCount;
      }
      
      if (lines.length > 3) {
        lines.slice(-3).forEach(line => {
          result += `  ${line}\n`;
          lineNum++;
          newLineNum++;
        });
      }
    }
  });
  
  return result;
}

// Extract prompt content from AI request data
export function extractPromptContent(data: any): any {
  // If data has a body field that's a string (http_parser events)
  if (data.body && typeof data.body === 'string') {
    try {
      return JSON.parse(data.body);
    } catch (e) {
      return null;
    }
  }
  
  // If data already has the prompt structure
  if (data.messages || data.prompt) {
    return data;
  }
  
  return null;
}

// Compare two AI prompts and generate a summary
export function comparePrompts(oldPrompt: any, newPrompt: any): {
  diff: string;
  summary: string;
  hasChanges: boolean;
} {
  const oldContent = extractPromptContent(oldPrompt);
  const newContent = extractPromptContent(newPrompt);
  
  if (!oldContent || !newContent) {
    return {
      diff: 'Unable to extract prompt content',
      summary: 'Unable to compare prompts',
      hasChanges: false
    };
  }
  
  const diff = generateTextDiff(oldContent, newContent);
  const jsonDiff = compareJson(oldContent, newContent);
  
  let summary = '';
  if (jsonDiff.added.length > 0) {
    summary += `Added ${jsonDiff.added.length} fields. `;
  }
  if (jsonDiff.removed.length > 0) {
    summary += `Removed ${jsonDiff.removed.length} fields. `;
  }
  if (jsonDiff.modified.length > 0) {
    summary += `Modified ${jsonDiff.modified.length} fields. `;
  }
  
  if (summary === '') {
    summary = 'No changes detected';
  }
  
  return {
    diff,
    summary,
    hasChanges: jsonDiff.added.length > 0 || jsonDiff.removed.length > 0 || jsonDiff.modified.length > 0
  };
}