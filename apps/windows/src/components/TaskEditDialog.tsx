import { useEffect, useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  TextField,
} from "@mui/material";
import { taskDetails } from "../api";
import { useTodoStore } from "../store";

interface Props {
  open: boolean;
  taskId?: string;
  onClose: () => void;
}

export default function TaskEditDialog({ open, taskId, onClose }: Props) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState("none");
  const [dueDate, setDueDate] = useState("");

  const dispatchMany = useTodoStore((s) => s.dispatchMany);

  useEffect(() => {
    if (!open) return;
    if (taskId) {
      taskDetails(taskId).then((d) => {
        setTitle(d.title);
        setDescription(d.description);
        setPriority(d.priority);
        setDueDate(d.dueDate ?? "");
      });
    } else {
      setTitle("");
      setDescription("");
      setPriority("none");
      setDueDate("");
    }
  }, [open, taskId]);

  const save = async () => {
    const t = title.trim();
    if (!t) return;
    if (taskId) {
      await dispatchMany([
        { type: "setTaskField", task: taskId, field: "title", value: t },
        { type: "setTaskField", task: taskId, field: "description", value: description },
        { type: "setTaskField", task: taskId, field: "priority", value: priority },
        { type: "setTaskField", task: taskId, field: "due_date", value: dueDate },
      ]);
    } else {
      await dispatchMany([
        {
          type: "createTask",
          title: t,
          description,
          priority,
          dueDate: dueDate || null,
          tags: [],
        },
      ]);
    }
    onClose();
  };

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>{taskId ? "编辑任务" : "新建任务"}</DialogTitle>
      <DialogContent>
        <TextField
          autoFocus
          fullWidth
          label="标题"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          sx={{ mt: 1 }}
        />
        <TextField
          fullWidth
          multiline
          minRows={3}
          label="描述"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          sx={{ mt: 2 }}
        />
        <FormControl fullWidth sx={{ mt: 2 }}>
          <InputLabel id="priority-label">优先级</InputLabel>
          <Select
            labelId="priority-label"
            label="优先级"
            value={priority}
            onChange={(e) => setPriority(e.target.value)}
          >
            <MenuItem value="none">无</MenuItem>
            <MenuItem value="low">低</MenuItem>
            <MenuItem value="medium">中</MenuItem>
            <MenuItem value="high">高</MenuItem>
          </Select>
        </FormControl>
        <TextField
          fullWidth
          type="date"
          label="截止日期"
          value={dueDate}
          onChange={(e) => setDueDate(e.target.value)}
          sx={{ mt: 2 }}
          InputLabelProps={{ shrink: true }}
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button onClick={save} variant="contained" disabled={!title.trim()}>
          保存
        </Button>
      </DialogActions>
    </Dialog>
  );
}
