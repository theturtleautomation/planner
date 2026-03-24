import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import CreateProjectModal from '../CreateProjectModal.tsx';

describe('CreateProjectModal', () => {
  it('submits when Enter is pressed in the project name field', async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);

    render(
      <CreateProjectModal
        isOpen={true}
        onClose={vi.fn()}
        onCreate={onCreate}
      />,
    );

    await user.type(screen.getByRole('textbox', { name: /project name/i }), 'Alpha Project{enter}');

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Alpha Project', undefined);
    });
  });

  it('submits when Enter is pressed in the description field', async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);

    render(
      <CreateProjectModal
        isOpen={true}
        onClose={vi.fn()}
        onCreate={onCreate}
      />,
    );

    await user.type(screen.getByRole('textbox', { name: /project name/i }), 'Alpha Project');
    await user.type(screen.getByRole('textbox', { name: /description \(optional\)/i }), 'Planning workspace{enter}');

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Alpha Project', 'Planning workspace');
    });
  });

  it('keeps Shift+Enter available for a newline in the description field', async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);

    render(
      <CreateProjectModal
        isOpen={true}
        onClose={vi.fn()}
        onCreate={onCreate}
      />,
    );

    const descriptionField = screen.getByRole('textbox', { name: /description \(optional\)/i });
    await user.type(descriptionField, 'Line one{shift>}{enter}{/shift}Line two');

    expect(descriptionField).toHaveValue('Line one\nLine two');
    expect(onCreate).not.toHaveBeenCalled();
  });
});
