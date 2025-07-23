# Auto-Detection Plan for General Prompt Templates

## Executive Summary

This document outlines a comprehensive plan for automatically detecting and extracting general prompt templates from AI agent interactions captured by the AgentSight framework. The approach focuses on pattern recognition, structural analysis, and diff-based learning to identify reusable prompt templates across different AI models and use cases.

## Current State Analysis

### Data Structure
Based on the analyzed code, the system captures AI interactions with the following key components:

1. **Prompt Events** (`eventParsers.ts:262-327`)
   - Model identification
   - System messages with potential cache control
   - Message arrays with role-based structure
   - Parameters (temperature, max_tokens, stream)
   - Metadata including user_id

2. **Response Events** (`eventParsers.ts:329-394`)
   - SSE event streams with content deltas
   - Model information
   - Usage statistics (tokens)
   - Message IDs and timing data

3. **Event Processing Pipeline**
   - Raw data extraction via `DataExtractor` class
   - Type determination based on source and data patterns
   - Structured parsing into `ParsedEvent` format
   - Timeline-based organization by process

## Proposed Auto-Detection Approach

### 1. Template Pattern Recognition

#### A. Structural Analysis
```typescript
interface PromptTemplate {
  id: string;
  name: string;
  modelPattern: string;  // Regex or exact match
  structure: {
    systemPrompt?: TemplateSection;
    messageFlow: MessagePattern[];
    parameters: ParameterPattern;
  };
  frequency: number;
  variations: TemplateVariation[];
  metadata: {
    firstSeen: number;
    lastSeen: number;
    applications: string[];
  };
}

interface TemplateSection {
  pattern: string;  // Template with placeholders
  variables: Variable[];
  isOptional: boolean;
}

interface MessagePattern {
  role: 'user' | 'assistant' | 'system';
  contentPattern: string;
  position: 'start' | 'middle' | 'end' | 'any';
  minOccurrences: number;
  maxOccurrences: number;
}
```

#### B. Detection Algorithm
1. **Parse Historical Data**: Extract all prompt events from logs
2. **Normalize Messages**: Remove specific content, keep structure
3. **Cluster Similar Prompts**: Group by:
   - Model type
   - System prompt similarity (using embedding or fuzzy matching)
   - Message flow patterns
   - Parameter ranges
4. **Extract Common Patterns**: Identify recurring structures
5. **Generate Templates**: Create abstracted templates with variables

### 2. Diff-Based Learning

#### A. Change Tracking System
```typescript
interface PromptDiff {
  templateId: string;
  timestamp: number;
  changes: {
    type: 'addition' | 'deletion' | 'modification';
    path: string;  // JSONPath to changed element
    before?: any;
    after?: any;
  }[];
  context: {
    previousPromptId: string;
    nextPromptId: string;
    sessionId: string;
  };
}
```

#### B. Implementation Strategy
1. **Session Tracking**: Group prompts by session/conversation
2. **Sequential Analysis**: Compare consecutive prompts
3. **Change Classification**:
   - Content changes (variable substitution)
   - Structural changes (adding/removing messages)
   - Parameter tuning
4. **Pattern Extraction**: Identify which parts change vs. stay constant

### 3. Template Extraction Pipeline

#### Phase 1: Data Collection
```typescript
class TemplateDetector {
  private prompts: Map<string, ParsedPrompt[]> = new Map();
  
  async collectPrompts(events: ParsedEvent[]): Promise<void> {
    // Filter and organize prompt events
    // Group by model and time windows
    // Store normalized versions
  }
}
```

#### Phase 2: Pattern Analysis
```typescript
class PatternAnalyzer {
  async analyzePatterns(prompts: ParsedPrompt[]): Promise<TemplatePattern[]> {
    // 1. Tokenize and normalize prompts
    // 2. Calculate similarity matrices
    // 3. Perform hierarchical clustering
    // 4. Extract centroid patterns
    // 5. Identify variable regions
  }
}
```

#### Phase 3: Template Generation
```typescript
class TemplateGenerator {
  generateTemplate(pattern: TemplatePattern): PromptTemplate {
    // 1. Replace variables with placeholders
    // 2. Create parameter constraints
    // 3. Generate documentation
    // 4. Validate against original prompts
  }
}
```

### 4. Implementation Components

