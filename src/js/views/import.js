const importView = {
    async doExecImport() {
        if (!this.repoPath || !this.bundlePath) {
            this.showError(t('selectBothFirst'));
            return;
        }
        this.importState = 'importing';
        this.setStatus(t('importingBundle'));
        try {
            const result = await api.execImport(this.repoPath, this.bundlePath);
            if (result.type === 'Success') {
                this.importState = 'done';
                this.setStatus(t('importSuccess'));
            } else if (result.type === 'Conflicted') {
                this.importState = 'conflicted';
                this.setStatus(t('conflictsIn', { n: result.files.length }));
                this.conflicts = await api.getConflicts(this.repoPath);
                this.selectedFile = 0;
                this.selectedHunk = 0;
                this.currentView = 'conflict';
            } else if (result.type === 'AlreadyUpToDate') {
                this.importState = 'done';
                this.setStatus(t('alreadyUpToDate'));
            }
        } catch (e) { this.showError(e); this.importState = 'error'; }
    },
};
