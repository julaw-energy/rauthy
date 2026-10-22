export type RowDeletion<TData> = {
    /** Name shown in the confirmation, e.g. (c) => `${c.name} (${c.id})`. */
    label?: (row: TData) => string;
    /** Wording of the confirmation: verb (default "Delete") and the note under the title (default "This cannot be undone."). */
    verb?: string;
    description?: string;
    /** Rows for which this returns false get no delete button (e.g. the owner of an organization). */
    canRemove?: (row: TData) => boolean;
    /** Deletes the record. Throw to keep the dialog open and show the server's message. */
    run: (row: TData) => Promise<unknown>;
    /** Called after a successful delete, e.g. to invalidate and refetch the list. */
    onDeleted?: () => void | Promise<unknown>;
};
