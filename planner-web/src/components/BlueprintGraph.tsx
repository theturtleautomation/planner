import { useEffect, useRef, useCallback, useMemo, useState } from 'react';
import * as d3 from 'd3';
import type { GraphNode, GraphLink, NodeSummary, EdgePayload, NodeType, EdgeType } from '../types/blueprint.ts';

// ─── Node type → visual config ──────────────────────────────────────────────

const NODE_COLORS: Record<string, string> = {
  decision:            'var(--color-primary)',
  technology:          'var(--color-blue)',
  component:           'var(--color-purple)',
  constraint:          'var(--color-warning)',
  pattern:             'var(--color-success)',
  quality_requirement: 'var(--color-gold)',
};

const NODE_SIZES: Record<string, { w: number; h: number }> = {
  decision:            { w: 185, h: 38 },
  technology:          { w: 160, h: 34 },
  component:           { w: 175, h: 36 },
  constraint:          { w: 170, h: 34 },
  pattern:             { w: 175, h: 34 },
  quality_requirement: { w: 185, h: 36 },
};

const TYPE_PREFIX: Record<string, string> = {
  decision: 'DEC',
  technology: 'TECH',
  component: 'COMP',
  constraint: 'CON',
  pattern: 'PAT',
  quality_requirement: 'QUAL',
};

// ─── Edge styling ───────────────────────────────────────────────────────────

