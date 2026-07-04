const importView = {
    async doVerifyBundle() {
        if (!this.bundlePath) { this.showError('Select a bundle file first'); return; }
        this.importState = 'verifying';
        this.setStatus('Verifying bundle...');
        try {
            this.bundleInfo = await api.verifyBundle(this.bundlePath);
            this.importState = 'idle';
            this.setStatus(`Bundle verified: ${this.bundleInfo.head_commit}`);
        } catch (e) { this.showError(e); this.importState = 'error'; }
    },

    async doExecImport() {
        if (!this.repoPath || !this.bundlePath) {
            this.showError('Select both repository and bundle file');
            return;
        }
        this.importState = 'importing';
        this.setStatus('Importing bundle...');
        try {
            const result = await api.execImport(this.repoPath, this.bundlePath);
            if (result.type === 'Success') {
                this.importState = 'done';
                this.setStatus('Import successful - merged cleanly');
            } else if (result.type === 'Conflicted') {
                this.importState = 'conflicted';
                this.setStatus(`Conflicts in ${result.files.length} file(s)`);
                this.conflicts = await api.getConflicts(this.repoPath);
                this.selectedFile = 0;
                this.selectedHunk = 0;
                this.currentView = 'conflict';
            } else if (result.type === 'AlreadyUpToDate') {
                this.importState = 'done';
                this.setStatus('Already up to date - nothing to import');
            }
        } catch (e) { this.showError(e); this.importState = 'error'; }
    },
};
