// Smart-list definitions shared between the sidebar and the task list header.

export const SMART_LISTS = [
  { id: "inbox", label: "收件箱" },
  { id: "today", label: "今天" },
  { id: "tomorrow", label: "明天" },
  { id: "next7", label: "未来 7 天" },
  { id: "completed", label: "已完成" },
] as const;

export function listLabel(id: string): string {
  return SMART_LISTS.find((l) => l.id === id)?.label ?? id;
}
