import {
  Box,
  Button,
  Checkbox,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import DeleteIcon from "@mui/icons-material/Delete";
import { listLabel } from "../lists";
import { useTodoStore } from "../store";

export default function TaskList() {
  const tasks = useTodoStore((s) => s.tasks);
  const currentList = useTodoStore((s) => s.currentList);
  const dispatch = useTodoStore((s) => s.dispatch);

  const toggle = (id: string, completed: boolean) => {
    void dispatch({ type: "setTaskCompleted", task: id, completed: !completed });
  };

  const remove = (id: string) => {
    void dispatch({ type: "deleteTask", task: id });
  };

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", mb: 2 }}>
        <Typography variant="h5">{listLabel(currentList)}</Typography>
        <Button variant="contained" startIcon={<AddIcon />}>
          新建任务
        </Button>
      </Box>

      {tasks.length === 0 ? (
        <Typography color="text.secondary" sx={{ mt: 2 }}>
          暂无任务
        </Typography>
      ) : (
        <List disablePadding>
          {tasks.map((t) => (
            <ListItem
              key={t.id}
              disablePadding
              secondaryAction={
                <IconButton edge="end" aria-label="删除" onClick={() => remove(t.id)}>
                  <DeleteIcon />
                </IconButton>
              }
            >
              <ListItemButton onClick={() => toggle(t.id, t.completed)}>
                <Checkbox edge="start" checked={t.completed} tabIndex={-1} disableRipple />
                <ListItemText
                  primary={t.title}
                  sx={{ textDecoration: t.completed ? "line-through" : "none" }}
                />
              </ListItemButton>
            </ListItem>
          ))}
        </List>
      )}
    </Box>
  );
}
