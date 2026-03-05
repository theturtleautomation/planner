// ─── Blueprint Types ─────────────────────────────────────────────────────────
// Mirrors the Rust types in planner-schemas/src/artifacts/blueprint.rs
// and the API response shapes in planner-server/src/api.rs.

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
export type TechnologyCategory = 'language' | 'framework' | 'library' | 'tool' | 'platform' | 'database' | 'infrastructure';
export type ConstraintSource = 'business' | 'technical' | 'regulatory' | 'resource';
export type PatternScope = 'system' | 'component' | 'code';
export type ImpactAction = 'reconverge' | 'update' | 'invalidate' | 'add' | 'remove';
export type ImpactSeverity = 'shallow' | 'medium' | 'deep';

// ─── Node summary (used in list endpoints) ─────────────────────────────────

export interface NodeSummary {
  id: string;
  name: string;
  node_type: string;
  tags: string[];
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

export interface DecisionNode {
  node_type: 'decision';
  id: string;
  title: string;
  status: DecisionStatus;
  context: string;
  options: { name: string; description: string; pros: string[]; cons: string[] }[];
  consequences: { description: string; type: 'positive' | 'negative' | 'neutral' }[];
  assumptions: { statement: string; risk: string; validation_approach: string }[];
  supersedes?: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface TechnologyNode {
  node_type: 'technology';
  id: string;
  name: string;
  version?: string;
  category: TechnologyCategory;
  ring: AdoptionRing;
  rationale: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface ComponentNode {
  node_type: 'component';
  id: string;
  name: string;
  description: string;
  responsibilities: string[];
  interfaces: { name: string; direction: string; protocol: string; description: string }[];
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface ConstraintNode {
  node_type: 'constraint';
  id: string;
  title: string;
  description: string;
  source: ConstraintSource;
  negotiable: boolean;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface PatternNode {
  node_type: 'pattern';
  id: string;
  name: string;
  description: string;
  scope: PatternScope;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface QualityRequirementNode {
  node_type: 'quality_requirement';
  id: string;
  attribute: string;
  scenario: string;
  measure: string;
  target: string;
  priority: string;
  tags: string[];
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
