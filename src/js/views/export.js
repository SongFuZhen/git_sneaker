const exportView = {
    async doPreviewExport() {
        if (!this.repoPath) { this.showError('Select a repository first'); return; }
        this.exportState = 'loading';
        this.setStatus('Scanning commits...');
        try {
            this.exportPreview = await api.previewExport(this.repoPath);
            this.exportState = 'idle';
            this.setStatus(`${this.exportPreview.commits.length} commits to sync`);
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },

    async doExecExport() {
        if (!this.repoPath || !this.exportPreview) return;
        this.exportState = 'exporting';
        this.setStatus('Creating bundle...');
        try {
            const result = await api.execExport(this.repoPath, this.repoPath);
            this.exportState = 'done';
            this.setStatus(`Bundle: ${result.file_path} (${(result.file_size / 1024).toFixed(1)} KB)`);
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },

    async doSelectExportOutput() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: 'Select Output Directory (e.g., USB drive)',
            });
            if (selected && this.repoPath && this.exportPreview) {
                this.exportState = 'exporting';
                this.setStatus('Creating bundle...');
                const result = await api.execExport(this.repoPath, selected);
                this.exportState = 'done';
                this.setStatus(`Bundle saved: ${result.file_path}`);
                this.repoInfo = await api.openRepo(this.repoPath);
            }
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },
};
