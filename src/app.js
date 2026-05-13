const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

function app() {
  return {
    version: '0.1.0',
    dark: false,
    dumpLoaded: false,
    parsing: false,
    dumpStats: { creances: 0, elapsed_ms: 0 },

    // Catalog + columns
    columnsCatalog: [],
    presets: {},
    personalProfiles: {},
    selectedColumns: [],
    activeProfile: '',

    // Filter state
    filters: {
      uges: [],
      natureCompte: [],
      commentaireContient: '',
      commentaireInsensible: true,
      notifCriterion: { kind: 'aucun' },
      datePivot: 'date_integration',
      dateMin: null,
      dateMax: null,
    },
    notifKind: 'aucun',
    etapeIds: [],
    statutValues: [],

    // Distinct values (loaded after parse)
    uges: [],
    natures: [],
    etapes: [],
    statuts: [],

    // Result
    estimatedCount: 0,
    previewOpen: false,
    previewRows: [],
    toast: '',

    async init() {
      // Match system theme
      try {
        this.dark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
      } catch (_) { this.dark = false; }

      try {
        const cat = await invoke('list_columns');
        this.columnsCatalog = cat.columns;
        this.presets = cat.presets;
      } catch (e) { console.error('list_columns', e); }

      try {
        const p = await invoke('load_profiles');
        this.personalProfiles = (p && p.profiles) ? p.profiles : {};
      } catch (e) { console.error('load_profiles', e); }

      try {
        const sess = await invoke('load_session');
        if (sess && sess.filters && Object.keys(sess.filters).length) {
          Object.assign(this.filters, sess.filters);
        }
        if (sess && Array.isArray(sess.columns) && sess.columns.length) {
          this.selectedColumns = sess.columns;
        }
        if (sess && sess.activeProfile) {
          this.activeProfile = sess.activeProfile;
        }
      } catch (e) { console.error('load_session', e); }

      // If nothing loaded, default to Standard CAMIEG
      if (this.selectedColumns.length === 0 && this.presets['Standard CAMIEG']) {
        this.selectedColumns = [...this.presets['Standard CAMIEG']];
      }

      // Watch and persist
      this.$watch('filters', () => { this.refreshEstimate(); this.persistSession(); }, { deep: true });
      this.$watch('selectedColumns', () => this.persistSession(), { deep: true });
      this.$watch('activeProfile', () => this.persistSession());
      this.$watch('notifKind', () => this.syncNotif());
      this.$watch('etapeIds', () => this.syncNotif(), { deep: true });
      this.$watch('statutValues', () => this.syncNotif(), { deep: true });
    },

    get groupedColumns() {
      const groups = {};
      this.columnsCatalog.forEach(c => {
        if (!groups[c.group]) groups[c.group] = { name: c.group, cols: [] };
        groups[c.group].cols.push(c);
      });
      return Object.values(groups);
    },

    labelOf(id) {
      const c = this.columnsCatalog.find(c => c.id === id);
      return c ? c.label : id;
    },

    applyProfile() {
      if (!this.activeProfile) return;
      if (this.activeProfile.startsWith('_')) {
        const name = this.activeProfile.slice(1);
        this.selectedColumns = [...(this.personalProfiles[name] || [])];
      } else {
        const cols = this.presets[this.activeProfile];
        if (cols) this.selectedColumns = [...cols];
      }
    },

    async savePersonalProfile() {
      const name = prompt('Nom du profil personnel :');
      if (!name) return;
      try {
        await invoke('save_profile', { name, cols: this.selectedColumns });
        this.personalProfiles[name] = [...this.selectedColumns];
        this.toastMsg('Profil sauvegardé');
      } catch (e) { alert('Erreur : ' + e); }
    },

    async pickFile() {
      try {
        const path = await open({ multiple: false, filters: [{ name: 'Dump SUCRE', extensions: ['dump', 'sql'] }] });
        if (path) await this.loadDump(path);
      } catch (e) { alert('Erreur : ' + e); }
    },

    async onDrop(ev) {
      const files = ev.dataTransfer && ev.dataTransfer.files;
      if (!files || !files.length) return;
      const file = files[0];
      // In Tauri 2 file drops, the dropped file path is on `file.path` (WebKit). Fallback to file.name (not usable as a path).
      const path = file.path || file.name;
      if (path) await this.loadDump(path);
    },

    async loadDump(path) {
      this.parsing = true;
      try {
        const stats = await invoke('parse_dump', { path });
        this.dumpStats = stats;
        this.dumpLoaded = true;
        const [uges, natures, etapes, statuts] = await Promise.all([
          invoke('list_uges'),
          invoke('list_natures'),
          invoke('list_etapes'),
          invoke('list_statuts'),
        ]);
        this.uges = uges;
        this.natures = natures;
        this.etapes = etapes;
        this.statuts = statuts;
        await this.refreshEstimate();
      } catch (e) {
        alert('Erreur de parsing : ' + e);
      } finally {
        this.parsing = false;
      }
    },

    resetDump() {
      this.dumpLoaded = false;
      this.dumpStats = { creances: 0, elapsed_ms: 0 };
      this.estimatedCount = 0;
    },

    syncNotif() {
      switch (this.notifKind) {
        case 'aucun': this.filters.notifCriterion = { kind: 'aucun' }; break;
        case 'motif_notif_non_vide': this.filters.notifCriterion = { kind: 'motif_notif_non_vide' }; break;
        case 'date_ar_notif_non_vide': this.filters.notifCriterion = { kind: 'date_ar_notif_non_vide' }; break;
        case 'etape_wf_dans': this.filters.notifCriterion = { kind: 'etape_wf_dans', ids: [...this.etapeIds] }; break;
        case 'statut_compte_dans': this.filters.notifCriterion = { kind: 'statut_compte_dans', values: [...this.statutValues] }; break;
      }
    },

    async refreshEstimate() {
      if (!this.dumpLoaded) { this.estimatedCount = 0; return; }
      try {
        this.estimatedCount = await invoke('count_filtered', { filters: this.filters });
      } catch (_) { /* ignore transient */ }
    },

    async openPreview() {
      try {
        this.previewRows = await invoke('preview', { filters: this.filters, columns: this.selectedColumns });
        this.previewOpen = true;
      } catch (e) { alert('Erreur aperçu : ' + e); }
    },

    async exportXlsx() {
      const ugePart = this.filters.uges.length ? this.filters.uges.join('-') : 'toutesUGE';
      const now = new Date();
      const stamp = now.getFullYear()
                  + String(now.getMonth() + 1).padStart(2, '0')
                  + String(now.getDate()).padStart(2, '0')
                  + '-'
                  + String(now.getHours()).padStart(2, '0')
                  + String(now.getMinutes()).padStart(2, '0');
      try {
        const path = await save({
          defaultPath: `raffinerie_${ugePart}_${stamp}.xlsx`,
          filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        });
        if (!path) return;
        await invoke('export_xlsx', { path, filters: this.filters, columns: this.selectedColumns });
        this.toastMsg('✓ Exporté : ' + path);
      } catch (e) {
        alert('Erreur export : ' + e);
      }
    },

    async persistSession() {
      const sess = {
        filters: this.filters,
        activeProfile: this.activeProfile,
        columns: this.selectedColumns,
      };
      try { await invoke('save_session', { session: sess }); } catch (_) {}
    },

    toastMsg(msg) {
      this.toast = msg;
      setTimeout(() => { this.toast = ''; }, 4000);
    },
  };
}
window.app = app;
