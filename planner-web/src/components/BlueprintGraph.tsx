import { useEffect, useCallback } from 'react';
import {
  ReactFlow,
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  Handle,
  Position,
  MarkerType,
  ReactFlowProvider,
} from '@xyflow/react';
import type {
  NodeProps,
  Node as FlowNode,
  Edge as FlowEdge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import dagre from '@dagrejs/dagre';
import * as d3 from 'd3';
import type { NodeSummary, EdgePayload, NodeType, EdgeType } from '../types/blueprint.ts';

// ─── Style Configuration ────────────────────────────────────────────────────

const NODE_COLORS: Record<string, string> = {
  project:             'var(--color-primary)',
  decision:            'var(--color-primary)',
  technology:          'var(--color-blue)',
  component:           'var(--color-purple)',
  constraint:          'var(--color-warning)',
  pattern:             'var(--color-success)',
  quality_requirement: 'var(--color-gold)',
};

const ICONS: Record<string, string> = {
  project: '⬢',
  decision: '◆',
  technology: '⬡',
  component: '▪',
  constraint: '◇',
  pattern: '◉',
  quality_requirement: '⛨',
};

const TYPE_PREFIX: Record<string, string> = {
  project: 'PROJ',
  decision: 'DEC',
  technology: 'TECH',
  component: 'COMP',
  constraint: 'CON',
  pattern: 'PAT',
  quality_requirement: 'QUAL',
};

function edgeColor(type: EdgeType | string): string {
  switch (type) {
    case 'contains':    return 'var(--color-primary)';
    case 'depends_on':  return 'var(--color-purple)';
    case 'decided_by':  return 'var(--color-blue)';
    case 'constrains':  return 'var(--color-warning)';
    case 'uses':        return 'var(--color-blue)';
    case 'implements':  return 'var(--color-success)';
    case 'satisfies':   return 'var(--color-gold)';
    case 'affects':     return 'var(--color-purple-hover, #bf8ef0)';
    case 'supersedes':  return 'var(--color-warning)';
    default:            return 'var(--color-text-faint)';
  }
}

// ─── Custom Node Component ──────────────────────────────────────────────────

type CustomNodeData = {
  summary: NodeSummary;
  isSelected: boolean;
  isDimmed: boolean;
  hasStaleWarning: boolean;
  hasOrphanWarning: boolean;
};

function CustomNodeComponent({ data, selected }: NodeProps<FlowNode<CustomNodeData>>) {
  const { summary, isDimmed, hasStaleWarning, hasOrphanWarning } = data;
  const color = NODE_COLORS[summary.node_type] || 'var(--color-text-faint)';
  const icon = ICONS[summary.node_type] || '◎';
  const prefix = TYPE_PREFIX[summary.node_type] || 'NODE';

  return (
    <div
      style={{
        padding: '10px 14px',
        borderRadius: '8px',
        background: 'var(--color-surface)',
        border: `1px solid ${selected ? color : 'var(--color-border)'}`,
        boxShadow: selected ? `0 0 0 1px ${color}` : 'var(--shadow-sm)',
        width: 220,
        opacity: isDimmed ? 0.2 : 1,
        transition: 'opacity 0.2s, box-shadow 0.2s, border-color 0.2s',
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
      }}
    >
      <Handle type="target" position={Position.Top} style={{ visibility: 'hidden' }} />
      
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '6px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ color, fontSize: '14px', lineHeight: 1 }}>{icon}</span>
          <span style={{ 
            color, 
            fontSize: '9px', 
            fontWeight: 700, 
            fontFamily: 'var(--font-mono)', 
            letterSpacing: '0.06em' 
          }}>
            {prefix}
          </span>
        </div>
        <div style={{ display: 'flex', gap: '4px' }}>
          {hasStaleWarning && (
            <div title="Stale: not updated recently" style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--color-warning)' }} />
          )}
          {hasOrphanWarning && (
            <div title="Unlinked: no explicit relationships" style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--color-error)' }} />
          )}
        </div>
      </div>
      
      <div 
        title={summary.name} // Tooltip for full name
        style={{ 
          color: 'var(--color-text)', 
          fontSize: '13px', 
          fontWeight: 500,
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
      >
        {summary.name}
      </div>

      <Handle type="source" position={Position.Bottom} style={{ visibility: 'hidden' }} />
    </div>
  );
}

const nodeTypes = {
  custom: CustomNodeComponent,
};

// ─── Layout Generators ──────────────────────────────────────────────────────

const NODE_WIDTH = 220;
const NODE_HEIGHT = 65;
const GRID_X_GAP = 96;
const GRID_Y_GAP = 72;

function getDagreLayout(nodes: FlowNode[], edges: FlowEdge[]): FlowNode[] {
  const dagreGraph = new dagre.graphlib.Graph();
  dagreGraph.setDefaultEdgeLabel(() => ({}));
  dagreGraph.setGraph({ rankdir: 'TB', nodesep: 50, ranksep: 100 });

  nodes.forEach((node) => {
    dagreGraph.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  });

  edges.forEach((edge) => {
    dagreGraph.setEdge(edge.source, edge.target);
  });

  dagre.layout(dagreGraph);

  return nodes.map((node) => {
    const nodeWithPosition = dagreGraph.node(node.id);
    return {
      ...node,
      targetPosition: Position.Top,
      sourcePosition: Position.Bottom,
      position: {
        x: nodeWithPosition.x - NODE_WIDTH / 2,
        y: nodeWithPosition.y - NODE_HEIGHT / 2,
      },
    };
  });
}

function getGridLayout(nodes: FlowNode[]): FlowNode[] {
  const columns = Math.max(1, Math.ceil(Math.sqrt(nodes.length)));
  const rowWidth = columns * NODE_WIDTH + Math.max(0, columns - 1) * GRID_X_GAP;

  return nodes.map((node, index) => {
    const column = index % columns;
    const row = Math.floor(index / columns);
    const x = column * (NODE_WIDTH + GRID_X_GAP) - rowWidth / 2 + NODE_WIDTH / 2;
    const y = row * (NODE_HEIGHT + GRID_Y_GAP);

    return {
      ...node,
      targetPosition: Position.Top,
      sourcePosition: Position.Bottom,
      position: {
        x,
        y,
      },
    };
  });
}

function getForceLayout(nodes: FlowNode[], edges: FlowEdge[]): FlowNode[] {
  if (edges.length === 0) {
    return getGridLayout(nodes);
  }

  const simNodes = nodes.map(n => ({ ...n, id: n.id, x: 0, y: 0 }));
  const simLinks = edges.map(e => ({ source: e.source, target: e.target }));

  const sim = d3.forceSimulation(simNodes as any)
    .force('link', d3.forceLink(simLinks as any).id((d: any) => d.id).distance(200))
    .force('charge', d3.forceManyBody().strength(-700))
    .force('center', d3.forceCenter(0, 0))
    .force('collision', d3.forceCollide().radius(120));

  sim.stop();
  sim.tick(300);

  return nodes.map((node) => {
    const simNode = (simNodes as any).find((n: any) => n.id === node.id);
    return {
      ...node,
      targetPosition: Position.Top,
      sourcePosition: Position.Bottom,
      position: {
        x: simNode?.x ?? 0,
        y: simNode?.y ?? 0,
      },
    };
  });
}

// ─── Component ──────────────────────────────────────────────────────────────

interface BlueprintGraphProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onHoverNode: (nodeId: string | null) => void;
  filterType: NodeType | null;
  layoutMode?: 'force' | 'hierarchical';
}

