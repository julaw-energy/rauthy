// The service can be branded with its own name (config `server.app_name`, editable under
// Config -> General). The UI texts and page titles say "Rauthy"; this module swaps in the
// configured name.
//
// The name is remembered in localStorage, so the page can be branded synchronously on the next
// visit. If the server reports a different name than the one applied, the page reloads once.

const KEY = 'app_name';
const DEFAULT = 'Rauthy';

let applied = DEFAULT;
let current = $state(DEFAULT);

export const brand = {
    /** the name to show, reactive */
    get name() {
        return current;
    },
};

function replaceDeep(obj: unknown, name: string, seen = new Set<unknown>()) {
    if (!obj || typeof obj !== 'object' || seen.has(obj)) return;
    seen.add(obj);
    const o = obj as Record<string, unknown>;
    for (const k of Object.keys(o)) {
        const v = o[k];
        if (typeof v === 'string') {
            if (v.includes(DEFAULT)) o[k] = v.replace(/\bRauthy\b/g, () => name);
        } else {
            replaceDeep(v, name, seen);
        }
    }
}

/** Replaces "Rauthy" in the (static) translation texts. Call after each `initI18n*()`. */
export function brandI18n(obj: unknown) {
    if (applied !== DEFAULT) replaceDeep(obj, applied);
}

function fixTitle() {
    if (applied !== DEFAULT && document.title.startsWith(DEFAULT)) {
        document.title = applied + document.title.slice(DEFAULT.length);
    }
}

export function initBranding() {
    if (typeof window === 'undefined') return;

    try {
        applied = localStorage.getItem(KEY) || DEFAULT;
    } catch {
        applied = DEFAULT;
    }
    current = applied;

    if (applied !== DEFAULT) {
        // every page sets its own `<title>Rauthy ...</title>`
        new MutationObserver(fixTitle).observe(document.head, {
            subtree: true,
            childList: true,
            characterData: true,
        });
        fixTitle();
    }

    fetch('/auth/v1/app_name')
        .then(r => (r.ok ? r.json() : undefined))
        .then((j?: { name?: string }) => {
            if (!j?.name) return;
            try {
                localStorage.setItem(KEY, j.name);
            } catch {
                // fine, the name is applied on the next visit instead
            }
            if (j.name !== applied) {
                // texts were already swapped (or not) with the old name, a reload is the clean way
                if (sessionStorage.getItem('app_name_reload') !== j.name) {
                    sessionStorage.setItem('app_name_reload', j.name);
                    location.reload();
                }
            }
        })
        .catch(() => {});
}
