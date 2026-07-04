const api = {
    openRepo: (path) => window.__TAURI__.core.invoke('open_repo', { path }),
    listCommits: (repoPath, limit) => window.__TAURI__.core.invoke('list_commits', { repoPath, limit }),
    getUnpushed: (repoPath) => window.__TAURI__.core.invoke('get_unpushed_commits', { repoPath }),
    getLastSync: (repoPath) => window.__TAURI__.core.invoke('get_last_sync', { repoPath }),

    previewExport: (repoPath) => window.__TAURI__.core.invoke('preview_export', { repoPath }),
    execExport: (repoPath, outputDir, from) =>
        window.__TAURI__.core.invoke('exec_export', { repoPath, outputDir, from }),

    verifyBundle: (bundlePath, repoPath) => window.__TAURI__.core.invoke('verify_bundle', { bundlePath, repoPath }),
    execImport: (repoPath, bundlePath) =>
        window.__TAURI__.core.invoke('exec_import', { repoPath, bundlePath }),

    getConflicts: (repoPath) => window.__TAURI__.core.invoke('get_conflicts', { repoPath }),
    autoResolve: (repoPath, conflicts) =>
        window.__TAURI__.core.invoke('auto_resolve_conflicts', { repoPath, conflicts }),
    applyResolution: (repoPath, filePath, hunks) =>
        window.__TAURI__.core.invoke('apply_resolution', { repoPath, filePath, hunks }),
    commitMerge: (repoPath, message) =>
        window.__TAURI__.core.invoke('commit_merge', { repoPath, message }),
    abortMerge: (repoPath) => window.__TAURI__.core.invoke('abort_merge', { repoPath }),
};
