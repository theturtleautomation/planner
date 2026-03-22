import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import DeleteNodeDialog from '../DeleteNodeDialog.tsx';

describe('DeleteNodeDialog', () => {
  it('shows the destructive modal copy and confirms deletion', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();

    render(
      <DeleteNodeDialog
        isOpen={true}
        nodeId="component-api"
        nodeName="API Service"
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    expect(screen.getByText(/remove this blueprint node and its linked edges/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /delete node/i }));

    expect(onConfirm).toHaveBeenCalledWith('component-api');
  });
});
