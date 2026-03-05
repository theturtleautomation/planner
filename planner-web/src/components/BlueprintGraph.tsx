import { useEffect, useRef, useCallback, useMemo } from 'react';
import * as d3 from 'd3';
import type { GraphNode, GraphLink, NodeSummary, EdgePayload, NodeType, EdgeType } from '../types/blueprint.ts';

// ─── Node type → visual config ──────────────────────────────────────────────

const NODE_COLORS: Record<string, string> = {
  decision:            'var(--color-primary)',
  technology:          'var(--color-success)',
  component:           'var(--color-blue)',
  constraint:          'var(--color-warning)',
  pattern:             'var(--color-purple)',
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

  // Truncate name for display
  const displayName = useCallback((name: string): string => {
    if (name.length > 20) return name.slice(0, 19) + '\u2026';
    return name;
  }, []);

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

    // Zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 4])
      .on('zoom', (event: d3.D3ZoomEvent<SVGSVGElement, unknown>) => {
        g.attr('transform', event.transform.toString());
      });
    zoomRef.current = zoom;
    svg.call(zoom);

    // Initial centering
    svg.call(zoom.transform, d3.zoomIdentity.translate(width / 2 - 40, height / 2 - 30).scale(0.7));

    // Copy data for mutation
    const simNodes = graphData.graphNodes.map(d => ({ ...d }));
    const simLinks = graphData.graphLinks.map(d => ({ ...d }));

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
      });

    // Keyboard nav
    nodeSel.on('keydown', function (event: KeyboardEvent, d: GraphNode) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        event.stopPropagation();
        onSelectNode(d.id);
      }
    });

    // Click background to deselect
    svg.on('click', () => {
      onSelectNode(null);
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

    // Force simulation
    const sim = d3.forceSimulation(simNodes)
      .force('link', d3.forceLink<GraphNode, GraphLink>(simLinks)
        .id(d => d.id)
        .distance(220)
        .strength(0.35))
      .force('charge', d3.forceManyBody().strength(-1400))
      .force('center', d3.forceCenter(0, 0))
      .force('collision', d3.forceCollide<GraphNode>().radius(d => {
        const s = NODE_SIZES[d.node_type] || NODE_SIZES.decision;
        return Math.max(s.w, s.h) / 2 + 20;
      }))
      .force('x', d3.forceX<GraphNode>(d => typeX[d.node_type] || 0).strength(0.15))
      .force('y', d3.forceY<GraphNode>(d => typeY[d.node_type] || 0).strength(0.15));

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
