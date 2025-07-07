export type QueueApi = QueueItem[];

export type QueueItem = {
  job_id: string;
  current: boolean;
  title: string;
  url: string;
  channel: string;
  uploader_id: string;
};

export type VideoType = "merged" | "split";
