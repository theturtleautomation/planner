// ─── Blueprint Types ─────────────────────────────────────────────────────────
// Mirrors the Rust types in planner-schemas/src/artifacts/blueprint.rs
// and the API response shapes in planner-server/src/api.rs.
//
// SYNC CHECK: Last verified against Rust structs on 2026-03-05.
// If you edit types here, update the Rust side too (or vice versa).

export type NodeType =
  | 'decision'
  | 'technology'
  | 'component'
  | 'constraint'
  | 'pattern'
  | 'quality_requirement';

export type EdgeType =
  | 'decided_by'
  | 'supersedes'
  | 'depends_on'
  | 'uses'
  | 'constrains'
  | 'implements'
  | 'satisfies'
  | 'affects';

export type DecisionStatus = 'proposed' | 'accepted' | 'deprecated' | 'superseded';
export type AdoptionRing = 'adopt' | 'trial' | 'assess' | 'hold';
export type TechnologyCategory = 'language' | 'framework' | 'library' | 'runtime' | 'tool' | 'platform' | 'protocol';
export type ComponentType = 'module' | 'service' | 'library' | 'store' | 'interface' | 'pipeline';
export type ComponentStatus = 'planned' | 'in_progress' | 'shipped' | 'deprecated';
export type ComponentNameSource = 'generated' | 'manual';
export type ComponentNamingStrategy = 'spec_group' | 'directory_scan' | 'factory_output' | 'manual_create' | 'backfill';
export type ConstraintType = 'technical' | 'organizational' | 'philosophical' | 'regulatory';
export type QualityAttribute = 'performance' | 'reliability' | 'security' | 'usability' | 'maintainability';
export type QualityPriority = 'critical' | 'high' | 'medium' | 'low';
export type ImpactAction = 'reconverge' | 'update' | 'invalidate' | 'add' | 'remove';
export type ImpactSeverity = 'shallow' | 'medium' | 'deep';
export type ScopeClass = 'global' | 'project' | 'project_contextual' | 'unscoped';
export type ScopeVisibility = 'shared' | 'project_local' | 'unscoped';
export type NodeLifecycle = 'active' | 'archived';
export type BlueprintExportKind = 'single_record' | 'scoped_view';

export interface ProjectScope {
  project_id: string;
  project_name?: string;
}

export interface SecondaryScopeRefs {
  feature?: string;
  widget?: string;
  artifact?: string;
  component?: string;
}

export interface SharedScope {
  linked_project_ids: string[];
  inherit_to_linked_projects: boolean;
}

export interface OverrideScope {
  shared_source_id: string;
  override_reason?: string;
  effective_from?: string;
}

export interface NodeScope {
  scope_class: ScopeClass;
  project?: ProjectScope;
  secondary: SecondaryScopeRefs;
  is_shared: boolean;
  shared?: SharedScope;
  lifecycle: NodeLifecycle;
  override_scope?: OverrideScope;
}

// ─── Node summary (used in list endpoints) ─────────────────────────────────

export interface NodeSummary {
  id: string;
  name: string;
  node_type: string;
  status: string;
  scope_class: ScopeClass;
  scope_visibility: ScopeVisibility;
  is_shared: boolean;
  lifecycle: NodeLifecycle;
  project_id?: string;
  project_name?: string;
  secondary_scope: SecondaryScopeRefs;
  linked_project_ids: string[];
  override_source_id?: string;
  override_reason?: string;
  override_effective_from?: string;
  tags: string[];
  has_documentation: boolean;
  updated_at: string;
}

// ─── Edge ──────────────────────────────────────────────────────────────────

export interface EdgePayload {
  source: string;
  target: string;
  edge_type: EdgeType;
  metadata?: string;
}

// ─── Full Blueprint snapshot ────────────────────────────────────────────────

export interface BlueprintResponse {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  counts: Record<string, number>;
  total_nodes: number;
  total_edges: number;
}

// ─── Node list response ─────────────────────────────────────────────────────

export interface NodeListResponse {
  nodes: NodeSummary[];
  count: number;
}

// ─── Full node (tagged union, matches Rust enum) ────────────────────────────
// The server returns BlueprintNode as a JSON object with a `node_type` tag.
// These interfaces MUST match the Rust serde output exactly.

// Matches Rust: DecisionOption { name, pros, cons, chosen }
export interface DecisionOption {
  name: string;
  pros: string[];
  cons: string[];
  chosen: boolean;
}

// Matches Rust: Consequence { description, positive }
export interface Consequence {
  description: string;
  positive: boolean;  // true = positive, false = negative
}

// Matches Rust: Assumption { description, confidence }
export interface Assumption {
  description: string;
  confidence: string;  // "high" | "medium" | "low"
}

