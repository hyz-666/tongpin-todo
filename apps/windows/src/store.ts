import { create } from "zustand";
import * as api from "./api";
import type { Command, RuntimeStatus, SearchHit, TaskSummary } from "./types";

interface TodoState {
  profilePath: string;
  ready: boolean;
  error: string | null;
  currentList: string;
  tasks: TaskSummary[];
  searchHits: SearchHit[];
  runtime: RuntimeStatus | null;

  open: (profilePath: string) => Promise<void>;
  loadTasks: (list?: string) => Promise<void>;
  dispatch: (command: Command) => Promise<void>;
  search: (text: string) => Promise<void>;
  setCurrentList: (list: string) => void;
  refreshRuntime: () => Promise<void>;
  close: () => Promise<void>;
}

export const useTodoStore = create<TodoState>((set, get) => ({
  profilePath: "",
  ready: false,
  error: null,
  currentList: "inbox",
  tasks: [],
  searchHits: [],
  runtime: null,

  open: async (profilePath) => {
    try {
      await api.openCore(profilePath);
      set({ profilePath, ready: true, error: null });
      await get().loadTasks();
      await get().refreshRuntime();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadTasks: async (list) => {
    try {
      const scope = list ?? get().currentList;
      const result = await api.listTasks(
        { list: scope, activeOnly: true },
        { cursor: undefined, limit: 200 },
        api.todayStr(),
      );
      set({ tasks: result.items, currentList: scope, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  dispatch: async (command) => {
    try {
      await api.dispatch(command);
      await get().loadTasks();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  search: async (text) => {
    try {
      if (!text.trim()) {
        set({ searchHits: [], error: null });
        return;
      }
      const hits = await api.search(text, 50);
      set({ searchHits: hits, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setCurrentList: (list) => set({ currentList: list }),

  refreshRuntime: async () => {
    try {
      const runtime = await api.runtimeStatus();
      set({ runtime });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  close: async () => {
    try {
      await api.closeCore();
      set({ ready: false, tasks: [], searchHits: [], runtime: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
