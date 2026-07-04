// app.js — shared helpers
const app = {
    setStatus(msg) { this.statusText = msg; },
    showError(err) {
        this.statusText = 'Error: ' + err;
        console.error(err);
    },

    async selectRepo() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: 'Select Git Repository',
            });
            if (selected) {
                this.repoPath = selected;
                this.repoInfo = await api.openRepo(selected);
                this.setStatus('Repository: ' + this.repoInfo.head_branch);
            }
        } catch (e) { this.showError(e); }
    },

    async selectBundleFile() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                filters: [{ name: 'Git Bundle', extensions: ['bundle'] }],
                title: 'Select Bundle File',
            });
            if (selected) {
                this.bundlePath = selected;
                this.bundleInfo = await api.verifyBundle(selected);
                this.setStatus('Bundle loaded: ' + this.bundleInfo.head_commit);
            }
        } catch (e) { this.showError(e); }
    },
};
