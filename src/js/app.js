const app = {
    setStatus(msg) { this.statusText = msg; },
    showError(err) {
        this.statusText = t('error') + ': ' + err;
        console.error(err);
    },

    async selectRepo() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: t('selectGitRepo'),
            });
            if (selected) {
                this.repoPath = selected;
                this.repoInfo = await api.openRepo(selected);
                this.selectedStartCommit = null;
                this.allCommits = [];
                await this.loadCommits();
            }
        } catch (e) { this.showError(e); }
    },

    async selectBundleFile() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                filters: [{ name: 'Git Bundle', extensions: ['bundle'] }],
                title: t('selectBundleFile'),
            });
            if (selected) {
                this.bundlePath = selected;
                this.setStatus(t('bundleSelected'));
            }
        } catch (e) { this.showError(e); }
    },
};