#### A. Frontend Enhancements
1. **Template Viewer**:
   - Display detected templates
   - Show usage frequency and variations
   - Allow manual refinement

2. **Diff Visualization**:
   - Side-by-side prompt comparison
   - Highlight changes between iterations
   - Track template evolution

3. **Template Library**:
   - Searchable template catalog
   - Export/import functionality
   - Version control integration

#### B. Backend Services
1. **Template Detection Service**:
   ```rust
   pub struct TemplateDetector {
       similarity_threshold: f32,
       min_occurrences: usize,
       analyzer: Box<dyn Analyzer>,
   }
   ```

2. **Diff Engine**:
   ```rust
   pub struct DiffEngine {
       diff_algorithm: DiffAlgorithm,
       context_lines: usize,
   }
   ```

3. **Storage Layer**:
   - Template database schema
   - Efficient similarity search
   - Version tracking

### 5. Detection Strategies

#### A. Similarity Metrics
1. **Structural Similarity**:
   - Jaccard similarity for message structure
   - Tree edit distance for nested content
   - Parameter range overlap

2. **Semantic Similarity**:
   - Embedding-based comparison for system prompts
   - Topic modeling for content classification
   - Intent classification

3. **Temporal Patterns**:
   - Frequency analysis
   - Time-based clustering
   - Session continuity

#### B. Variable Detection
1. **Placeholder Recognition**:
   - Identify changing content regions
   - Extract variable patterns (e.g., `{user_name}`, `{{context}}`)
   - Detect format strings and templates

2. **Type Inference**:
   - Analyze variable content types
   - Identify constraints and validations
   - Generate type definitions

### 6. Advanced Features

#### A. Multi-Model Support
- Detect cross-model template adaptations
- Track model-specific optimizations
- Generate compatibility matrices

#### B. Template Evolution Tracking
- Version control for templates
- Change impact analysis
- A/B testing support

#### C. Smart Suggestions
- Recommend templates based on context
- Suggest optimizations
- Predict parameter values

### 7. Implementation Roadmap

#### Phase 1: Basic Detection (Weeks 1-2)
- [ ] Implement prompt normalization
- [ ] Basic clustering algorithm
- [ ] Simple template extraction
- [ ] CLI tool for analysis

#### Phase 2: Diff Engine (Weeks 3-4)
- [ ] Sequential prompt comparison
- [ ] Change classification
- [ ] Diff visualization UI
- [ ] Session tracking

#### Phase 3: Advanced Analysis (Weeks 5-6)
- [ ] Semantic similarity integration
- [ ] Variable type inference
- [ ] Template validation
- [ ] Performance optimization

#### Phase 4: UI Integration (Weeks 7-8)
- [ ] Template library frontend
- [ ] Interactive template editor
- [ ] Export/import functionality
- [ ] Documentation generation

### 8. Technical Considerations

#### Performance
- Use incremental processing for real-time detection
- Implement caching for similarity calculations
- Optimize storage with compression

#### Scalability
- Design for distributed processing
- Support streaming analysis
- Implement data retention policies

#### Privacy
- Ensure sensitive data is masked
- Support configurable redaction rules
- Implement access controls

### 9. Success Metrics

1. **Detection Accuracy**:
   - Template precision/recall
   - Variable extraction accuracy
   - False positive rate

2. **Performance Metrics**:
   - Processing latency
   - Memory usage
   - Storage efficiency

3. **User Metrics**:
   - Template reuse rate
   - Time saved per prompt
   - User satisfaction scores

### 10. Future Enhancements

1. **Machine Learning Integration**:
   - Train models on detected patterns
   - Predict optimal templates
   - Auto-generate new templates

2. **Cross-Organization Learning**:
   - Anonymous template sharing
   - Industry-specific template libraries
   - Best practice recommendations

3. **Integration Ecosystem**:
   - IDE plugins for template insertion
   - API for template management
   - CI/CD pipeline integration

## Conclusion

This plan provides a comprehensive approach to automatically detecting and managing prompt templates in the AgentSight system. By combining structural analysis, diff-based learning, and advanced pattern recognition, we can build a powerful system that helps users understand, optimize, and reuse their AI prompts effectively.

The modular design allows for incremental implementation while maintaining flexibility for future enhancements. The focus on both technical robustness and user experience ensures that the solution will provide immediate value while scaling to meet future needs.