// Events that arrived while nobody was looking at the event list. The admin layout feeds it from its
// own light event stream, the bell buttons show it. Reset it when the list becomes visible.
export const unread = $state({ count: 0 });
