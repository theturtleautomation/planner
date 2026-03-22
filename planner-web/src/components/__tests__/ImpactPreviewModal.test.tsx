import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import ImpactPreviewModal from '../ImpactPreviewModal.tsx';
import type { ImpactReport } from '../../types/blueprint.ts';

const report: ImpactReport = {
  source_node_id: 'dec-auth',
  source_node_name: 'Auth Decision',
  summary: {
    add: 1,
    reconverge: 1,
  },
  entries: [
    {
      action: 'add',
      node_id: 'tech-redis',
      node_type: 'technology',
      explanation: 'Adds Redis as a new supporting dependency.',
    },
    {
      action: 'reconverge',
      node_id: 'component-api',
      node_type: 'component',
      explanation: 'Updates the API service to consume the new cache.',
      severity: 'deep',
    },
  ],
};

describe('ImpactPreviewModal', () => {
  it('renders the updated modal framing and allows apply/close actions', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onApply = vi.fn();

    render(
      <ImpactPreviewModal
        isOpen={true}
        report={report}
        loading={false}
        onClose={onClose}
        onApply={onApply}
      />,
    );

    expect(screen.getByText(/review the proposed graph changes/i)).toBeInTheDocument();
    expect(screen.getByText(/impact plan: auth decision/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /apply & reconverge/i }));
    expect(onApply).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: /close modal/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
