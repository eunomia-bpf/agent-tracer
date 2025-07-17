# AgentSight Frontend

A modern React/TypeScript web interface for visualizing AI agent observability data from the AgentSight collector. Features interactive timeline views, process trees, and real-time event streaming.

## Overview

The AgentSight Frontend provides intuitive visualization of:

- **SSL/TLS Traffic**: HTTP requests, responses, and encrypted communications
- **Process Events**: Lifecycle tracking, file operations, and system calls
- **Agent Behavior**: Combined view of agent interactions and system activity
- **Real-time Data**: Live streaming and analysis of agent operations

## Features

### Timeline View
- Interactive event timeline with zoom and pan controls
- Event grouping by type, source, and process
- Real-time filtering and search capabilities
- Minimap for navigation across large datasets
- Export functionality for analysis

### Process Tree View
- Hierarchical visualization of process relationships
- Lifecycle tracking (fork, exec, exit events)
- Resource usage monitoring and metrics
- Parent-child relationship mapping
- Interactive node expansion and filtering

### Log View
- Raw event inspection with JSON formatting
- Syntax highlighting and pretty printing
- Advanced filtering and search
- Error detection and validation
- Export capabilities (JSON, CSV)

## Technology Stack

- **Framework**: Next.js 15+ with React 18+
- **Language**: TypeScript 5+ with strict typing
- **Styling**: Tailwind CSS with responsive design
- **State Management**: React hooks and context
- **Data Processing**: Real-time log parsing and event correlation

## Quick Start

### Prerequisites

- **Node.js**: 18+ or 20+ (recommended)
- **npm**: 9+ or yarn/pnpm alternative
- **AgentSight Collector**: Running for data source

### Installation

```bash
# Clone repository
git clone https://github.com/eunomia-bpf/agentsight.git --recursive
cd agentsight/frontend

# Install dependencies
npm install

# Start development server
npm run dev

# Open browser
open http://localhost:3000
```

### Production Build

```bash
# Build for production
npm run build

# Start production server
npm start

# Or serve via collector (recommended)
cd ../collector && cargo run server
# Access at http://localhost:8080
```

## Usage

### Data Loading

The frontend supports multiple data input methods:

#### 1. Real-time Streaming (Recommended)
```bash
# Start collector with server mode
cd ../collector && cargo run server

# Access web interface with live data
open http://localhost:8080/timeline
```

#### 2. File Upload
- Click "Upload Log" button
- Select AgentSight log files
- Automatic parsing and visualization

#### 3. Text Paste
- Paste JSON event logs directly
- Real-time parsing and validation
- Error detection and correction

### Navigation

#### Timeline View
- **Zoom**: Mouse wheel or zoom controls
- **Pan**: Click and drag timeline
- **Filter**: Use search bar and filter controls
- **Select**: Click events for detailed inspection

#### Process Tree
- **Expand/Collapse**: Click process nodes
- **Filter**: Process name, PID, or lifecycle events
- **Details**: Hover for quick info, click for full details

#### Controls
- **View Toggle**: Switch between Timeline, Process Tree, and Log views
- **Sync Data**: Manual refresh from collector
- **Clear Data**: Reset all loaded events
- **Export**: Download filtered data

## Configuration

### Environment Variables

```bash
# API endpoint for data syncing
NEXT_PUBLIC_API_URL=http://localhost:8080

# Enable debug mode
NEXT_PUBLIC_DEBUG=true

# Custom port for development
PORT=3000
```

### Build Configuration

See `next.config.js` for:
- Asset optimization
- Build output configuration
- Development vs production settings

## Development

### Project Structure

```
agentsight/frontend/
├── src/
│   ├── app/           # Next.js app directory
│   │   ├── page.tsx   # Main application page
│   │   └── layout.tsx # Application layout
│   ├── components/    # React components
│   │   ├── LogView.tsx       # Log inspection view
│   │   ├── TimelineView.tsx  # Interactive timeline
│   │   ├── ProcessTreeView.tsx # Process hierarchy
│   │   ├── UploadPanel.tsx   # File upload interface
│   │   ├── common/           # Shared components
│   │   ├── log/              # Log view components
│   │   ├── process-tree/     # Process tree components
│   │   └── timeline/         # Timeline components
│   ├── lib/           # Utility libraries
│   ├── types/         # TypeScript type definitions
│   └── utils/         # Helper functions
├── public/            # Static assets
├── package.json       # Dependencies and scripts
└── tailwind.config.ts # Styling configuration
```

