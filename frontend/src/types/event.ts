export interface Event {
  id: string;
  timestamp: number;
  source: string;
  pid: number;
  comm: string;
  data: any;
}

export interface GroupedEvents {
  [source: string]: Event[];
}

export interface ProcessedEvent extends Event {
  datetime: Date;
  formattedTime: string;
  sourceColor: string;
  sourceColorClass: string;
}