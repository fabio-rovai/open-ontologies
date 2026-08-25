import { AppShell } from './AppShell';

// The old Layout component talked to the engine directly (Graph3D issued
// its own SPARQL, the toolbar called mcp-client straight) and only ever
// worked in the desktop build. AppShell replaces it: components take data
// and callbacks as props and read through the DemoSource abstraction, so
// the same tree renders here and in the static web build (see main.tsx).
function App() {
  return <AppShell />;
}

export default App;
