import { invoke } from "@tauri-apps/api/core";
import type {
  Command,
  MutationReceipt,
  Page,
  PagedTasks,
  RuntimeStatus,
  SearchHit,
  TaskQuery,
} from "./types";

export function openCore(profilePath: string): Promise<void> {
  return invoke("open_core", { profilePath });
}

export function dispatch(command: Command): Promise<MutationReceipt> {
  return invoke("dispatch", { command });
}

export function listTasks(query: TaskQuery, page: Page, today: string): Promise<PagedTasks> {
  return invoke("list_tasks", { query, page, today });
}

export function search(text: string, limit: number): Promise<SearchHit[]> {
  return invoke("search", { text, limit });
}

export function runtimeStatus(): Promise<RuntimeStatus> {
  return invoke("runtime_status");
}

export function closeCore(): Promise<void> {
  return invoke("close_core");
}

/** Local date as `YYYY-MM-DD` (used by smart-list scopes like Today/Next7). */
export function todayStr(): string {
  const d = new Date();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}
