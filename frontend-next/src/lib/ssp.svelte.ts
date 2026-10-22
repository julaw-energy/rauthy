// Server side pagination (SSP) for Rauthy list endpoints.
//
// Small installations get the whole list with HTTP 200. Large ones get HTTP 206 plus the headers
// `x-page-size`, `x-page-count`, `x-continuation-token` (and an endpoint specific total count
// header). The server only knows "next" / "previous" page, so jumping pages walks there.
import type { PaginationState } from '@tanstack/svelte-table';
import { fetchGet } from '$api/fetch';

export function createSsp<T>(
    url: string,
    opts: { countHeader?: string; pageSize?: number; onError?: (msg: string) => void } = {},
) {
    let items: T[] = $state([]);
    let active = $state(false);
    let fetching = $state(false);
    let page = $state(1);
    let pageSize = $state(opts.pageSize ?? 20);
    let token: string | undefined | null = $state();
    let total = $state(0);

    async function load(params?: string): Promise<[number, Headers]> {
        fetching = true;
        let u = url;
        if (params) {
            u += `?${params}`;
        }

        let res = await fetchGet<T[]>(u);
        if (res.error) {
            opts.onError?.(`Error fetching ${url}: ${res.error.message}`);
        } else if (res.body) {
            if (res.status === 206) {
                active = true;
                pageSize = Number.parseInt(res.headers.get('x-page-size') || String(pageSize));
                token = res.headers.get('x-continuation-token');
                total = opts.countHeader ? Number.parseInt(res.headers.get(opts.countHeader) || '0') : 0;
            } else {
                active = false;
            }
            items = res.body;
        }
        fetching = false;

        return [res.status, res.headers];
    }

    /** (Re)load the first page with the current page size. */
    async function first() {
        token = undefined;
        page = 1;
        await load(`page_size=${pageSize}`);
    }

    /** Pass to `<DataTable handleFetch>`. */
    async function handleFetch(pg: PaginationState) {
        if (pg.pageSize !== pageSize) {
            pageSize = pg.pageSize;
            await first();
            return;
        }
        const target = pg.pageIndex + 1;
        if (target === 1) {
            await first();
            return;
        }
        while (page !== target) {
            const backwards = target < page;
            let params = `page_size=${pageSize}`;
            if (backwards) {
                if (page === 2) {
                    await first();
                    continue;
                }
                params += `&backwards=true&offset=${items.length - 1}`;
            }
            if (token) {
                params += `&continuation_token=${token}`;
            }
            await load(params);
            page += backwards ? -1 : 1;
        }
    }

    return {
        get items() { return items; },
        set items(v: T[]) { items = v; },
        get active() { return active; },
        get fetching() { return fetching; },
        get page() { return page; },
        get pageSize() { return pageSize; },
        get total() { return total; },
        get paginationData() { return { page, limit: pageSize, totalRows: total }; },
        load,
        first,
        handleFetch,
    };
}