function BlueprintGraphInner({
  nodes: rawNodes,
  edges: rawEdges,
  selectedNodeId,
  onSelectNode,
  onHoverNode,
  filterType,
  layoutMode = 'force',
}: BlueprintGraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState<FlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<FlowEdge>([]);

  // Health calculations
  const STALE_DAYS = 30;

  // Convert raw data to React Flow format and calculate initial layout
  useEffect(() => {
    const nowMs = Date.now();
    const filteredNodes = filterType
      ? rawNodes.filter(n => n.node_type === filterType)
      : rawNodes;

    const nodeIds = new Set(filteredNodes.map(n => n.id));
    const filteredEdges = rawEdges.filter(e => nodeIds.has(e.source) && nodeIds.has(e.target));

    const initialFlowNodes: FlowNode[] = filteredNodes.map(n => {
      const updMs = new Date(n.updated_at).getTime();
      const isStale = !isNaN(updMs) && (nowMs - updMs) > STALE_DAYS * 86400000;
      const isOrphan = !rawEdges.some(e => e.source === n.id || e.target === n.id);

      return {
        id: n.id,
        type: 'custom',
        position: { x: 0, y: 0 },
        data: {
          summary: n,
          isDimmed: false,
          hasStaleWarning: isStale,
          hasOrphanWarning: isOrphan,
        },
      };
    });

    const initialFlowEdges: FlowEdge[] = filteredEdges.map(e => ({
      id: `${e.source}-${e.target}-${e.edge_type}`,
      source: e.source,
      target: e.target,
      animated: e.edge_type === 'affects',
      style: {
        stroke: edgeColor(e.edge_type),
        strokeWidth: 2,
        strokeDasharray: e.edge_type === 'decided_by' ? '5,5' : 'none',
      },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: edgeColor(e.edge_type),
      },
    }));

    const layoutedNodes = layoutMode === 'hierarchical' 
      ? getDagreLayout(initialFlowNodes, initialFlowEdges)
      : getForceLayout(initialFlowNodes, initialFlowEdges);

    setNodes(layoutedNodes);
    setEdges(initialFlowEdges);
  }, [rawNodes, rawEdges, filterType, layoutMode, setNodes, setEdges]);

  // Update node visual state based on selection
  useEffect(() => {
    setNodes(nds =>
      nds.map(n => {
        let isDimmed = false;
        
        if (selectedNodeId) {
          // Find if this node is connected to the selected node
          const isConnected = edges.some(e => 
            (e.source === selectedNodeId && e.target === n.id) ||
            (e.target === selectedNodeId && e.source === n.id)
          );
          if (n.id !== selectedNodeId && !isConnected) {
            isDimmed = true;
          }
        }

        return {
          ...n,
          data: {
            ...n.data,
            isDimmed,
            isSelected: n.id === selectedNodeId,
          },
        };
      })
    );

    setEdges(eds => 
      eds.map(e => {
        let opacity = 1;
        if (selectedNodeId) {
          if (e.source !== selectedNodeId && e.target !== selectedNodeId) {
            opacity = 0.1;
          }
        }
        return {
          ...e,
          style: { ...e.style, opacity },
        };
      })
    );
  }, [selectedNodeId, setNodes, setEdges]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: FlowNode) => {
      if (selectedNodeId === node.id) {
        onSelectNode(null);
      } else {
        onSelectNode(node.id);
      }
    },
    [selectedNodeId, onSelectNode]
  );

  const onPaneClick = useCallback(() => {
    onSelectNode(null);
  }, [onSelectNode]);

  const onNodeMouseEnter = useCallback(
    (_: React.MouseEvent, node: FlowNode) => {
      onHoverNode(node.id);
    },
    [onHoverNode]
  );

  const onNodeMouseLeave = useCallback(() => {
    onHoverNode(null);
  }, [onHoverNode]);

  if (rawNodes.length === 0) {
    return (
      <div style={{
        width: '100%', height: '100%',
        display: 'flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center',
        gap: '12px', color: 'var(--color-text-faint)',
        fontSize: 'var(--text-sm)',
      }}>
        <span style={{ fontSize: '28px', opacity: 0.3 }}>◇</span>
        <span>no blueprint nodes yet</span>
        <span style={{ fontSize: 'var(--text-xs)', opacity: 0.65 }}>
          nodes are created as the planner builds your system
        </span>
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        onNodeMouseEnter={onNodeMouseEnter}
        onNodeMouseLeave={onNodeMouseLeave}
        onPaneClick={onPaneClick}
        fitView
        minZoom={0.1}
        maxZoom={2}
      >
        <Background color="#ccc" gap={16} />
        <Controls showInteractive={false} />
        <MiniMap 
          nodeColor={n => {
            const data = n.data as CustomNodeData;
            return NODE_COLORS[data.summary.node_type] || '#ccc';
          }}
          maskColor="rgba(0, 0, 0, 0.1)"
          style={{ background: 'var(--color-surface)' }}
        />
      </ReactFlow>
    </div>
  );
}

export default function BlueprintGraph(props: BlueprintGraphProps) {
  return (
    <ReactFlowProvider>
      <BlueprintGraphInner {...props} />
    </ReactFlowProvider>
  );
}
