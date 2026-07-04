const exportView = {
    allCommits: [],
    selectedStartCommit: null,
    commitLimit: 50,
    exportMode: 'full',

    async loadCommits() {
        if (!this.repoPath) return;
        this.setStatus(t('scanningCommits'));
        try {
            this.allCommits = await api.listCommits(this.repoPath, this.commitLimit);
            this.setStatus(t('ready'));
        } catch (e) { this.showError(e); }
    },

    async loadMoreCommits() {
        if (!this.repoPath) return;
        this.commitLimit += 50;
        await this.loadCommits();
    },

    selectStartCommit(commit) {
        this.selectedStartCommit = commit;
    },

    async doFullExport() {
        if (!this.repoPath) return;
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: t('selectOutputDir'),
            });
            if (selected) {
                this.exportState = 'exporting';
                this.exportMode = 'full';
                this.setStatus(t('creatingBundle'));
                const result = await api.execExport(this.repoPath, selected, null);
                this.exportState = 'done';
                this.setStatus(`${t('bundleSaved')}: ${result.file_path}`);
                this.repoInfo = await api.openRepo(this.repoPath);
            }
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },

    async doSelectExportOutput() {
        if (!this.repoPath) return;
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: t('selectOutputDir'),
            });
            if (selected) {
                this.exportState = 'exporting';
                this.setStatus(t('creatingBundle'));
                const from = this.exportMode === 'incremental' && this.selectedStartCommit
                    ? this.selectedStartCommit.full_hash
                    : null;
                const result = await api.execExport(this.repoPath, selected, from);
                this.exportState = 'done';
                this.setStatus(`${t('bundleSaved')}: ${result.file_path}`);
                this.repoInfo = await api.openRepo(this.repoPath);
            }
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },
};