// Matches Rust: Decision { id, title, status, context, options, consequences, assumptions, supersedes?, tags, created_at, updated_at }
export interface DecisionNode {
  node_type: 'decision';
  id: string;
  title: string;
  status: DecisionStatus;
  context: string;
  options: DecisionOption[];
  consequences: Consequence[];
  assumptions: Assumption[];
  supersedes?: string;
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

// Matches Rust: Technology { id, name, version?, category, ring, rationale, license?, tags, created_at, updated_at }
export interface TechnologyNode {
  node_type: 'technology';
  id: string;
  name: string;
  version?: string;
  category: TechnologyCategory;
  ring: AdoptionRing;
  rationale: string;
  license?: string;
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

// Matches Rust: Component { id, name, component_type, description, provides, consumes, status, tags, created_at, updated_at }
export interface ComponentNode {
  node_type: 'component';
  id: string;
  name: string;
  component_type: ComponentType;
  naming?: {
    origin_key: string;
    source: ComponentNameSource;
    strategy: ComponentNamingStrategy;
    generated_name: string;
    naming_version: number;
    last_generated_at: string;
  };
  description: string;
  provides: string[];
  consumes: string[];
  status: ComponentStatus;
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

// Matches Rust: Constraint { id, title, constraint_type, description, source, tags, created_at, updated_at }
export interface ConstraintNode {
  node_type: 'constraint';
  id: string;
  title: string;
  constraint_type: ConstraintType;
  description: string;
  source: string;  // free text — who/what imposed this constraint
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

// Matches Rust: Pattern { id, name, description, rationale, tags, created_at, updated_at }
export interface PatternNode {
  node_type: 'pattern';
  id: string;
  name: string;
  description: string;
  rationale: string;
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

// Matches Rust: QualityRequirement { id, attribute, scenario, priority, tags, created_at, updated_at }
export interface QualityRequirementNode {
  node_type: 'quality_requirement';
  id: string;
  attribute: QualityAttribute;
  scenario: string;
  priority: QualityPriority;
  tags: string[];
  documentation?: string;
  scope: NodeScope;
  created_at: string;
  updated_at: string;
}

export type BlueprintNode =
  | DecisionNode
  | TechnologyNode
  | ComponentNode
  | ConstraintNode
  | PatternNode
  | QualityRequirementNode;

// ─── Impact analysis ────────────────────────────────────────────────────────

export interface ImpactEntry {
  node_id: string;
  node_name: string;
  node_type: string;
  action: ImpactAction;
  severity: ImpactSeverity;
  explanation: string;
}

export interface ImpactReport {
  source_node_id: string;
  source_node_name: string;
  change_description: string;
  entries: ImpactEntry[];
  summary: Record<string, number>;
  timestamp: string;
}

// ─── Event sourcing ───────────────────────────────────────────────────────

export type BlueprintEventType =
  | 'node_created'
  | 'node_updated'
  | 'node_deleted'
  | 'edge_created'
  | 'edges_deleted'
  | 'export_recorded';

export interface BlueprintEventPayload {
  event_type: BlueprintEventType;
  summary: string;
  timestamp: string;
  data: Record<string, unknown>;
}

export interface BlueprintEventsResponse {
  events: BlueprintEventPayload[];
  total: number;
}

// ─── Reconvergence engine ─────────────────────────────────────────────────

export type ReconvergenceStepStatus = 'pending' | 'running' | 'done' | 'skipped' | 'error';

/** A single step in the reconvergence process (e.g., update a downstream node). */
export interface ReconvergenceStep {
  step_id: string;
  node_id: string;
  node_name: string;
  node_type: string;
  action: ImpactAction;
  severity: ImpactSeverity;
  description: string;
  status: ReconvergenceStepStatus;
  error?: string;
}

/** Request body for POST /blueprint/reconverge. */
export interface ReconvergenceRequest {
  source_node_id: string;
  impact_report: ImpactReport;
  /** If true, auto-accept shallow/medium severity; prompt for deep. */
  auto_apply: boolean;
}

/** Response from POST /blueprint/reconverge. */
export interface ReconvergenceResult {
  steps: ReconvergenceStep[];
  summary: {
    total: number;
    applied: number;
    skipped: number;
    errors: number;
    needs_review: number;
  };
  timestamp: string;
}

// ─── D3 simulation types ────────────────────────────────────────────────────

export interface GraphNode extends NodeSummary {
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
  vx?: number;
  vy?: number;
}

export interface GraphLink {
  source: string | GraphNode;
  target: string | GraphNode;
  edge_type: EdgeType;
  metadata?: string;
}

// ─── Discovery / Proposed Nodes ───────────────────────────────────────────────

export type DiscoverySource = 'cargo_toml' | 'directory_scan' | 'pipeline_run' | 'manual';

export type ProposalStatus = 'pending' | 'accepted' | 'rejected' | 'merged';

export interface ProposedNode {
  id: string;
  /** The node data that would be created */
  node: BlueprintNode;
  /** Where this proposal came from */
  source: DiscoverySource;
  /** Human-readable reason for the proposal */
  reason: string;
  /** Current review status */
  status: ProposalStatus;
  /** ISO timestamp of when it was proposed */
  proposed_at: string;
  /** ISO timestamp of when it was reviewed */
  reviewed_at?: string;
  /** Confidence score from scanner (0-1) */
  confidence: number;
  /** File path or artifact that triggered the proposal */
  source_artifact?: string;
}

export interface DiscoveryScanRequest {
  /** Which scanners to run */
  scanners: ('cargo_toml' | 'directory_structure' | 'all')[];
  /** Root path to scan (relative to project) */
  root_path?: string;
}

export interface DiscoveryScanResult {
  scanner: string;
  proposed_count: number;
  skipped_count: number;
  errors: string[];
  duration_ms: number;
}

export interface DiscoveryRunResponse {
  results: DiscoveryScanResult[];
  total_proposed: number;
}

export interface ProposedNodesResponse {
  proposals: ProposedNode[];
  total: number;
}
