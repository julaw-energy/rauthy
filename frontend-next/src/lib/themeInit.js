// Classic, blocking script loaded from <head> (see `routes/+layout.svelte`). It applies the saved
// theme before the first paint so there is no flash of the wrong theme on refresh.
//
// This must be an external file: the CSP forbids inline scripts (`script-src 'self'`), which is why
// mode-watcher's own inline head script is disabled.
(function () {
    try {
        var m = localStorage.getItem('mode-watcher-mode') || 'system';
        var dark = m === 'dark' || (m === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);

        var root = document.documentElement;
        root.classList.toggle('dark', dark);
        root.style.colorScheme = dark ? 'dark' : 'light';

        // the backend served theme works with classes on <body>, which does not exist yet
        var apply = function () {
            var body = document.body;
            if (!body) return false;
            body.classList.toggle('theme-dark', dark);
            body.classList.toggle('theme-light', !dark);
            if (getComputedStyle(body).getPropertyValue('--action').trim()) {
                body.classList.add('rauthy-theme');
            }
            return true;
        };
        if (!apply()) {
            var obs = new MutationObserver(function () {
                if (apply()) obs.disconnect();
            });
            obs.observe(root, { childList: true });
        }
    } catch (e) {
        // nothing to do - the app applies the theme when it starts
    }
})();
