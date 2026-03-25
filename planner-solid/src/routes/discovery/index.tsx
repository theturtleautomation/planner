import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createResource, createSignal, For, Match, Show, Switch } from "solid-js";

import {
  acceptEdgeProposal,
  acceptProposal,
  listProposedEdges,
  listProposedNodes,
  rejectEdgeProposal,
  rejectProposal,
  runDiscoveryScan,
} from "~/lib/api";
import type { ProposedEdge, ProposedNode, ProposalStatus } from "~/lib/types";

type ProposalView = "nodes" | "edges";

const STATUS_FILTERS: Array<{ key: ProposalStatus | "all"; label: string }> = [
  { key: "all", label: "All" },
  { key: "pending", label: "Pending" },
  { key: "accepted", label: "Accepted" },
  { key: "rejected", label: "Rejected" },
  { key: "merged", label: "Merged" },
];

const PROPOSAL_VIEWS: Array<{ key: ProposalView; label: string }> = [
  { key: "nodes", label: "Node proposals" },
  { key: "edges", label: "Edge proposals" },
];

const SOURCE_LABELS: Record<string, string> = {
  cargo_toml: "Cargo.toml",
  directory_scan: "Directory scan",
  pipeline_run: "Pipeline",
  manual: "Manual",
  code_graph_context: "Code graph",
};

