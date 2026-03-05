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

  it('renders complexity text', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'API', complexity: 'high' }}
      />
    );
    expect(screen.getByText('high')).toBeInTheDocument();
  });

  it('renders complexity label', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'CLI', complexity: 'low' }}
      />
    );
    expect(screen.getByText(/complexity:/i)).toBeInTheDocument();
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
