import { useEffect, useState } from "react";
import {
  Box,
  Button,
  Checkbox,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  TextField,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import DeleteIcon from "@mui/icons-material/Delete";
import SearchIcon from "@mui/icons-material/Search";
import InputAdornment from "@mui/material/InputAdornment";
import { listLabel } from "../lists";
import { useTodoStore } from "../store";
import TaskEditDialog from "./TaskEditDialog";

export default function TaskList() {
  const tasks = useTodoStore((s) => s.tasks);
  const searchHits = useTodoStore((s) => s.searchHits);
  const currentList = useTodoStore((s) => s.currentList);
  const dispatch = useTodoStore((s) => s.dispatch);
  const search = useTodoStore((s) => s.search);

  const [searchText, setSearchText] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | undefined>(undefined);

  useEffect(() => {
    const t = setTimeout(() => void search(searchText), 300);
    return () => clearTimeout(t);
  }, [searchText, search]);

  const isSearching = searchText.trim().length > 0;

  const toggle = (id: string, completed: boolean) => {
    void dispatch({ type: "setTaskCompleted", task: id, completed: !completed });
  };

  const remove = (id: string) => {
    void dispatch({ type: "deleteTask", task: id });
  };

  const openCreate = () => {
    setEditingId(undefined);
    setDialogOpen(true);
  };

  const openEdit = (id: string) => {
    setEditingId(id);
    setDialogOpen(true);
  };

  const rows = isSearching
    ? searchHits.map((h) => ({ id: h.taskId, title: h.title, completed: false }))
    : tasks;

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", mb: 2 }}>
        <Typography variant="h5">{isSearching ? "搜索结果" : listLabel(currentList)}</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={openCreate}>
          新建任务
        </Button>
      </Box>

      <TextField
        fullWidth
        size="small"
        placeholder="搜索任务…"
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        sx={{ mb: 2 }}
        slotProps={{
          input: {
            startAdornment: (
              <InputAdornment position="start">
                <SearchIcon />
              </InputAdornment>
            ),
          },
        }}
      />

      {rows.length === 0 ? (
        <Typography color="text.secondary" sx={{ mt: 2 }}>
          {isSearching ? "无匹配结果" : "暂无任务"}
        </Typography>
      ) : (
        <List disablePadding>
          {rows.map((t) => (
            <ListItem
              key={t.id}
              disablePadding
              secondaryAction={
                <IconButton edge="end" aria-label="删除" onClick={() => remove(t.id)}>
                  <DeleteIcon />
                </IconButton>
              }
            >
              <ListItemButton onClick={() => openEdit(t.id)}>
                <Checkbox
                  edge="start"
                  checked={t.completed}
                  tabIndex={-1}
                  disableRipple
                  onClick={(e) => {
                    e.stopPropagation();
                    toggle(t.id, t.completed);
                  }}
                />
                <ListItemText
                  primary={t.title}
                  sx={{ textDecoration: t.completed ? "line-through" : "none" }}
                />
              </ListItemButton>
            </ListItem>
          ))}
        </List>
      )}

      <TaskEditDialog
        open={dialogOpen}
        taskId={editingId}
        onClose={() => setDialogOpen(false)}
      />
    </Box>
  );
}