### Key Components

#### EventFilters (`src/components/common/EventFilters.tsx`)
- Configurable filtering interface
- Search, type, source, and time range filters
- Real-time filter application

#### Timeline (`src/components/timeline/Timeline.tsx`)
- Core timeline visualization
- Zoom, pan, and selection handling
- Event rendering and interaction

#### ProcessNode (`src/components/process-tree/ProcessNode.tsx`)
- Individual process visualization
- Lifecycle state management
- Interactive expansion and details

### Development Commands

```bash
# Development server with hot reload
npm run dev

# Type checking
npm run type-check

# Linting and formatting
npm run lint
npm run lint:fix

# Build and test
npm run build
npm run start

# Clean build cache
rm -rf .next node_modules
npm install
```

### Adding New Features

#### 1. New Visualization Component
```typescript
// src/components/MyView.tsx
import { Event } from '@/types/event';

interface MyViewProps {
  events: Event[];
}

export function MyView({ events }: MyViewProps) {
  // Component implementation
}
```

#### 2. Event Type Support
```typescript
// src/types/event.ts
export interface MyEventData {
  // New event data structure
}

// src/utils/eventParsers.ts
export function parseMyEvent(data: any): MyEventData {
  // Parsing logic
}
```

#### 3. New Analyzer Integration
```typescript
// src/utils/eventProcessing.ts
export function processMyEvents(events: Event[]): ProcessedEvent[] {
  // Processing logic
}
```

## API Integration

### Data Endpoints

The frontend connects to the collector's REST API:

```typescript
// GET /api/events - Fetch all events
// GET /api/events/stream - Server-sent events stream
// POST /api/events - Upload event data
```

### Event Format

Events follow the standardized AgentSight format:

```typescript
interface Event {
  id: string;
  timestamp: number;
  source: string;
  pid: number;
  comm: string;
  data: Record<string, any>;
}
```

## Performance

### Optimization Features

- **Virtual Scrolling**: Handle large event datasets efficiently
- **Lazy Loading**: Load components and data on demand
- **Memoization**: Prevent unnecessary re-renders
- **Debounced Filtering**: Reduce computation during user input

### Memory Management

- **Event Pagination**: Limit in-memory event count
- **Data Cleanup**: Automatic cleanup of old events
- **Component Cleanup**: Proper cleanup on unmount

## Deployment

### Next.js Standalone

```bash
# Build standalone application
npm run build

# Deploy dist/ directory to web server
# Requires Node.js runtime
```

### Embedded in Collector

```bash
# Build frontend for embedding
npm run build

# Collector automatically serves at /timeline
cd ../collector && cargo run server
```

### Container Deployment

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build
EXPOSE 3000
CMD ["npm", "start"]
```

## Troubleshooting

### Common Issues

#### 1. Data Not Loading
- Verify collector is running and accessible
- Check browser console for network errors
- Ensure CORS is configured correctly

#### 2. Performance Issues
- Reduce event count with filters
- Check for memory leaks in browser dev tools
- Consider pagination for large datasets

#### 3. Build Errors
- Clear Next.js cache: `rm -rf .next`
- Update dependencies: `npm update`
- Check TypeScript errors: `npm run type-check`

### Debug Mode

```bash
# Enable debug logging
NEXT_PUBLIC_DEBUG=true npm run dev

# Check browser console for detailed logs
# Network tab for API communication
```

## Browser Support

- **Chrome**: 90+ (recommended)
- **Firefox**: 88+
- **Safari**: 14+
- **Edge**: 90+

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Follow TypeScript and React best practices
4. Add tests for new functionality
5. Ensure all linting passes: `npm run lint`
6. Submit a pull request

### Code Style

- **TypeScript**: Strict mode with explicit types
- **Components**: Functional components with hooks
- **Styling**: Tailwind CSS with consistent patterns
- **Testing**: Jest and React Testing Library

## License

MIT License - see [LICENSE](../LICENSE) for details.

## Related Projects

- **AgentSight Collector**: Data collection and analysis (`../collector/`)
- **Analysis Tools**: Python utilities for data processing (`../script/`)
- **Documentation**: Comprehensive guides and examples (`../docs/`)