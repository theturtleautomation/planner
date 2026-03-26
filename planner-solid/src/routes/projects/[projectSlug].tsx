import { ProjectWorkspaceScreen } from "~/components/projects/ProjectWorkspaceScreen";

import { useProjectWorkspaceController } from "./project-workspace-controller";

export default function ProjectWorkspacePage() {
  const controller = useProjectWorkspaceController();
  return <ProjectWorkspaceScreen controller={controller} />;
}
