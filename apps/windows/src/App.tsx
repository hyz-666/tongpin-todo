import { Container, Typography, CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  palette: {
    mode: "light",
    primary: { main: "#534AB7" },
  },
});

export default function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Container maxWidth="md" sx={{ mt: 4 }}>
        <Typography variant="h4" component="h1">
          tongpin-todo
        </Typography>
        <Typography variant="body1" color="text.secondary" sx={{ mt: 1 }}>
          Windows 客户端骨架已就绪（Task 1）。任务列表与同步将在后续任务接入。
        </Typography>
      </Container>
    </ThemeProvider>
  );
}
