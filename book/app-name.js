// Shows the configured service name instead of "Rauthy" in the built-in docs (served by Rauthy
// itself at /auth/v1/book/). Code blocks stay untouched, so commands and image names keep working.
(function () {
    var DEFAULT = 'Rauthy';
    var KEY = 'app_name';
    var applied = DEFAULT;

    function swap(name) {
        if (!name || name === DEFAULT) return;
        var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
            acceptNode: function (n) {
                if (n.nodeValue.indexOf(DEFAULT) === -1) return NodeFilter.FILTER_REJECT;
                for (var p = n.parentNode; p && p !== document.body; p = p.parentNode) {
                    if (/^(CODE|PRE|SCRIPT|STYLE|TEXTAREA)$/.test(p.nodeName)) return NodeFilter.FILTER_REJECT;
                }
                return NodeFilter.FILTER_ACCEPT;
            },
        });
        var nodes = [];
        while (walker.nextNode()) nodes.push(walker.currentNode);
        nodes.forEach(function (n) {
            n.nodeValue = n.nodeValue.replace(/\bRauthy\b/g, function () { return name; });
        });
        document.title = document.title.replace(/\bRauthy\b/g, function () { return name; });
        applied = name;
    }

    function run() {
        var cached = null;
        try { cached = localStorage.getItem(KEY); } catch (e) {}
        swap(cached);

        fetch('/auth/v1/app_name')
            .then(function (r) { return r.ok ? r.json() : null; })
            .then(function (j) {
                if (!j || !j.name) return;
                try { localStorage.setItem(KEY, j.name); } catch (e) {}
                // the cached name was wrong or missing: apply the real one now
                if (j.name !== applied && applied === DEFAULT) swap(j.name);
            })
            .catch(function () {});
    }

    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', run);
    else run();
})();
