import { Title } from "@solidjs/meta";
import { Show } from "solid-js";

import { ProjectAdvancedPanel } from "~/components/projects/ProjectAdvancedPanel";
import { ProjectSessionList } from "~/components/projects/ProjectSessionList";
import { ProjectWorkspaceHero } from "~/components/projects/ProjectWorkspaceHero";
import type { ProjectWorkspaceController } from "~/routes/projects/project-workspace-controller";
import { presentSessionTitle } from "~/lib/workspace";

interface ProjectWorkspaceScreenProps {
  controller: ProjectWorkspaceController;
}

export function ProjectWorkspaceScreen(props: ProjectWorkspaceScreenProps) {
  return (
    <section class="page page-scroll">
      <Title>{props.controller.project()?.project.name ?? "Project"}</Title>
      <div class="stack page-frame">
        <Show when={props.controller.project()} fallback={<div class="empty-state">Loading project workspace…</div>}>
          {response => {
            const currentProject = () => response().project;
            const currentSummary = () => props.controller.summary();
            const activeSession = () => props.controller.activeSession();

            return (
              <>
                <ProjectWorkspaceHero
                  activeSessionId={activeSession()?.id ?? null}
                  error={props.controller.error()}
                  focusCopy={
                    activeSession()?.project_description?.trim() ||
                    "Start a new Socratic analysis to shape this project's working truth."
                  }
                  focusTitle={activeSession() ? presentSessionTitle(activeSession()!) : "No active analysis yet"}
                  projectDescription={currentProject().description}
                  projectName={currentProject().name}
                  readinessLabel={props.controller.buildReadiness().label}
                  readinessTone={props.controller.readinessTone()}
                  starting={props.controller.starting()}
                  statusLabel={currentSummary()?.statusLabel ?? "Ready to start"}
                  onStartAnalysis={() => void props.controller.handleStartAnalysis()}
                />

                <ProjectSessionList sessions={props.controller.projectSessions()} />

                <ProjectAdvancedPanel
                  activeSessionId={activeSession()?.id ?? null}
                  activeSessionStep={activeSession()?.current_step ?? null}
                  activityLoading={props.controller.activityLoading()}
                  activitySummary={props.controller.activitySummary()}
                  applyPending={props.controller.applyPending()}
                  blueprintSummary={props.controller.blueprintSummary()}
                  buildExecution={props.controller.buildExecution()}
                  buildLoading={props.controller.buildLoading()}
                  buildPath={props.controller.buildPath()}
                  buildReadiness={props.controller.buildReadiness()}
                  executionLoading={props.controller.executionLoading()}
                  importReview={props.controller.importReview()}
                  importState={props.controller.importState()}
                  knowledgeSummary={props.controller.knowledgeSummary()}
                  open={props.controller.advancedOpen()}
                  outputsLoading={props.controller.outputsLoading()}
                  outputArtifacts={props.controller.outputArtifacts()}
                  projectSlug={currentProject().slug}
                  promptBank={props.controller.promptBank()}
                  readinessLoading={props.controller.readinessLoading()}
                  reimportPending={props.controller.reimportPending()}
                  reviewError={props.controller.reviewError()}
                  reviewLoading={props.controller.reviewLoading()}
                  reviewSummary={props.controller.reviewSummary()}
                  selectionPendingNodeId={props.controller.selectionPendingNodeId()}
                  tab={props.controller.advancedTab()}
                  onApplyImportReview={() => void props.controller.handleApplyImportReview()}
                  onClose={props.controller.closeProjectSurfaces}
                  onOpen={props.controller.openProjectSurfaces}
                  onReimport={() => void props.controller.handleReimport()}
                  onSetImportNodeIncluded={(nodeId, included) =>
                    void props.controller.handleSetImportNodeIncluded(nodeId, included)}
                  onTabChange={props.controller.handleProjectSurfaceTabChange}
                />
              </>
            );
          }}
        </Show>
      </div>
    </section>
  );
}
