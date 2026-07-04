const conflictView = {
    get currentFile() {
        if (!this.conflicts || this.conflicts.length === 0) return null;
        return this.conflicts[this.selectedFile];
    },
    get currentHunk() {
        const f = this.currentFile;
        if (!f || !f.hunks || f.hunks.length === 0) return null;
        return f.hunks[this.selectedHunk];
    },
    get isCurrentFileResolved() {
        if (!this.currentFile) return false;
        const d = this.resolvedDecisions[this.currentFile.path];
        return d && d.length === this.currentFile.hunks.length;
    },
    isFileResolved(f) {
        const d = this.resolvedDecisions[f.path];
        return d && d.length === f.hunks.length;
    },

    nextHunk() { if (this.selectedHunk < this.currentFile.hunks.length - 1) this.selectedHunk++; },
    prevHunk() { if (this.selectedHunk > 0) this.selectedHunk--; },
    nextFile() {
        if (this.selectedFile < this.conflicts.length - 1) { this.selectedFile++; this.selectedHunk = 0; }
    },
    prevFile() {
        if (this.selectedFile > 0) { this.selectedFile--; this.selectedHunk = 0; }
    },
    selectFile(idx) { this.selectedFile = idx; this.selectedHunk = 0; },

    async doAutoResolve() {
        if (!this.repoPath || !this.conflicts) return;
        this.setStatus(t('analyzingConflicts'));
        try {
            this.autoReport = await api.autoResolve(this.repoPath, this.conflicts);
            this.setStatus(this.autoReport.summary);
            for (const r of this.autoReport.resolved) {
                for (const f of this.conflicts) {
                    for (const h of f.hunks) {
                        if (h.id === r.hunk_id) {
                            if (!this.resolvedDecisions[f.path]) this.resolvedDecisions[f.path] = [];
                            const idx = this.resolvedDecisions[f.path].findIndex(d => d.hunk_id === r.hunk_id);
                            if (idx >= 0) {
                                this.resolvedDecisions[f.path][idx] = r;
                            } else {
                                this.resolvedDecisions[f.path].push(r);
                            }
                        }
                    }
                }
            }
        } catch (e) { this.showError(e); }
    },

    takeLocal() {
        if (!this.currentHunk || !this.currentFile) return;
        this._record(this.currentFile.path, this.selectedHunk, { type: 'TakeLocal' }, this.currentHunk.local);
    },
    takeRemote() {
        if (!this.currentHunk || !this.currentFile) return;
        this._record(this.currentFile.path, this.selectedHunk, { type: 'TakeRemote' }, this.currentHunk.remote);
    },

    _record(filePath, hunkId, decision, content) {
        if (!this.resolvedDecisions[filePath]) this.resolvedDecisions[filePath] = [];
        const idx = this.resolvedDecisions[filePath].findIndex(d => d.hunk_id === hunkId);
        const entry = { hunk_id: hunkId, decision, merged_content: content, auto: false, confidence: 1.0 };
        if (idx >= 0) { this.resolvedDecisions[filePath][idx] = entry; }
        else { this.resolvedDecisions[filePath].push(entry); }
    },

    async applyCurrentFile() {
        if (!this.currentFile) return;
        const decisions = this.resolvedDecisions[this.currentFile.path];
        if (!decisions || decisions.length !== this.currentFile.hunks.length) {
            this.showError(t('resolveHunksFirst'));
            return;
        }
        this.setStatus(t('applyingResolution'));
        try {
            await api.applyResolution(this.repoPath, this.currentFile.path, decisions);
            this.setStatus(`${t('applied')}: ${this.currentFile.path}`);
            const allResolved = this.conflicts.every(f => {
                const d = this.resolvedDecisions[f.path];
                return d && d.length === f.hunks.length;
            });
            if (allResolved) this.setStatus(t('allResolved'));
        } catch (e) { this.showError(e); }
    },

    async doCommitMerge() {
        this.setStatus(t('committingMerge'));
        try {
            await api.commitMerge(this.repoPath, 'Merge from GitSneaker bundle');
            this.setStatus(t('mergeCommitted'));
            this.currentView = 'export';
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); }
    },

    async doAbortMerge() {
        try {
            await api.abortMerge(this.repoPath);
            this.setStatus(t('mergeAborted'));
            this.currentView = 'export';
            this.conflicts = [];
            this.resolvedDecisions = {};
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); }
    },
};
