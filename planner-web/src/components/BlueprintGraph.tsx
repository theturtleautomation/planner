import { useEffect, useRef, useCallback } from 'react';
import * as d3 from 'd3';
import type { GraphNode, GraphLink, NodeSummary, EdgePayload, NodeType } from '../types/blueprint.ts';

// ─── Node type → visual config ──────────────────────────────────────────────

const NODE_CONFIG: Record<string, { color: string; shape: 'circle' | 'diamond' | 'rect' | 'hexagon'; label: string }> = {
  decision:            { color: '#4f98a3', shape: 'diamond', label: 'Decision' },
  technology:          { color: '#6daa45', shape: 'circle',  label: 'Technology' },
  component:           { color: '#5591c7', shape: 'rect',    label: 'Component' },
  constraint:          { color: '#bb653b', shape: 'hexagon', label: 'Constraint' },
  pattern:             { color: '#a86fdf', shape: 'circle',  label: 'Pattern' },
  quality_requirement: { color: '#e8af34', shape: 'rect',    label: 'Quality Req.' },
};

const EDGE_COLORS: Record<string, string> = {
  decided_by:  '#4f98a3',
  supersedes:  '#bb653b',
  depends_on:  '#5591c7',
  uses:        '#6daa45',
  constrains:  '#bb653b',
  implements:  '#a86fdf',
  satisfies:   '#e8af34',
  affects:     '#d163a7',
};

function getNodeConfig(nodeType: string) {
  return NODE_CONFIG[nodeType] ?? { color: '#8a8987', shape: 'circle' as const, label: nodeType };
}

// ─── Shape rendering ────────────────────────────────────────────────────────

function drawNodeShape(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  shape: string,
  radius: number,
  color: string,
  isSelected: boolean,
  isHovered: boolean,
) {
  ctx.beginPath();
  const r = isSelected ? radius * 1.3 : isHovered ? radius * 1.15 : radius;

  switch (shape) {
    case 'diamond':
      ctx.moveTo(x, y - r);
      ctx.lineTo(x + r, y);
      ctx.lineTo(x, y + r);
      ctx.lineTo(x - r, y);
      ctx.closePath();
      break;
    case 'rect':
      ctx.rect(x - r * 0.85, y - r * 0.7, r * 1.7, r * 1.4);
      break;
    case 'hexagon': {
      const a = (Math.PI * 2) / 6;
      for (let i = 0; i < 6; i++) {
        const px = x + r * Math.cos(a * i - Math.PI / 6);
        const py = y + r * Math.sin(a * i - Math.PI / 6);
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();
      break;
    }
    default: // circle
      ctx.arc(x, y, r, 0, Math.PI * 2);
      break;
  }

  // Fill
  if (isSelected) {
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.35;
  } else {
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.18;
  }
  ctx.fill();
  ctx.globalAlpha = 1;

  // Stroke
  ctx.strokeStyle = color;
  ctx.lineWidth = isSelected ? 2.5 : isHovered ? 2 : 1.2;
  ctx.stroke();
}

// ─── Component ──────────────────────────────────────────────────────────────

interface BlueprintGraphProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onHoverNode: (nodeId: string | null) => void;
  width: number;
  height: number;
  filterType: NodeType | null;
}

