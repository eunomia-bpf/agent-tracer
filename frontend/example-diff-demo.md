# AI Prompt Diff Demo

## How it works:

1. **Tracking**: The system tracks AI prompts per process (by PID)
2. **Comparison**: When a new prompt is detected from the same process, it's compared with the previous prompt
3. **Diff Display**: Changes are shown using a git-diff style format

## Visual Changes:

### First Prompt (No Previous):
- Color: Blue gradient (normal)
- Tags: "AI PROMPT", model name, method
- Border: Blue

### Subsequent Prompts with Changes:
- Color: Yellow/Orange gradient (indicates changes)
- Tags: "AI PROMPT", model name, method, **"CHANGED"**
- Border: Yellow
- Fold Content: Shows diff summary (e.g., "📝 Added 2 fields. Modified 3 fields.")
- Expanded Content: Shows both the diff and full content

## Diff Format Example:

```
=== CHANGES FROM PREVIOUS PROMPT ===
- [0] USER:
- What is 2+2?
+ [0] USER:
+ What is 2+2?
+ 
+ [1] ASSISTANT:
+ 2 + 2 = 4
+ 
+ [2] USER:
+ What is 3+3?

=== FULL CONTENT ===
{
  "model": "claude-3-5-haiku",
  "messages": [
    {"role": "user", "content": "What is 2+2?"},
    {"role": "assistant", "content": "2 + 2 = 4"},
    {"role": "user", "content": "What is 3+3?"}
  ]
}
```

## Features:

1. **Git-style diff**: Shows additions (+) and removals (-) clearly
2. **Summary**: Quick overview of changes in collapsed view
3. **Visual indicators**: Different colors for changed vs unchanged prompts
4. **Context preservation**: Shows surrounding context for changes
5. **Memory efficient**: Only keeps last 10 prompts per process

This helps track how AI conversations evolve over time within the same process!