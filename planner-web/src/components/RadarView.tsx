import { useRef, useEffect, useCallback } from 'react';
import type { NodeSummary, NodeType } from '../types/blueprint.ts';

interface RadarViewProps {
  nodes: NodeSummary[];
  onSelectNode: (nodeId: string) => void;
}

// Technology categories to quadrants
const SUBTYPE_QUADRANT: Record<string, number> = {
  language: 0, runtime: 0, framework: 1, library: 2, tool: 3,
  platform: 1, database: 2, infrastructure: 3,
};

const QUADRANT_LABELS = ['Languages', 'Frameworks', 'Libraries', 'Tools'];
const RING_NAMES = ['Adopt', 'Trial', 'Assess', 'Hold'];
const RING_FRACTIONS = [0.3, 0.55, 0.78, 1.0];
const QUAD_ANGLES = [-Math.PI * 0.75, -Math.PI * 0.25, Math.PI * 0.25, Math.PI * 0.75];

// Maps node tags or names to derive a subtype (category)
function inferCategory(node: NodeSummary): string {
  const tags = node.tags.map(t => t.toLowerCase());
  for (const [cat] of Object.entries(SUBTYPE_QUADRANT)) {
    if (tags.includes(cat)) return cat;
  }
  // Fallback: look at the name
  const name = node.name.toLowerCase();
  if (name.includes('rust') || name.includes('python') || name.includes('typescript')) return 'language';
  if (name.includes('tokio') || name.includes('runtime')) return 'runtime';
  if (name.includes('axum') || name.includes('ratatui') || name.includes('react') || name.includes('framework')) return 'framework';
  if (name.includes('cli') || name.includes('tool') || name.includes('gemini')) return 'tool';
  return 'library';
}

// Maps ring from tags
function inferRing(node: NodeSummary): number {
  const tags = node.tags.map(t => t.toLowerCase());
  if (tags.includes('adopt')) return 0;
  if (tags.includes('trial')) return 1;
  if (tags.includes('assess')) return 2;
  if (tags.includes('hold')) return 3;
  return 1; // default to trial
}

export default function RadarView({ nodes, onSelectNode }: RadarViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Only show technology nodes
  const techs = nodes.filter(n => n.node_type === ('technology' as NodeType));

  const render = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;

    const containerW = container.clientWidth;
    const containerH = container.clientHeight;
    const size = Math.min(containerW, containerH) - 80;
    if (size < 100) return;

    const cx = size / 2;
    const cy = size / 2;
    const maxR = size / 2 - 40;

    const cs = getComputedStyle(document.documentElement);
    const textColor = cs.getPropertyValue('--color-text').trim();
    const textMuted = cs.getPropertyValue('--color-text-muted').trim();
    const textFaint = cs.getPropertyValue('--color-text-faint').trim();
    const divider = cs.getPropertyValue('--color-divider').trim();
    const primary = cs.getPropertyValue('--color-primary').trim();
    const blue = cs.getPropertyValue('--color-blue').trim();

    const rings = RING_FRACTIONS.map(f => maxR * f);

    let svg = `<svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" style="display:block;margin:auto">`;

    // Ring circles
    rings.forEach((r, i) => {
      svg += `<circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="${divider}" stroke-width="1"/>`;
      svg += `<text x="${cx + 6}" y="${cy - r + 14}" fill="${textFaint}" font-size="10" font-family="var(--font-body)" font-weight="600" letter-spacing="0.06em">${RING_NAMES[i].toUpperCase()}</text>`;
    });

    // Cross lines
    svg += `<line x1="${cx}" y1="${cy - maxR}" x2="${cx}" y2="${cy + maxR}" stroke="${divider}" stroke-width="0.5"/>`;
    svg += `<line x1="${cx - maxR}" y1="${cy}" x2="${cx + maxR}" y2="${cy}" stroke="${divider}" stroke-width="0.5"/>`;

    // Quadrant labels
    QUADRANT_LABELS.forEach((q, i) => {
      const angle = QUAD_ANGLES[i];
      const labelR = maxR + 20;
      const lx = cx + Math.cos(angle) * labelR;
      const ly = cy + Math.sin(angle) * labelR;
      svg += `<text x="${lx}" y="${ly}" fill="${textMuted}" font-size="10" font-family="var(--font-body)" font-weight="600" text-anchor="middle" letter-spacing="0.04em">${q}</text>`;
    });

    // Group techs by quadrant + ring for deterministic spacing
    interface QKey { quadIdx: number; ringIdx: number }
    const groups = new Map<string, { tech: NodeSummary; qk: QKey }[]>();

    techs.forEach(tech => {
      const cat = inferCategory(tech);
      const quadIdx = SUBTYPE_QUADRANT[cat] ?? 0;
      const ringIdx = inferRing(tech);
      const key = `${quadIdx}-${ringIdx}`;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push({ tech, qk: { quadIdx, ringIdx } });
    });

    techs.forEach(tech => {
      const cat = inferCategory(tech);
      const quadIdx = SUBTYPE_QUADRANT[cat] ?? 0;
      const ringIdx = inferRing(tech);
      const key = `${quadIdx}-${ringIdx}`;
      const group = groups.get(key) ?? [];
      const indexInGroup = group.findIndex(g => g.tech.id === tech.id);
      const countInGroup = group.length;

      const prevR = ringIdx > 0 ? rings[ringIdx - 1] : 0;
      const ringR = rings[ringIdx];
      const rFraction = countInGroup === 1 ? 0.5 : 0.3 + (indexInGroup / (countInGroup - 1)) * 0.5;
      const r = prevR + (ringR - prevR) * rFraction;

      const baseAngle = QUAD_ANGLES[quadIdx];
      const spread = 0.45;
      const angFraction = countInGroup === 1 ? 0 : (indexInGroup / (countInGroup - 1) - 0.5);
      const angle = baseAngle + angFraction * spread;

      const x = cx + Math.cos(angle) * r;
      const y = cy + Math.sin(angle) * r;
      const dotColor = ringIdx === 0 ? primary : blue;

      const escapedName = tech.name.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      const escapedId = tech.id.replace(/"/g, '&quot;');

      svg += `<circle cx="${x}" cy="${y}" r="6" fill="${dotColor}" opacity="0.9" style="cursor:pointer" data-tech-id="${escapedId}"/>`;
      svg += `<text x="${x + 10}" y="${y + 4}" fill="${textColor}" font-size="11" font-family="var(--font-body)" font-weight="500">${escapedName}</text>`;
    });

    svg += `</svg>`;
    container.innerHTML = svg;

    // Click handlers
    container.querySelectorAll('[data-tech-id]').forEach(el => {
      el.addEventListener('click', () => {
        const id = (el as SVGElement).getAttribute('data-tech-id');
        if (id) onSelectNode(id);
      });
    });
  }, [techs, onSelectNode]);

  useEffect(() => {
    render();
    const observer = new ResizeObserver(() => render());
    if (containerRef.current) observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [render]);

  if (techs.length === 0) {
    return (
      <div style={{
        width: '100%', height: '100%', display: 'flex',
        alignItems: 'center', justifyContent: 'center',
        color: 'var(--color-text-faint)', fontSize: 'var(--text-sm)',
      }}>
        No technology nodes to display on radar
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%', height: '100%', display: 'flex',
        alignItems: 'center', justifyContent: 'center',
      }}
    />
  );
}