export default function BlueprintGraph({
  nodes,
  edges,
  selectedNodeId,
  onSelectNode,
  onHoverNode,
  width,
  height,
  filterType,
}: BlueprintGraphProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const simRef = useRef<d3.Simulation<GraphNode, GraphLink> | null>(null);
  const nodesRef = useRef<GraphNode[]>([]);
  const linksRef = useRef<GraphLink[]>([]);
  const hoveredRef = useRef<string | null>(null);
  const transformRef = useRef(d3.zoomIdentity);

  // Convert props to graph data
  const buildGraphData = useCallback(() => {
    const filteredNodes = filterType
      ? nodes.filter(n => n.node_type === filterType)
      : nodes;

    const nodeIds = new Set(filteredNodes.map(n => n.id));
    const filteredEdges = edges.filter(e => nodeIds.has(e.source) && nodeIds.has(e.target));

    // Preserve positions for nodes that still exist
    const existingPositions = new Map<string, { x: number; y: number }>();
    for (const n of nodesRef.current) {
      if (n.x !== undefined && n.y !== undefined) {
        existingPositions.set(n.id, { x: n.x, y: n.y });
      }
    }

    const graphNodes: GraphNode[] = filteredNodes.map(n => {
      const pos = existingPositions.get(n.id);
      return {
        ...n,
        x: pos?.x ?? (width / 2 + (Math.random() - 0.5) * 200),
        y: pos?.y ?? (height / 2 + (Math.random() - 0.5) * 200),
      };
    });

    const graphLinks: GraphLink[] = filteredEdges.map(e => ({
      source: e.source,
      target: e.target,
      edge_type: e.edge_type,
      metadata: e.metadata,
    }));

    return { graphNodes, graphLinks };
  }, [nodes, edges, filterType, width, height]);

  // Hit test: find node under mouse
  const hitTest = useCallback((mx: number, my: number): GraphNode | null => {
    const t = transformRef.current;
    const x = (mx - t.x) / t.k;
    const y = (my - t.y) / t.k;
    const r = 16;
    // Check in reverse order (topmost first)
    for (let i = nodesRef.current.length - 1; i >= 0; i--) {
      const n = nodesRef.current[i];
      if (n.x !== undefined && n.y !== undefined) {
        const dx = x - n.x;
        const dy = y - n.y;
        if (dx * dx + dy * dy < r * r) return n;
      }
    }
    return null;
  }, []);

  // Render loop
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    ctx.save();
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.scale(dpr, dpr);

    const t = transformRef.current;
    ctx.translate(t.x, t.y);
    ctx.scale(t.k, t.k);

    // Draw edges
    for (const link of linksRef.current) {
      const src = link.source as GraphNode;
      const tgt = link.target as GraphNode;
      if (src.x === undefined || src.y === undefined || tgt.x === undefined || tgt.y === undefined) continue;

      const edgeColor = EDGE_COLORS[link.edge_type] ?? '#5a5957';
      const isConnected = selectedNodeId && (src.id === selectedNodeId || tgt.id === selectedNodeId);

      ctx.beginPath();
      ctx.moveTo(src.x, src.y);
      ctx.lineTo(tgt.x, tgt.y);
      ctx.strokeStyle = edgeColor;
      ctx.globalAlpha = isConnected ? 0.9 : selectedNodeId ? 0.15 : 0.4;
      ctx.lineWidth = isConnected ? 2 : 1;
      ctx.stroke();
      ctx.globalAlpha = 1;

      // Arrowhead
      const angle = Math.atan2(tgt.y - src.y, tgt.x - src.x);
      const arrLen = 8;
      const midX = (src.x + tgt.x) / 2 + (tgt.x - src.x) * 0.15;
      const midY = (src.y + tgt.y) / 2 + (tgt.y - src.y) * 0.15;
      ctx.beginPath();
      ctx.moveTo(midX, midY);
      ctx.lineTo(midX - arrLen * Math.cos(angle - 0.4), midY - arrLen * Math.sin(angle - 0.4));
      ctx.lineTo(midX - arrLen * Math.cos(angle + 0.4), midY - arrLen * Math.sin(angle + 0.4));
      ctx.closePath();
      ctx.fillStyle = edgeColor;
      ctx.globalAlpha = isConnected ? 0.9 : selectedNodeId ? 0.15 : 0.4;
      ctx.fill();
      ctx.globalAlpha = 1;
    }

    // Draw nodes
    for (const node of nodesRef.current) {
      if (node.x === undefined || node.y === undefined) continue;
      const config = getNodeConfig(node.node_type);
      const isSelected = node.id === selectedNodeId;
      const isHovered = node.id === hoveredRef.current;

      // Dim non-connected nodes when something is selected
      if (selectedNodeId && !isSelected) {
        const isConnected = linksRef.current.some(l => {
          const src = (l.source as GraphNode).id;
          const tgt = (l.target as GraphNode).id;
          return (src === selectedNodeId && tgt === node.id) || (tgt === selectedNodeId && src === node.id);
        });
        if (!isConnected) {
          ctx.globalAlpha = 0.25;
        }
      }

      drawNodeShape(ctx, node.x, node.y, config.shape, 14, config.color, isSelected, isHovered);

      // Label
      ctx.font = '500 10px "Inter", system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillStyle = isSelected ? config.color : '#cdccca';
      ctx.globalAlpha = ctx.globalAlpha < 1 ? ctx.globalAlpha : (isSelected || isHovered ? 1 : 0.85);

      // Truncate name
      const maxLen = 18;
      const label = node.name.length > maxLen ? node.name.slice(0, maxLen - 1) + '\u2026' : node.name;
      ctx.fillText(label, node.x, node.y + 18);

      ctx.globalAlpha = 1;
    }

    ctx.restore();
  }, [selectedNodeId]);

  // Initialize simulation
  useEffect(() => {
    if (width === 0 || height === 0) return;

    const { graphNodes, graphLinks } = buildGraphData();
    nodesRef.current = graphNodes;
    linksRef.current = graphLinks;

    const sim = d3.forceSimulation(graphNodes)
      .force('link', d3.forceLink<GraphNode, GraphLink>(graphLinks)
        .id(d => d.id)
        .distance(100)
        .strength(0.4))
      .force('charge', d3.forceManyBody().strength(-300).distanceMax(400))
      .force('center', d3.forceCenter(width / 2, height / 2).strength(0.05))
      .force('collision', d3.forceCollide<GraphNode>().radius(28))
      .alphaDecay(0.02)
      .on('tick', render);

    simRef.current = sim;

    return () => {
      sim.stop();
    };
  }, [nodes, edges, filterType, width, height, buildGraphData, render]);

  // Canvas setup + zoom + interaction
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    // Zoom
    const zoomBehavior = d3.zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.2, 4])
      .on('zoom', (event: d3.D3ZoomEvent<HTMLCanvasElement, unknown>) => {
        transformRef.current = event.transform;
        render();
      });

    const sel = d3.select(canvas);
    sel.call(zoomBehavior);

    // Drag
    sel.call(
      d3.drag<HTMLCanvasElement, unknown>()
        .container(canvas)
        .subject((event: d3.D3DragEvent<HTMLCanvasElement, unknown, GraphNode>) => {
          const node = hitTest(event.x, event.y);
          return node ?? undefined;
        })
        .on('start', (event: d3.D3DragEvent<HTMLCanvasElement, unknown, GraphNode>) => {
          if (!event.active) simRef.current?.alphaTarget(0.3).restart();
          event.subject.fx = event.subject.x;
          event.subject.fy = event.subject.y;
        })
        .on('drag', (event: d3.D3DragEvent<HTMLCanvasElement, unknown, GraphNode>) => {
          const t = transformRef.current;
          event.subject.fx = (event.sourceEvent.offsetX - t.x) / t.k;
          event.subject.fy = (event.sourceEvent.offsetY - t.y) / t.k;
        })
        .on('end', (event: d3.D3DragEvent<HTMLCanvasElement, unknown, GraphNode>) => {
          if (!event.active) simRef.current?.alphaTarget(0);
          event.subject.fx = null;
          event.subject.fy = null;
        })
    );

    // Click
    canvas.addEventListener('click', (e: MouseEvent) => {
      const node = hitTest(e.offsetX, e.offsetY);
      onSelectNode(node?.id ?? null);
    });

    // Hover
    canvas.addEventListener('mousemove', (e: MouseEvent) => {
      const node = hitTest(e.offsetX, e.offsetY);
      const newHovered = node?.id ?? null;
      if (newHovered !== hoveredRef.current) {
        hoveredRef.current = newHovered;
        onHoverNode(newHovered);
        canvas.style.cursor = newHovered ? 'pointer' : 'grab';
        render();
      }
    });
  }, [width, height, render, hitTest, onSelectNode, onHoverNode]);

  // Empty state
  if (nodes.length === 0) {
    return (
      <div style={{
        width, height,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '12px',
        color: 'var(--text-secondary)',
        fontSize: '13px',
      }}>
        <span style={{ fontSize: '28px', opacity: 0.3 }}>◇</span>
        <span>no blueprint nodes yet</span>
        <span style={{ fontSize: '11px', opacity: 0.65 }}>
          nodes are created as the planner builds your system
        </span>
      </div>
    );
  }

  return (
    <canvas
      ref={canvasRef}
      style={{
        width: `${width}px`,
        height: `${height}px`,
        background: 'transparent',
      }}
    />
  );
}
