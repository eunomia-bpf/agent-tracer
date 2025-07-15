export interface Event {
  id: string;
  timestamp: number;
  source: string;
  data: any;
}

export interface GroupedEvents {
  [source: string]: Event[];
}

export interface ProcessedEvent extends Event {
  datetime: Date;
  formattedTime: string;
  sourceColor: string;
}