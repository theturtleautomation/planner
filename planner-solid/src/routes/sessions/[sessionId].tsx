import SessionWorkspaceScreen from "./session-workspace-screen";
import { useSessionWorkspaceController } from "./session-workspace-controller";

export default function SessionWorkspacePage() {
  const controller = useSessionWorkspaceController();
  return <SessionWorkspaceScreen controller={controller} />;
}
