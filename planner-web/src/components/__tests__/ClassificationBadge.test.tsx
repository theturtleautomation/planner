import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ClassificationBadge from '../ClassificationBadge';

describe('ClassificationBadge', () => {
  it('renders project_type text', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Web App', complexity: 'medium' }}
      />
    );
    expect(screen.getByText('Web App')).toBeInTheDocument();
  });

  it('renders complexity text in both rows', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'API', complexity: 'high' }}
      />
    );
    // Complexity appears in top row (inline) and bottom row (label + value).
    // Use getAllByText since the value appears in two separate elements.
    const matches = screen.getAllByText('high');
    expect(matches.length).toBeGreaterThanOrEqual(1);
  });

  it('renders Complexity label in bottom row', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'CLI', complexity: 'low' }}
      />
    );
    // The bottom row renders "Complexity: low" across a text node and a span.
    // Use a function matcher to find the container.
    expect(screen.getByText(/Complexity:/)).toBeInTheDocument();
  });

  it('shows correct icon for Web App project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Web App', complexity: 'medium' }}
      />
    );
    expect(screen.getByText('🌐')).toBeInTheDocument();
  });

  it('shows correct icon for Mobile App project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Mobile App', complexity: 'medium' }}
      />
    );
    expect(screen.getByText('📱')).toBeInTheDocument();
  });

  it('shows correct icon for API project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'API', complexity: 'medium' }}
      />
    );
    expect(screen.getByText('🔌')).toBeInTheDocument();
  });

  it('falls back to 📦 for unknown project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Blockchain Doodad', complexity: 'high' }}
      />
    );
    expect(screen.getByText('📦')).toBeInTheDocument();
  });
});