function edgeColor(type: EdgeType | string): string {
  switch (type) {
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

function edgeDash(type: string): string {
  switch (type) {
    case 'decided_by':  return '8,4';
    case 'constrains':  return '3,3';
    case 'implements':  return '2,4';
    case 'satisfies':   return '8,3,2,3';
    case 'affects':     return '6,4';
    default:            return 'none';
  }
}

function edgeWidth(type: string): number {
  if (type === 'depends_on') return 1.8;
  if (type === 'uses') return 1;
  return 1.2;
}

// ─── Shape path generators ──────────────────────────────────────────────────

function getNodeShapePath(type: string, w: number, h: number): string | null {
  const hw = w / 2;
  const hh = h / 2;
  switch (type) {
    case 'technology': {
      // Hexagon
      const inset = hh * 0.85;
      return `M${-hw + inset},${-hh} L${hw - inset},${-hh} L${hw},0 L${hw - inset},${hh} L${-hw + inset},${hh} L${-hw},0 Z`;
    }
    case 'constraint': {
      // Diamond
      return `M0,${-hh} L${hw},0 L0,${hh} L${-hw},0 Z`;
    }
    case 'quality_requirement': {
      // Shield
      const sw = hw * 0.95;
      const sh = hh;
      return `M${-sw},${-sh * 0.7} L0,${-sh} L${sw},${-sh * 0.7} L${sw},${sh * 0.2} Q${sw},${sh * 0.7} 0,${sh} Q${-sw},${sh * 0.7} ${-sw},${sh * 0.2} Z`;
    }
    default:
      return null; // Use rect/ellipse directly
  }
}

// ─── Component ──────────────────────────────────────────────────────────────

interface BlueprintGraphProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onHoverNode: (nodeId: string | null) => void;
  filterType: NodeType | null;
}

export default function BlueprintGraph({
  nodes,
  edges,
  selectedNodeId,
  onSelectNode,
  onHoverNode,
  filterType,
}: BlueprintGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const simRef = useRef<d3.Simulation<GraphNode, GraphLink> | null>(null);
  const gRef = useRef<d3.Selection<SVGGElement, unknown, null, undefined> | null>(null);
  const linkSelRef = useRef<d3.Selection<SVGLineElement, GraphLink, SVGGElement, unknown> | null>(null);
  const nodeSelRef = useRef<d3.Selection<SVGGElement, GraphNode, SVGGElement, unknown> | null>(null);
  const zoomRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
  const initializedRef = useRef(false);
  const minimapNodesRef = useRef<d3.Selection<SVGCircleElement, GraphNode, SVGGElement, unknown> | null>(null);
  const minimapViewportRef = useRef<d3.Selection<SVGRectElement, unknown, null, undefined> | null>(null);
  const simNodesRef = useRef<GraphNode[]>([]);
  const minimapScaleRef = useRef<{ minX: number; minY: number; scale: number; ox: number; oy: number; mmW: number; mmH: number } | null>(null);

  // Neighborhood focus state (E.4)
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);

  // Minimap viewport updater (called from zoom handler)
  const updateMinimapViewport = useCallback((transform: d3.ZoomTransform, svgW: number, svgH: number) => {
    const vr = minimapViewportRef.current;
    const sc = minimapScaleRef.current;
    if (!vr || !sc) return;
    // Invert the viewport corners from SVG coords to graph coords
    const inv = transform.invert([0, 0]);
    const inv2 = transform.invert([svgW, svgH]);
    const vx = sc.ox + (inv[0] - sc.minX) * sc.scale;
    const vy = sc.oy + (inv[1] - sc.minY) * sc.scale;
    const vw = (inv2[0] - inv[0]) * sc.scale;
    const vh = (inv2[1] - inv[1]) * sc.scale;
    vr.attr('x', vx).attr('y', vy).attr('width', Math.max(0, vw)).attr('height', Math.max(0, vh));
  }, []);

  // Build graph data from props
  const graphData = useMemo(() => {
    const filteredNodes = filterType
      ? nodes.filter(n => n.node_type === filterType)
      : nodes;

    const nodeIds = new Set(filteredNodes.map(n => n.id));
    const filteredEdges = edges.filter(e => nodeIds.has(e.source) && nodeIds.has(e.target));

    const graphNodes: GraphNode[] = filteredNodes.map(n => ({ ...n }));
    const graphLinks: GraphLink[] = filteredEdges.map(e => ({
      source: e.source,
      target: e.target,
      edge_type: e.edge_type,
      metadata: e.metadata,
    }));

    return { graphNodes, graphLinks };
  }, [nodes, edges, filterType]);

  // Display name as-is — mockup spec: no truncation
  const displayName = useCallback((name: string): string => name, []);

  // Render the shape for each node type
  const renderNodeShape = useCallback((sel: d3.Selection<SVGGElement, GraphNode, SVGGElement, unknown>) => {
    sel.each(function (d) {
      const g = d3.select(this);
      const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
      const color = NODE_COLORS[d.node_type] || 'var(--color-text-faint)';

      switch (d.node_type) {
        case 'decision': {
          // Rounded rect
          g.append('rect')
            .attr('class', 'node-shape')
            .attr('rx', 6).attr('ry', 6)
            .attr('width', s.w).attr('height', s.h)
            .attr('x', -s.w / 2).attr('y', -s.h / 2)
            .attr('fill', color).attr('fill-opacity', 0.12)
            .attr('stroke', color).attr('stroke-width', 1.5).attr('stroke-opacity', 0.5);
          break;
        }
        case 'component': {
          // Square rect with sharp corners
          g.append('rect')
            .attr('class', 'node-shape')
            .attr('rx', 2).attr('ry', 2)
            .attr('width', s.w).attr('height', s.h)
            .attr('x', -s.w / 2).attr('y', -s.h / 2)
            .attr('fill', color).attr('fill-opacity', 0.12)
            .attr('stroke', color).attr('stroke-width', 1.5).attr('stroke-opacity', 0.5);
          break;
        }
        case 'pattern': {
          // Ellipse
          g.append('ellipse')
            .attr('class', 'node-shape')
            .attr('rx', s.w / 2).attr('ry', s.h / 2)
            .attr('cx', 0).attr('cy', 0)
            .attr('fill', color).attr('fill-opacity', 0.12)
            .attr('stroke', color).attr('stroke-width', 1.5).attr('stroke-opacity', 0.5);
          break;
        }
        case 'technology':
        case 'constraint':
        case 'quality_requirement': {
          // Path-based shapes (hexagon, diamond, shield)
          const pathD = getNodeShapePath(d.node_type, s.w, s.h);
          if (pathD) {
            g.append('path')
              .attr('class', 'node-shape')
              .attr('d', pathD)
              .attr('fill', color).attr('fill-opacity', 0.12)
              .attr('stroke', color).attr('stroke-width', 1.5).attr('stroke-opacity', 0.5);
          }
          break;
        }
      }
    });
  }, []);

  // Initialize the SVG graph
  useEffect(() => {
    const svgEl = svgRef.current;
    if (!svgEl || graphData.graphNodes.length === 0) return;

    const svg = d3.select(svgEl);
    const width = svgEl.clientWidth;
    const height = svgEl.clientHeight;

    // Clean up previous simulation
    if (simRef.current) {
      simRef.current.stop();
      simRef.current = null;
    }

    // Clear SVG
    svg.selectAll('*').remove();

    // Arrow marker defs
    const defs = svg.append('defs');
    const allEdgeTypes: string[] = ['depends_on', 'decided_by', 'constrains', 'uses', 'implements', 'satisfies', 'affects', 'supersedes'];
    allEdgeTypes.forEach(type => {
      const mSize = type === 'uses' ? 6 : 8;
      defs.append('marker')
        .attr('id', `arrow-${type}`)
        .attr('viewBox', '0 0 10 6')
        .attr('refX', 10)
        .attr('refY', 3)
        .attr('markerWidth', mSize)
        .attr('markerHeight', mSize * 0.75)
        .attr('orient', 'auto')
        .append('path')
        .attr('d', 'M0,0 L10,3 L0,6 Z')
        .attr('fill', edgeColor(type));
    });

    // Root group for zoom/pan
    const g = svg.append('g');
    gRef.current = g;

    // Zoom behavior (filter out dblclick to allow neighborhood focus)
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 4])
      .filter((event: Event) => !(event.type === 'dblclick'))
      .on('zoom', (event: d3.D3ZoomEvent<SVGSVGElement, unknown>) => {
        g.attr('transform', event.transform.toString());
        // Update minimap viewport indicator
        updateMinimapViewport(event.transform, width, height);
      });
    zoomRef.current = zoom;
    svg.call(zoom);

    // Initial centering
    svg.call(zoom.transform, d3.zoomIdentity.translate(width / 2 - 40, height / 2 - 30).scale(0.7));

    // Copy data for mutation
    const simNodes = graphData.graphNodes.map(d => ({ ...d }));
    const simLinks = graphData.graphLinks.map(d => ({ ...d }));
    simNodesRef.current = simNodes;

    // Render edges
    const edgeGroup = g.append('g').attr('class', 'edges');
    const linkSel = edgeGroup.selectAll<SVGLineElement, GraphLink>('line')
      .data(simLinks)
      .join('line')
      .attr('class', 'edge-line')
      .attr('stroke', d => edgeColor(d.edge_type))
      .attr('stroke-width', d => edgeWidth(d.edge_type))
      .attr('stroke-dasharray', d => edgeDash(d.edge_type))
      .attr('opacity', 0.55)
      .attr('marker-end', d => `url(#arrow-${d.edge_type})`);
    linkSelRef.current = linkSel;

    // Render nodes
    const nodeGroup = g.append('g').attr('class', 'nodes');
    const nodeSel = nodeGroup.selectAll<SVGGElement, GraphNode>('g')
      .data(simNodes)
      .join('g')
      .attr('class', 'node-group')
      .attr('tabindex', '0')
      .attr('role', 'button')
      .style('cursor', 'pointer');
    nodeSelRef.current = nodeSel;

    // Render distinct shapes per type
    renderNodeShape(nodeSel);

    // Type prefix text
    nodeSel.append('text')
      .attr('class', 'node-prefix')
      .attr('x', d => {
        const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
        if (d.node_type === 'constraint' || d.node_type === 'quality_requirement') return -s.w / 2 + 22;
        return -s.w / 2 + 10;
      })
      .attr('y', 1)
      .attr('dominant-baseline', 'middle')
      .attr('fill', d => NODE_COLORS[d.node_type] || 'var(--color-text-faint)')
      .attr('font-size', '9px')
      .attr('font-weight', '700')
      .attr('font-family', 'var(--font-mono)')
      .attr('letter-spacing', '0.06em')
      .text(d => TYPE_PREFIX[d.node_type] || '');

    // Node name text
    nodeSel.append('text')
      .attr('class', 'node-label')
      .attr('x', d => {
        const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
        if (d.node_type === 'constraint' || d.node_type === 'quality_requirement') return -s.w / 2 + 52;
        return -s.w / 2 + 44;
      })
      .attr('y', 1)
      .attr('dominant-baseline', 'middle')
      .attr('fill', 'var(--color-text)')
      .attr('font-size', '11px')
      .attr('font-weight', '500')
      .attr('font-family', 'var(--font-body)')
      .text(d => displayName(d.name));

    // Health indicators (stale/orphan dots)
    const STALE_DAYS = 30;
    const nowMs = Date.now();
    nodeSel.each(function(d) {
      const updMs = new Date(d.updated_at).getTime();
      const isStale = !isNaN(updMs) && (nowMs - updMs) > STALE_DAYS * 86400000;
      const isOrphan = !edgeData.some(e => e.source === d.id || e.target === d.id);
      if (!isStale && !isOrphan) return;
      const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
      const g = d3.select(this);
      let xOff = s.w / 2 - 8;
      if (isStale) {
        g.append('circle')
          .attr('cx', xOff).attr('cy', -s.h / 2 + 8)
          .attr('r', 4)
          .attr('fill', 'var(--color-warning)')
          .attr('stroke', 'var(--color-surface)').attr('stroke-width', 1.5);
        g.append('title').text(`Stale: not updated in ${STALE_DAYS}+ days`);
        xOff -= 10;
      }
      if (isOrphan) {
        g.append('circle')
          .attr('cx', xOff).attr('cy', -s.h / 2 + 8)
          .attr('r', 4)
          .attr('fill', 'var(--color-error)')
          .attr('stroke', 'var(--color-surface)').attr('stroke-width', 1.5);
        if (!isStale) g.append('title').text('Orphan: no connected edges');
      }
    });

    // Hover interactions
    nodeSel
      .on('mouseenter', function (_event, d) {
        onHoverNode(d.id);
        d3.select(this).select('.node-shape')
          .attr('stroke-opacity', 1)
          .attr('fill-opacity', 0.2);
        // Highlight connected edges
        linkSel
          .attr('opacity', e => {
            const src = typeof e.source === 'string' ? e.source : (e.source as GraphNode).id;
            const tgt = typeof e.target === 'string' ? e.target : (e.target as GraphNode).id;
            return (src === d.id || tgt === d.id) ? 0.95 : 0.06;
          })
          .attr('stroke-width', e => {
            const src = typeof e.source === 'string' ? e.source : (e.source as GraphNode).id;
            const tgt = typeof e.target === 'string' ? e.target : (e.target as GraphNode).id;
            return (src === d.id || tgt === d.id) ? 2.5 : 1;
          });
      })
      .on('mouseleave', function () {
        onHoverNode(null);
        d3.select(this).select('.node-shape')
          .attr('stroke-opacity', 0.5)
          .attr('fill-opacity', 0.12);
        linkSel
          .attr('opacity', 0.55)
          .attr('stroke-width', e => edgeWidth(e.edge_type));
      })
      .on('click', function (event, d) {
        event.stopPropagation();
        onSelectNode(d.id);
      })
      .on('dblclick', function (event, d) {
        event.stopPropagation();
        event.preventDefault();
        setFocusedNodeId(prev => prev === d.id ? null : d.id);
      });

    // Keyboard nav
    nodeSel.on('keydown', function (event: KeyboardEvent, d: GraphNode) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        event.stopPropagation();
        onSelectNode(d.id);
      }
    });

    // Click background to deselect + clear focus
    svg.on('click', () => {
      onSelectNode(null);
      setFocusedNodeId(null);
    });

    // Drag behavior
    function dragBehavior(sim: d3.Simulation<GraphNode, GraphLink>) {
      return d3.drag<SVGGElement, GraphNode>()
        .on('start', (event, d) => {
          if (!event.active) sim.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
        })
        .on('drag', (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on('end', (event, d) => {
          if (!event.active) sim.alphaTarget(0);
          d.fx = null;
          d.fy = null;
        });
    }

    // Type-based positioning forces (spreads node types into zones)
    const typeX: Record<string, number> = {
      decision: -180, technology: 350, component: 80,
      constraint: -420, pattern: 420, quality_requirement: 500,
    };
    const typeY: Record<string, number> = {
      decision: -60, technology: -40, component: 140,
      constraint: -280, pattern: 280, quality_requirement: 100,
    };

    // ─── Adaptive force parameters based on graph size (E.2) ─────────────
    const nodeCount = simNodes.length;
    const chargeStrength = nodeCount <= 8 ? -1200 : nodeCount <= 20 ? -1400 : nodeCount <= 50 ? -1800 : -2400;
    const linkDistance = nodeCount <= 8 ? 180 : nodeCount <= 20 ? 220 : 260;
    const typeForceStrength = nodeCount <= 8 ? 0.12 : 0.15;

    // Force simulation
    const sim = d3.forceSimulation(simNodes)
      .force('link', d3.forceLink<GraphNode, GraphLink>(simLinks)
        .id(d => d.id)
        .distance(linkDistance)
        .strength(0.35))
      .force('charge', d3.forceManyBody().strength(chargeStrength))
      .force('center', d3.forceCenter(0, 0))
      .force('collision', d3.forceCollide<GraphNode>().radius(d => {
        const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
        return Math.max(s.w, s.h) / 2 + 20;
      }))
      .force('x', d3.forceX<GraphNode>(d => typeX[d.node_type] || 0).strength(typeForceStrength))
      .force('y', d3.forceY<GraphNode>(d => typeY[d.node_type] || 0).strength(typeForceStrength));

    // ─── Pre-bake: run simulation to near-equilibrium before first paint (E.1) ─
    sim.stop();
    const preBakeTicks = Math.min(300, 100 + nodeCount * 8);
    sim.tick(preBakeTicks);

    // Position nodes + edges at pre-baked positions immediately
    linkSel
      .attr('x1', d => (d.source as GraphNode).x ?? 0)
      .attr('y1', d => (d.source as GraphNode).y ?? 0)
      .attr('x2', d => (d.target as GraphNode).x ?? 0)
      .attr('y2', d => (d.target as GraphNode).y ?? 0);
    nodeSel.attr('transform', d => `translate(${d.x ?? 0},${d.y ?? 0})`);

    // Now restart with low alpha for interactive settling + drag
    sim.alpha(0.1).restart();

    sim.on('tick', () => {
      linkSel
        .attr('x1', d => (d.source as GraphNode).x ?? 0)
        .attr('y1', d => (d.source as GraphNode).y ?? 0)
        .attr('x2', d => (d.target as GraphNode).x ?? 0)
        .attr('y2', d => (d.target as GraphNode).y ?? 0);

      nodeSel.attr('transform', d => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    simRef.current = sim;
    nodeSel.call(dragBehavior(sim));
    initializedRef.current = true;

    // ─── Minimap (E.3) ─────────────────────────────────────────────────────
    const MM_W = 160;
    const MM_H = 110;
    const MM_PAD = 12;
    const mmGroup = svg.append('g')
      .attr('class', 'minimap')
      .attr('transform', `translate(${width - MM_W - MM_PAD}, ${MM_PAD})`);
    // Background
    mmGroup.append('rect')
      .attr('width', MM_W).attr('height', MM_H)
      .attr('rx', 4)
      .attr('fill', 'var(--color-surface)').attr('fill-opacity', 0.85)
      .attr('stroke', 'var(--color-border)').attr('stroke-width', 1);
    // Node dots
    const mmNodeGroup = mmGroup.append('g').attr('class', 'mm-nodes');
    const mmNodes = mmNodeGroup.selectAll<SVGCircleElement, GraphNode>('circle')
      .data(simNodes)
      .join('circle')
      .attr('r', 2.5)
      .attr('fill', d => NODE_COLORS[d.node_type] || 'var(--color-text-faint)')
      .attr('fill-opacity', 0.8);
    minimapNodesRef.current = mmNodes;
    // Viewport rect
    const mmViewport = mmGroup.append('rect')
      .attr('class', 'mm-viewport')
      .attr('fill', 'var(--color-primary)').attr('fill-opacity', 0.08)
      .attr('stroke', 'var(--color-primary)').attr('stroke-width', 1).attr('stroke-opacity', 0.4)
      .attr('rx', 2);
    minimapViewportRef.current = mmViewport;

    // Minimap update function
    function updateMinimap() {
      if (!simNodesRef.current.length) return;
      const ns = simNodesRef.current;
      const xs = ns.map(n => n.x ?? 0);
      const ys = ns.map(n => n.y ?? 0);
      const pad = 60;
      const minX = Math.min(...xs) - pad;
      const maxX = Math.max(...xs) + pad;
      const minY = Math.min(...ys) - pad;
      const maxY = Math.max(...ys) + pad;
      const gw = maxX - minX || 1;
      const gh = maxY - minY || 1;
      const mmScale = Math.min((MM_W - 8) / gw, (MM_H - 8) / gh);
      const ox = (MM_W - gw * mmScale) / 2;
      const oy = (MM_H - gh * mmScale) / 2;
      minimapScaleRef.current = { minX, minY, scale: mmScale, ox, oy, mmW: MM_W, mmH: MM_H };
      mmNodes
        .attr('cx', d => ox + ((d.x ?? 0) - minX) * mmScale)
        .attr('cy', d => oy + ((d.y ?? 0) - minY) * mmScale);
    }
    // Initial minimap positions
    updateMinimap();
    // Also update minimap on tick
    sim.on('tick.minimap', updateMinimap);

    return () => {
      sim.stop();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphData, renderNodeShape, displayName]);

  // Apply filter opacity
  useEffect(() => {
    if (!nodeSelRef.current || !linkSelRef.current) return;
    if (filterType) {
      nodeSelRef.current.attr('opacity', d => d.node_type === filterType ? 1 : 0.15);
      linkSelRef.current.attr('opacity', 0.06);
    } else {
      nodeSelRef.current.attr('opacity', 1);
      linkSelRef.current.attr('opacity', 0.55);
    }
  }, [filterType]);

  // Neighborhood focus mode (E.4): dim non-neighbor nodes on dblclick
  useEffect(() => {
    if (!nodeSelRef.current || !linkSelRef.current) return;
    if (focusedNodeId) {
      // Build 1-hop neighbor set from edges
      const neighborIds = new Set<string>([focusedNodeId]);
      graphData.graphLinks.forEach(e => {
        const src = typeof e.source === 'string' ? e.source : (e.source as GraphNode).id;
        const tgt = typeof e.target === 'string' ? e.target : (e.target as GraphNode).id;
        if (src === focusedNodeId) neighborIds.add(tgt);
        if (tgt === focusedNodeId) neighborIds.add(src);
      });
      nodeSelRef.current.attr('opacity', d => neighborIds.has(d.id) ? 1 : 0.08);
      linkSelRef.current
        .attr('opacity', e => {
          const src = typeof e.source === 'string' ? e.source : (e.source as GraphNode).id;
          const tgt = typeof e.target === 'string' ? e.target : (e.target as GraphNode).id;
          return (src === focusedNodeId || tgt === focusedNodeId) ? 0.85 : 0.03;
        })
        .attr('stroke-width', e => {
          const src = typeof e.source === 'string' ? e.source : (e.source as GraphNode).id;
          const tgt = typeof e.target === 'string' ? e.target : (e.target as GraphNode).id;
          return (src === focusedNodeId || tgt === focusedNodeId) ? 2.5 : 1;
        });
    } else if (!filterType) {
      // Reset to default (only if no filter is active)
      nodeSelRef.current.attr('opacity', 1);
      linkSelRef.current
        .attr('opacity', 0.55)
        .attr('stroke-width', e => edgeWidth(e.edge_type));
    }
  }, [focusedNodeId, graphData.graphLinks, filterType]);

  // Resize handler
  useEffect(() => {
    const svgEl = svgRef.current;
    if (!svgEl) return;

    const observer = new ResizeObserver(() => {
      if (!initializedRef.current || !zoomRef.current) return;
      const width = svgEl.clientWidth;
      const height = svgEl.clientHeight;
      const svg = d3.select(svgEl);
      svg.call(
        zoomRef.current.transform,
        d3.zoomIdentity.translate(width / 2 - 40, height / 2 - 30).scale(0.7),
      );
    });
    observer.observe(svgEl);
    return () => observer.disconnect();
  }, []);

  // Empty state
  if (nodes.length === 0) {
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
    <svg
      ref={svgRef}
      style={{ width: '100%', height: '100%', background: 'transparent' }}
    />
  );
}
