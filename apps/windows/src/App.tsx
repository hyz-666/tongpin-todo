import { useEffect } from "react";
import { appDataDir } from "@tauri-apps/api/path";
import { Box, CssBaseline, ThemeProvider, createTheme } from "@mui/material";
import Sidebar from "./components/Sidebar";
import TaskList from "./components/TaskList";
import { useTodoStore } from "./store";

const theme = createTheme({
  palette: {
    mode: "light",
    primary: { main: "#534AB7" },
  },
});

export default function App() {
  const ready = useTodoStore((s) => s.ready);
  const error = useTodoStore((s) => s.error);
  const open = useTodoStore((s) => s.open);

  useEffect(() => {
    (async () => {
      const dir = await appDataDir();
      await open(`${dir}profile`);
    })();
  }, [open]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {error ? (
        <Box sx={{ p: 3 }}>
          <Box sx={{ fontWeight: 500, mb: 1 }}>启动失败</Box>
          <Box sx={{ color: "error.main" }}>{error}</Box>
        </Box>
      ) : !ready ? (
        <Box sx={{ p: 3 }}>正在加载…</Box>
      ) : (
        <Box sx={{ display: "flex", height: "100vh" }}>
          <Sidebar />
          <Box sx={{ flexGrow: 1, overflow: "auto" }}>
            <TaskList />
          </Box>
        </Box>
      )}
    </ThemeProvider>
  );
}
