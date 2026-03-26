import { AlertDialog } from "@kobalte/core";

import styles from "./ConfirmDialog.module.css";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  error?: string | null;
  confirmLabel: string;
  pending?: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void | Promise<void>;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
  return (
    <AlertDialog.Root open={props.open} onOpenChange={props.onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay class={styles.overlay} />
        <AlertDialog.Content class={styles.content}>
          <div class={styles.body}>
            <AlertDialog.Title class={styles.title}>{props.title}</AlertDialog.Title>
            <AlertDialog.Description class={styles.description}>
              {props.description}
            </AlertDialog.Description>
            {props.error ? <div class="error-copy">{props.error}</div> : null}
          </div>
          <div class={styles.actions}>
            <AlertDialog.CloseButton aria-label="Cancel" class="btn btn-subtle" disabled={props.pending}>
              Cancel
            </AlertDialog.CloseButton>
            <button
              class="btn btn-danger"
              disabled={props.pending}
              type="button"
              onClick={() => void props.onConfirm()}
            >
              {props.pending ? "Deleting…" : props.confirmLabel}
            </button>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
