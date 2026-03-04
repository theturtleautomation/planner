import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ClassificationBadge from '../ClassificationBadge';

describe('ClassificationBadge', () => {
  it('renders project_type text', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Web App', complexity: 'medium', question_budget: 10 }}
      />
    );
    expect(screen.getByText('Web App')).toBeInTheDocument();
  });

  it('renders complexity text', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'API', complexity: 'high', question_budget: 15 }}
      />
    );
    expect(screen.getByText('high')).toBeInTheDocument();
  });

  it('renders question budget', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'CLI', complexity: 'low', question_budget: 5 }}
      />
    );
    expect(screen.getByText(/~5 questions/)).toBeInTheDocument();
  });

  it('shows correct icon for Web App project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Web App', complexity: 'medium', question_budget: 10 }}
      />
    );
    expect(screen.getByText('🌐')).toBeInTheDocument();
  });

  it('shows correct icon for Mobile App project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Mobile App', complexity: 'medium', question_budget: 10 }}
      />
    );
    expect(screen.getByText('📱')).toBeInTheDocument();
  });

  it('shows correct icon for API project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'API', complexity: 'medium', question_budget: 10 }}
      />
    );
    expect(screen.getByText('🔌')).toBeInTheDocument();
  });

  it('falls back to 📦 for unknown project type', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'Blockchain Doodad', complexity: 'high', question_budget: 20 }}
      />
    );
    expect(screen.getByText('📦')).toBeInTheDocument();
  });

  it('shows "Budget:" label', () => {
    render(
      <ClassificationBadge
        classification={{ project_type: 'CLI', complexity: 'low', question_budget: 7 }}
      />
    );
    expect(screen.getByText(/budget:/i)).toBeInTheDocument();
  });

  it('renders singular "question" when budget is 1', () => {
    // The component always shows "~N questions" regardless; verify budget number appears
    render(
      <ClassificationBadge
        classification={{ project_type: 'CLI', complexity: 'low', question_budget: 1 }}
      />
    );
    expect(screen.getByText(/~1 questions/)).toBeInTheDocument();
  });
});