function relativeTime(timestamp: string): string {
  const diff = Date.now() - new Date(timestamp).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function nodeDisplayName(proposal: ProposedNode): string {
  return proposal.node.name ?? proposal.node.title ?? proposal.node.label ?? proposal.node.scenario ?? proposal.id;
}

function statusGroupLabel(status: ProposalStatus | "all"): string {
  if (status === "all") return "All proposals";
  if (status === "pending") return "Pending review";
  if (status === "accepted") return "Accepted";
  if (status === "rejected") return "Rejected";
  return "Merged";
}

export default function DiscoveryPage() {
  const [proposalView, setProposalView] = createSignal<ProposalView>("nodes");
  const [filterStatus, setFilterStatus] = createSignal<ProposalStatus | "all">("pending");
  const [selectedProposalId, setSelectedProposalId] = createSignal<string | null>(null);
  const [refreshNonce, setRefreshNonce] = createSignal(0);
  const [scanMessage, setScanMessage] = createSignal<string | null>(null);
  const [scanError, setScanError] = createSignal<string | null>(null);
  const [actionLoadingId, setActionLoadingId] = createSignal<string | null>(null);
  const [isScanning, setIsScanning] = createSignal(false);

  const [nodeResponse, { refetch: refetchNodes }] = createResource(
    () => [filterStatus(), refreshNonce()] as const,
    async ([status]) => listProposedNodes(status === "all" ? undefined : status),
  );

  const [edgeResponse, { refetch: refetchEdges }] = createResource(
    () => [filterStatus(), refreshNonce()] as const,
    async ([status]) => listProposedEdges(status === "all" ? undefined : status),
  );

  const activeProposals = createMemo<Array<ProposedNode | ProposedEdge>>(() =>
    proposalView() === "nodes" ? nodeResponse()?.proposals ?? [] : edgeResponse()?.proposals ?? [],
  );

  createEffect(() => {
    const proposals = activeProposals();
    if (proposals.length === 0) {
      setSelectedProposalId(null);
      return;
    }
    if (!selectedProposalId() || !proposals.some(proposal => proposal.id === selectedProposalId())) {
      setSelectedProposalId(proposals[0]!.id);
    }
  });

  const selectedProposal = createMemo(() => activeProposals().find(proposal => proposal.id === selectedProposalId()) ?? null);
  const pendingCount = createMemo(() => activeProposals().filter(proposal => proposal.status === "pending").length);
  const reviewedCount = createMemo(() => activeProposals().filter(proposal => proposal.status !== "pending").length);

  const groupedProposals = createMemo(() => {
    const proposals = activeProposals();
    if (filterStatus() !== "all") {
      return [{ key: filterStatus(), title: statusGroupLabel(filterStatus()), proposals }];
    }

    const pending = proposals.filter(proposal => proposal.status === "pending");
    const reviewed = proposals.filter(proposal => proposal.status !== "pending");
    const groups: Array<{ key: string; title: string; proposals: Array<ProposedNode | ProposedEdge> }> = [];
    if (pending.length > 0) groups.push({ key: "pending", title: "Pending review", proposals: pending });
    if (reviewed.length > 0) groups.push({ key: "reviewed", title: "Reviewed", proposals: reviewed });
    return groups;
  });

  async function handleScan() {
    setIsScanning(true);
    setScanError(null);
    setScanMessage(null);
    try {
      const response = await runDiscoveryScan();
      setScanMessage(
        `Scan complete. ${response.total_proposed} node proposal${response.total_proposed === 1 ? "" : "s"} and ${response.total_edge_proposed} edge proposal${response.total_edge_proposed === 1 ? "" : "s"} refreshed.`,
      );
      setRefreshNonce(value => value + 1);
      void refetchNodes();
      void refetchEdges();
    } catch (error) {
      setScanError(error instanceof Error ? error.message : "Unable to run discovery scan.");
    } finally {
      setIsScanning(false);
    }
  }

  async function handleNodeDecision(proposal: ProposedNode, action: "accept" | "reject") {
    setActionLoadingId(proposal.id);
    try {
      if (action === "accept") {
        await acceptProposal(proposal.id);
      } else {
        await rejectProposal(proposal.id);
      }
      setRefreshNonce(value => value + 1);
      void refetchNodes();
    } finally {
      setActionLoadingId(null);
    }
  }

  async function handleEdgeDecision(proposal: ProposedEdge, action: "accept" | "reject") {
    setActionLoadingId(proposal.id);
    try {
      if (action === "accept") {
        await acceptEdgeProposal(proposal.id);
      } else {
        await rejectEdgeProposal(proposal.id);
      }
      setRefreshNonce(value => value + 1);
      void refetchEdges();
    } finally {
      setActionLoadingId(null);
    }
  }

  return (
    <section class="page page-scroll">
      <Title>Discovery</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Discovery triage</div>
              <h1 class="page-title">Discovery</h1>
              <p class="page-copy">
                Review inferred structure as a triage desk. Pending proposals stay visually
                dominant while scan controls and supporting context remain attached and secondary.
              </p>
            </div>
            <div class="page-actions">
              <button class="btn btn-subtle" type="button" onClick={() => void handleScan()} disabled={isScanning()}>
                {isScanning() ? "Scanning…" : "Run scan"}
              </button>
            </div>
          </div>
          <div class="page-summary-row">
            <span class="pill">{pendingCount()} pending</span>
            <span class="page-summary-note">
              {reviewedCount()} reviewed. {proposalView() === "nodes" ? "Node proposals are active." : "Edge proposals are active."}
            </span>
          </div>
          <Show when={scanMessage()}>{message => <div class="status-copy">{message()}</div>}</Show>
          <Show when={scanError()}>{message => <div class="error-copy">{message()}</div>}</Show>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Mode and filters</div>
              <h2 class="section-title">Review controls</h2>
            </div>
          </div>
          <div class="timeline-toolbar">
            <div class="advanced-tab-row" role="tablist" aria-label="Discovery proposal mode">
              <For each={PROPOSAL_VIEWS}>
                {view => (
                  <button
                    class={`advanced-tab${proposalView() === view.key ? " is-active" : ""}`}
                    type="button"
                    role="tab"
                    aria-selected={proposalView() === view.key}
                    onClick={() => setProposalView(view.key)}
                  >
                    {view.label}
                  </button>
                )}
              </For>
            </div>
            <div class="timeline-filter-row">
              <For each={STATUS_FILTERS}>
                {status => (
                  <button
                    class={`timeline-filter-chip${filterStatus() === status.key ? " is-active" : ""}`}
                    type="button"
                    onClick={() => setFilterStatus(status.key)}
                  >
                    {status.label}
                  </button>
                )}
              </For>
            </div>
          </div>
        </section>

        <section class="section-panel discovery-layout">
          <div class="discovery-list-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Primary triage</div>
                <h2 class="section-title">Proposal queue</h2>
              </div>
            </div>
            <Show
              when={!(proposalView() === "nodes" ? nodeResponse.loading : edgeResponse.loading)}
              fallback={<div class="advanced-loading">Loading discovery proposals…</div>}
            >
              <Show
                when={activeProposals().length > 0}
                fallback={
                  <div class="empty-state">
                    {proposalView() === "nodes"
                      ? "No node proposals match the current filter."
                      : "No edge proposals match the current filter."}
                  </div>
                }
              >
                <div class="timeline-stack">
                  <For each={groupedProposals()}>
                    {group => (
                      <section class="timeline-group" data-group-key={group.key}>
                        <div class="timeline-group-head">
                          <h3 class="group-title timeline-group-title">{group.title}</h3>
                          <span class="pill">
                            {group.proposals.length} item{group.proposals.length === 1 ? "" : "s"}
                          </span>
                        </div>
                        <div class="advanced-list">
                          <For each={group.proposals}>
                            {proposal => (
                              <div class={`proposal-card${selectedProposalId() === proposal.id ? " is-active" : ""}`}>
                                <button class="proposal-card-main" type="button" onClick={() => setSelectedProposalId(proposal.id)}>
                                  <div>
                                    <div class="advanced-item-title">
                                      {proposalView() === "nodes"
                                        ? nodeDisplayName(proposal as ProposedNode)
                                        : `${(proposal as ProposedEdge).edge.source} → ${(proposal as ProposedEdge).edge.target}`}
                                    </div>
                                    <div class="advanced-item-copy">
                                      {SOURCE_LABELS[proposal.source] ?? proposal.source} · {Math.round(proposal.confidence * 100)}% confidence · {relativeTime(proposal.proposed_at)}
                                    </div>
                                    <div class="advanced-item-meta">{proposal.reason}</div>
                                  </div>
                                  <span class="pill">{proposal.status}</span>
                                </button>
                                <Show when={proposal.status === "pending"}>
                                  <div class="proposal-card-actions">
                                    <button
                                      class="btn btn-primary"
                                      type="button"
                                      disabled={actionLoadingId() === proposal.id}
                                      onClick={() =>
                                        proposalView() === "nodes"
                                          ? void handleNodeDecision(proposal as ProposedNode, "accept")
                                          : void handleEdgeDecision(proposal as ProposedEdge, "accept")
                                      }
                                    >
                                      Accept
                                    </button>
                                    <button
                                      class="btn btn-subtle"
                                      type="button"
                                      disabled={actionLoadingId() === proposal.id}
                                      onClick={() =>
                                        proposalView() === "nodes"
                                          ? void handleNodeDecision(proposal as ProposedNode, "reject")
                                          : void handleEdgeDecision(proposal as ProposedEdge, "reject")
                                      }
                                    >
                                      Reject
                                    </button>
                                  </div>
                                </Show>
                              </div>
                            )}
                          </For>
                        </div>
                      </section>
                    )}
                  </For>
                </div>
              </Show>
            </Show>
          </div>

          <div class="discovery-detail-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Attached context</div>
                <h2 class="section-title">Selected proposal</h2>
              </div>
            </div>
            <Show
              when={selectedProposal()}
              fallback={<div class="empty-state">Select a proposal to inspect its supporting context.</div>}
            >
              {proposal => (
                <div class="advanced-column-panel">
                  <div>
                    <div class="advanced-item-title">
                      <Switch>
                        <Match when={proposalView() === "nodes"}>{nodeDisplayName(proposal() as ProposedNode)}</Match>
                        <Match when={proposalView() === "edges"}>
                          {(proposal() as ProposedEdge).edge.source} → {(proposal() as ProposedEdge).edge.target}
                        </Match>
                      </Switch>
                    </div>
                    <div class="advanced-item-copy">
                      {SOURCE_LABELS[proposal().source] ?? proposal().source} · {proposal().status}
                    </div>
                  </div>
                  <div>
                    <div class="advanced-label">Reason</div>
                    <div class="advanced-value">{proposal().reason}</div>
                  </div>
                  <div>
                    <div class="advanced-label">Source artifact</div>
                    <div class="advanced-value">{proposal().source_artifact ?? "Not attached"}</div>
                  </div>
                  <Show when={proposalView() === "nodes"}>
                    <div>
                      <div class="advanced-label">Proposed node context</div>
                      <div class="advanced-value">
                        {(proposal() as ProposedNode).node.node_type}
                        <Show when={(proposal() as ProposedNode).node.scope.project?.project_name}>
                          {` · ${(proposal() as ProposedNode).node.scope.project?.project_name}`}
                        </Show>
                      </div>
                    </div>
                  </Show>
                  <Show when={proposalView() === "edges"}>
                    <div>
                      <div class="advanced-label">Proposed edge</div>
                      <div class="advanced-value">
                        {(proposal() as ProposedEdge).edge.edge_type} between {(proposal() as ProposedEdge).edge.source} and {(proposal() as ProposedEdge).edge.target}
                      </div>
                    </div>
                  </Show>
                </div>
              )}
            </Show>
          </div>
        </section>
      </div>
    </section>
  );
}
