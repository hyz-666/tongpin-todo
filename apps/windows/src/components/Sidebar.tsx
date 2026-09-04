import type { ReactNode } from "react";
import { Box, List, ListItemButton, ListItemIcon, ListItemText, Toolbar } from "@mui/material";
import InboxIcon from "@mui/icons-material/Inbox";
import TodayIcon from "@mui/icons-material/Today";
import EventIcon from "@mui/icons-material/Event";
import DateRangeIcon from "@mui/icons-material/DateRange";
import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import { SMART_LISTS } from "../lists";
import { useTodoStore } from "../store";

const ICONS: Record<string, ReactNode> = {
  inbox: <InboxIcon />,
  today: <TodayIcon />,
  tomorrow: <EventIcon />,
  next7: <DateRangeIcon />,
  completed: <CheckCircleIcon />,
};

export default function Sidebar() {
  const currentList = useTodoStore((s) => s.currentList);
  const setCurrentList = useTodoStore((s) => s.setCurrentList);
  const loadTasks = useTodoStore((s) => s.loadTasks);

  const select = (id: string) => {
    setCurrentList(id);
    void loadTasks(id);
  };

  return (
    <Box sx={{ width: 220, flexShrink: 0, borderRight: 1, borderColor: "divider" }}>
      <Toolbar sx={{ fontWeight: 500 }}>tongpin-todo</Toolbar>
      <List>
        {SMART_LISTS.map((l) => (
          <ListItemButton key={l.id} selected={currentList === l.id} onClick={() => select(l.id)}>
            <ListItemIcon>{ICONS[l.id]}</ListItemIcon>
            <ListItemText primary={l.label} />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}
