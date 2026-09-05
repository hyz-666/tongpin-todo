import { Box } from "@mui/material";
import { useTodoStore } from "../store";
import type { SyncState } from "../types";

const STYLES: Record<SyncState, { color: string; label: string }> = {
  offline: { color: "#9e9e9e", label: "离线" },
  syncing: { color: "#f59e0b", label: "同步中" },
  connected: { color: "#2e7d32", label: "已连接" },
};

export default function SyncStatusIndicator() {
  const sync = useTodoStore((s) => s.sync);

  const state = sync?.state ?? "offline";
  const style = STYLES[state];
  const peers = sync?.peers.length ?? 0;
  const label = state === "connected" && peers > 0 ? `${style.label} (${peers})` : style.label;

  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 0.75,
        px: 1.5,
        py: 0.5,
        fontSize: 13,
        color: "text.secondary",
        borderBottom: 1,
        borderColor: "divider",
      }}
    >
      <Box
        sx={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          backgroundColor: style.color,
        }}
      />
      {label}
    </Box>
  );
}
