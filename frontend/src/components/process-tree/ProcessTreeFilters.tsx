'use client';

import { useState } from 'react';
import { FunnelIcon, XMarkIcon } from '@heroicons/react/24/outline';

export interface ProcessTreeFilters {
  eventTypes: string[];
  models: string[];
  sources: string[];
  commands: string[];
  searchText: string;
  timeRange: {
    start?: number;
    end?: number;
  };
}

interface ProcessTreeFiltersProps {
  filters: ProcessTreeFilters;
  onFiltersChange: (filters: ProcessTreeFilters) => void;
  availableOptions: {
    eventTypes: string[];
    models: string[];
    sources: string[];
    commands: string[];
  };
  totalEvents: number;
  filteredEvents: number;
}

export function ProcessTreeFiltersComponent({
  filters,
  onFiltersChange,
  availableOptions,
  totalEvents,
  filteredEvents
}: ProcessTreeFiltersProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const updateFilters = (key: keyof ProcessTreeFilters, value: any) => {
    onFiltersChange({
      ...filters,
      [key]: value
    });
  };

  const toggleArrayFilter = (key: 'eventTypes' | 'models' | 'sources' | 'commands', item: string) => {
    const currentArray = filters[key];
    const newArray = currentArray.includes(item)
      ? currentArray.filter(i => i !== item)
      : [...currentArray, item];
    updateFilters(key, newArray);
  };

  const clearAllFilters = () => {
    onFiltersChange({
      eventTypes: [],
      models: [],
      sources: [],
      commands: [],
      searchText: '',
      timeRange: {}
    });
  };

  const hasActiveFilters = 
    filters.eventTypes.length > 0 ||
    filters.models.length > 0 ||
    filters.sources.length > 0 ||
    filters.commands.length > 0 ||
    filters.searchText.length > 0 ||
    filters.timeRange.start ||
    filters.timeRange.end;

  return (
    <div className="bg-gray-50 border-b border-gray-200">
      {/* Filter Header */}
      <div className="px-4 py-3 flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center space-x-2 text-sm font-medium text-gray-700 hover:text-gray-900"
          >
            <FunnelIcon className="h-4 w-4" />
            <span>Filters</span>
            {hasActiveFilters && (
              <span className="bg-blue-100 text-blue-800 text-xs rounded-full px-2 py-1">
                Active
              </span>
            )}
          </button>
          
          {/* Quick Filter Presets */}
          <div className="flex items-center space-x-2">
            <button
              onClick={() => updateFilters('eventTypes', ['prompt', 'response'])}
              className="text-xs bg-purple-100 text-purple-800 px-2 py-1 rounded hover:bg-purple-200"
            >
              AI Only
            </button>
            <button
              onClick={() => updateFilters('eventTypes', ['file'])}
              className="text-xs bg-cyan-100 text-cyan-800 px-2 py-1 rounded hover:bg-cyan-200"
            >
              Files Only
            </button>
            <button
              onClick={() => updateFilters('eventTypes', ['process'])}
              className="text-xs bg-green-100 text-green-800 px-2 py-1 rounded hover:bg-green-200"
            >
              Processes Only
            </button>
          </div>
          
          <div className="text-sm text-gray-500">
            Showing {filteredEvents.toLocaleString()} of {totalEvents.toLocaleString()} events
          </div>
        </div>

        {hasActiveFilters && (
          <button
            onClick={clearAllFilters}
            className="text-sm text-gray-500 hover:text-gray-700 flex items-center space-x-1"
          >
            <XMarkIcon className="h-4 w-4" />
            <span>Clear all</span>
          </button>
        )}
      </div>

      {/* Expanded Filters */}
      {isExpanded && (
        <div className="px-4 pb-4 space-y-4">
          {/* Search */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Search
            </label>
            <input
              type="text"
              value={filters.searchText}
              onChange={(e) => updateFilters('searchText', e.target.value)}
              placeholder="Search in content, models, commands..."
              className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Event Types */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Event Types
              </label>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {availableOptions.eventTypes.map(type => (
                  <label key={type} className="flex items-center space-x-2 text-sm">
                    <input
                      type="checkbox"
                      checked={filters.eventTypes.includes(type)}
                      onChange={() => toggleArrayFilter('eventTypes', type)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <span className="capitalize text-gray-900">{type}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Models */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                AI Models
              </label>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {availableOptions.models.map(model => (
                  <label key={model} className="flex items-center space-x-2 text-sm">
                    <input
                      type="checkbox"
                      checked={filters.models.includes(model)}
                      onChange={() => toggleArrayFilter('models', model)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <span className="truncate text-gray-900">{model}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Sources */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Sources
              </label>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {availableOptions.sources.map(source => (
                  <label key={source} className="flex items-center space-x-2 text-sm">
                    <input
                      type="checkbox"
                      checked={filters.sources.includes(source)}
                      onChange={() => toggleArrayFilter('sources', source)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <span className="text-gray-900">{source}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Commands */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Commands
              </label>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {availableOptions.commands.map(command => (
                  <label key={command} className="flex items-center space-x-2 text-sm">
                    <input
                      type="checkbox"
                      checked={filters.commands.includes(command)}
                      onChange={() => toggleArrayFilter('commands', command)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <span className="font-mono text-xs text-gray-900">{command}</span>
                  </label>
                ))}
              </div>
            </div>
          </div>

          {/* Time Range */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Time Range
            </label>
            <div className="flex space-x-2 items-center">
              <input
                type="datetime-local"
                value={filters.timeRange.start ? new Date(filters.timeRange.start).toISOString().slice(0, 16) : ''}
                onChange={(e) => updateFilters('timeRange', {
                  ...filters.timeRange,
                  start: e.target.value ? new Date(e.target.value).getTime() : undefined
                })}
                className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <span className="text-sm text-gray-500">to</span>
              <input
                type="datetime-local"
                value={filters.timeRange.end ? new Date(filters.timeRange.end).toISOString().slice(0, 16) : ''}
                onChange={(e) => updateFilters('timeRange', {
                  ...filters.timeRange,
                  end: e.target.value ? new Date(e.target.value).getTime() : undefined
                })}
                className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}