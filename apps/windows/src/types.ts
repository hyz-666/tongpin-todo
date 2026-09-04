// Shared TypeScript types matching the Rust command-layer DTOs (camelCase).

export type Priority = "none" | "low" | "medium" | "high";

export type Command =
  | {
      type: "createTask";
      title: string;
      description: string;
      dueDate?: string | null;
      dueTime?: string | null;
      priority: string;
      listId?: string | null;
      tags: string[];
    }
  | { type: "setTaskField"; task: string; field: string; value: string }
  | { type: "setTaskCompleted"; task: string; completed: boolean }
  | { type: "deleteTask"; task: string }
  | { type: "restoreTask"; task: string }
  | { type: "createList"; name: string }
  | { type: "deleteList"; list: string }
  | { type: "createTag"; name: string }
  | { type: "setTaskTag"; task: string; tag: string; attached: boolean };

export interface TaskQuery {
  list: string;
  activeOnly: boolean;
}

export interface Page {
  cursor?: string | null;
  limit: number;
}

export interface TaskSummary {
  id: string;
  title: string;
  completed: boolean;
  dueDate?: string | null;
  priority: string;
  listId?: string | null;
}

export interface PagedTasks {
  items: TaskSummary[];
  nextCursor?: string | null;
}

export interface SearchHit {
  taskId: string;
  title: string;
}

export interface MutationReceipt {
  operationIds: string[];
  affectedEntities: string[];
  projectionRevision: number;
}

export interface TaskDetails {
  id: string;
  title: string;
  description: string;
  dueDate?: string | null;
  dueTime?: string | null;
  priority: string;
  completed: boolean;
  listId?: string | null;
  tags: string[];
}

export interface PeerStatus {
  deviceId: string;
  reachable: boolean;
}

export interface RuntimeStatus {
  replica: string;
  peers: PeerStatus[];
}
