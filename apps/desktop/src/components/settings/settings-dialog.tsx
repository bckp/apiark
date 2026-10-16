import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, FolderOpen, Download, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { LANGUAGES } from "@/lib/i18n";
import { useSettingsStore } from "@/stores/settings-store";
import type { AppSettings } from "@apiark/types";
import { open as openFileDialog, save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import { exportAppState, importAppState } from "@/lib/tauri-api";
import { tokenSwatchGroups } from "@/styles/tokens";

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const { settings, updateSettings } = useSettingsStore();
  const { t, i18n } = useTranslation();

  const update = (patch: Partial<AppSettings>) => {
    updateSettings(patch);
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-[520px] max-h-[85vh] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] shadow-xl focus:outline-none">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-[var(--color-border)] px-6 py-4">
            <Dialog.Title className="text-lg font-semibold text-[var(--color-text-primary)]">
              {t("settings.title")}
            </Dialog.Title>
            <Dialog.Close className="rounded p-1 text-[var(--color-text-muted)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text-primary)]">
              <X className="h-4 w-4" />
            </Dialog.Close>
          </div>

          <div className="space-y-6 p-6">
            {/* Appearance Section */}
            <section>
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
                {t("settings.appearance")}
              </h3>

              {/* Theme */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.theme")}
                </label>
                <div className="flex gap-2">
                  {(["system", "light", "dark", "black"] as const).map((themeOption) => (
                    <button
                      key={themeOption}
                      onClick={() => update({ theme: themeOption })}
                      className={`rounded px-4 py-2 text-sm capitalize transition-colors ${
                        settings.theme === themeOption
                          ? "bg-[var(--color-accent)] text-white"
                          : "bg-[var(--color-elevated)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
                      }`}
                    >
                      {themeOption === "black" ? t("settings.black") : themeOption}
                    </button>
                  ))}
                </div>
              </div>

              {/* Accent Color */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.accentColor")}
                </label>
                <div className="flex gap-2">
                  {([
                    { name: "indigo", color: "#6366f1" },
                    { name: "blue", color: "#3b82f6" },
                    { name: "emerald", color: "#10b981" },
                    { name: "amber", color: "#f59e0b" },
                    { name: "rose", color: "#f43f5e" },
                    { name: "violet", color: "#8b5cf6" },
                    { name: "cyan", color: "#06b6d4" },
                    { name: "orange", color: "#f97316" },
                  ] as const).map((a) => (
                    <button
                      key={a.name}
                      onClick={() => update({ accentColor: a.name })}
                      title={a.name.charAt(0).toUpperCase() + a.name.slice(1)}
                      className={`h-7 w-7 rounded-full transition-all ${
                        settings.accentColor === a.name
                          ? "scale-110 ring-2 ring-offset-2 ring-offset-[var(--color-surface)]"
                          : "hover:scale-110 opacity-70 hover:opacity-100"
                      }`}
                      style={{
                        backgroundColor: a.color,
                        // Use inline style for ring color since it's dynamic
                        ...(settings.accentColor === a.name ? { ["--tw-ring-color" as string]: a.color } : {}),
                      }}
                    />
                  ))}
                </div>
              </div>

              {/* Layout */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.panelLayout")}
                </label>
                <div className="flex gap-2">
                  {([
                    { value: "horizontal" as const, label: t("settings.sideBySide") },
                    { value: "vertical" as const, label: t("settings.stacked") },
                    { value: "tabbed" as const, label: t("settings.tabbed") },
                  ]).map((l) => (
                    <button
                      key={l.value}
                      onClick={() => update({ layout: l.value })}
                      className={`rounded-lg border px-4 py-2 text-sm transition-all ${
                        settings.layout === l.value
                          ? "border-[var(--color-accent)] bg-[var(--color-accent)]/10 text-[var(--color-accent)]"
                          : "border-[var(--color-border)] text-[var(--color-text-muted)] hover:border-[var(--color-text-dimmed)]"
                      }`}
                    >
                      {l.label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Sidebar width */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.sidebarWidth")}: {settings.sidebarWidth}px
                </label>
                <input
                  type="range"
                  min={180}
                  max={400}
                  step={4}
                  value={settings.sidebarWidth}
                  onChange={(e) => update({ sidebarWidth: Number(e.target.value) })}
                  className="w-full accent-[var(--color-accent)]"
                />
              </div>

              {/* Language */}
              <div>
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.language")}
                </label>
                <div className="flex flex-wrap gap-2">
                  {LANGUAGES.map((lang) => (
                    <button
                      key={lang.code}
                      onClick={() => {
                        i18n.changeLanguage(lang.code);
                        localStorage.setItem("apiark-language", lang.code);
                      }}
                      className={`rounded px-3 py-1.5 text-sm transition-colors ${
                        i18n.language === lang.code
                          ? "bg-[var(--color-accent)] text-white"
                          : "bg-[var(--color-elevated)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
                      }`}
                    >
                      {lang.label}
                    </button>
                  ))}
                </div>
              </div>
            </section>

            {/* Network Section */}
            <section>
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
                {t("settings.network")}
              </h3>

              {/* SSL Verification */}
              <div className="mb-3 flex items-center justify-between">
                <label className="text-sm text-[var(--color-text-secondary)]">
                  {t("settings.sslVerification")}
                </label>
                <ToggleSwitch
                  checked={settings.verifySsl}
                  onChange={(v) => update({ verifySsl: v })}
                />
              </div>

              {/* Follow Redirects */}
              <div className="mb-3 flex items-center justify-between">
                <label className="text-sm text-[var(--color-text-secondary)]">
                  {t("settings.followRedirects")}
                </label>
                <ToggleSwitch
                  checked={settings.followRedirects}
                  onChange={(v) => update({ followRedirects: v })}
                />
              </div>

              {/* Timeout */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.timeout")}
                </label>
                <input
                  type="number"
                  min={0}
                  step={1000}
                  value={settings.timeoutMs}
                  onChange={(e) => update({ timeoutMs: Number(e.target.value) || 30000 })}
                  className="w-full rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                />
              </div>

              {/* Proxy */}
              <div>
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.proxyUrl")}
                </label>
                <input
                  type="text"
                  value={settings.proxyUrl ?? ""}
                  onChange={(e) => update({ proxyUrl: e.target.value || null })}
                  placeholder="http://proxy.example.com:8080"
                  className="mb-2 w-full rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] placeholder-[var(--color-text-dimmed)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                />
                {settings.proxyUrl && (
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={settings.proxyUsername ?? ""}
                      onChange={(e) => update({ proxyUsername: e.target.value || null })}
                      placeholder={t("settings.proxyUsername")}
                      className="flex-1 rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] placeholder-[var(--color-text-dimmed)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                    />
                    <input
                      type="password"
                      value={settings.proxyPassword ?? ""}
                      onChange={(e) => update({ proxyPassword: e.target.value || null })}
                      placeholder={t("settings.proxyPassword")}
                      className="flex-1 rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] placeholder-[var(--color-text-dimmed)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                    />
                  </div>
                )}
              </div>
            </section>

            {/* Certificates Section */}
            <section>
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
                {t("settings.certificates")}
              </h3>

              {/* Custom CA Certificate */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.caCert")}
                </label>
                <FilePathInput
                  value={settings.caCertPath ?? ""}
                  onChange={(v) => update({ caCertPath: v || null })}
                  placeholder="Path to CA certificate (.pem, .crt)"
                  filters={[{ name: "Certificates", extensions: ["pem", "crt", "cer"] }]}
                />
              </div>

              {/* Client Certificate */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.clientCert")}
                </label>
                <FilePathInput
                  value={settings.clientCertPath ?? ""}
                  onChange={(v) => update({ clientCertPath: v || null })}
                  placeholder="Path to client certificate (.pem, .crt, .pfx)"
                  filters={[{ name: "Certificates", extensions: ["pem", "crt", "cer", "pfx", "p12"] }]}
                />
              </div>

              {/* Client Key */}
              <div className="mb-4">
                <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                  {t("settings.clientKey")}
                </label>
                <FilePathInput
                  value={settings.clientKeyPath ?? ""}
                  onChange={(v) => update({ clientKeyPath: v || null })}
                  placeholder="Path to client private key (.pem, .key)"
                  filters={[{ name: "Keys", extensions: ["pem", "key"] }]}
                />
              </div>

              {/* PFX Passphrase */}
              {settings.clientCertPath?.match(/\.(pfx|p12)$/i) && (
                <div className="mb-4">
                  <label className="mb-2 block text-sm text-[var(--color-text-secondary)]">
                    {t("settings.pfxPassphrase")}
                  </label>
                  <input
                    type="password"
                    value={settings.clientCertPassphrase ?? ""}
                    onChange={(e) => update({ clientCertPassphrase: e.target.value || null })}
                    placeholder="Passphrase (optional)"
                    className="w-full rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] placeholder-[var(--color-text-dimmed)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                  />
                </div>
              )}
            </section>

            {/* AI Section */}
            {/* Keyboard Shortcuts */}


            {/* Backup Section */}
            <BackupSection />

            {/* Design Tokens Reference */}
            <DesignTokensSection />
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function BackupSection() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<string | null>(null);
  const [includeHistory, setIncludeHistory] = useState(true);
  const [importConfirm, setImportConfirm] = useState<string | null>(null);

  const handleExport = async () => {
    const path = await saveFileDialog({
      defaultPath: `apiark-backup-${new Date().toISOString().slice(0, 10)}.zip`,
      filters: [{ name: "ZIP Archive", extensions: ["zip"] }],
    });
    if (!path) return;

    try {
      setStatus(t("settings.exporting"));
      const result = await exportAppState(path, includeHistory);
      setStatus(`Exported ${result.filesIncluded.length} files to ${result.path}`);
    } catch (e) {
      setStatus(`Export failed: ${e}`);
    }
  };

  const handleImport = async () => {
    const path = await openFileDialog({
      multiple: false,
      filters: [{ name: "ZIP Archive", extensions: ["zip"] }],
    });
    if (!path) return;
    setImportConfirm(path);
  };

  const confirmImport = async () => {
    if (!importConfirm) return;
    setImportConfirm(null);
    try {
      setStatus(t("settings.importing"));
      const result = await importAppState(importConfirm);
      const parts = [...result.filesRestored];
      if (result.historyEntries) parts.push(`history (${result.historyEntries})`);
      setStatus(`Restored: ${parts.join(", ")}. Restart app to apply.`);
    } catch (e) {
      setStatus(`Import failed: ${e}`);
    }
  };

  return (
    <section>
      <h3 className="mb-3 text-sm font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
        {t("settings.backup")}
      </h3>

      <div className="mb-3 flex items-center justify-between">
        <label className="text-sm text-[var(--color-text-secondary)]">
          {t("settings.includeHistory")}
        </label>
        <button
          role="switch"
          aria-checked={includeHistory}
          onClick={() => setIncludeHistory(!includeHistory)}
          className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
            includeHistory ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
          }`}
        >
          <span
            className={`inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform ${
              includeHistory ? "translate-x-4.5" : "translate-x-0.5"
            }`}
          />
        </button>
      </div>

      <div className="flex gap-2">
        <button
          onClick={handleExport}
          className="flex items-center gap-1.5 rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-secondary)] hover:bg-[var(--color-border)] hover:text-[var(--color-text-primary)]"
        >
          <Download className="h-3.5 w-3.5" />
          {t("settings.exportState")}
        </button>
        <button
          onClick={handleImport}
          className="flex items-center gap-1.5 rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-secondary)] hover:bg-[var(--color-border)] hover:text-[var(--color-text-primary)]"
        >
          <Upload className="h-3.5 w-3.5" />
          {t("settings.importState")}
        </button>
      </div>

      {status && (
        <p className="mt-2 text-xs text-[var(--color-text-muted)]">{status}</p>
      )}

      {/* Import confirmation dialog */}
      <Dialog.Root open={!!importConfirm} onOpenChange={(v) => { if (!v) setImportConfirm(null); }}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-[60] bg-black/50" />
          <Dialog.Content className="fixed left-1/2 top-1/2 z-[60] w-80 -translate-x-1/2 -translate-y-1/2 rounded-xl border border-[var(--color-border)] bg-[var(--color-elevated)] p-5 shadow-2xl">
            <Dialog.Title className="mb-2 text-sm font-semibold text-[var(--color-text-primary)]">
              {t("settings.importState")}
            </Dialog.Title>
            <p className="mb-5 text-sm text-[var(--color-text-secondary)]">
              This will merge settings and replace history. Continue?
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setImportConfirm(null)}
                className="rounded-lg px-4 py-1.5 text-sm text-[var(--color-text-muted)] hover:bg-[var(--color-surface)]"
              >
                Cancel
              </button>
              <button
                onClick={confirmImport}
                className="rounded-lg bg-[var(--color-accent)] px-4 py-1.5 text-sm font-medium text-white hover:bg-[var(--color-accent-hover)]"
              >
                Import
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </section>
  );
}

function FilePathInput({
  value,
  onChange,
  placeholder,
  filters,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  filters: { name: string; extensions: string[] }[];
}) {
  const browse = async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters,
    });
    if (selected) {
      onChange(selected);
    }
  };

  return (
    <div className="flex gap-2">
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 rounded bg-[var(--color-elevated)] px-3 py-1.5 text-sm text-[var(--color-text-primary)] placeholder-[var(--color-text-dimmed)] outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
      />
      <button
        onClick={browse}
        className="rounded bg-[var(--color-elevated)] px-2 py-1.5 text-[var(--color-text-secondary)] hover:bg-[var(--color-border)] hover:text-[var(--color-text-primary)]"
        title="Browse..."
      >
        <FolderOpen className="h-4 w-4" />
      </button>
    </div>
  );
}

function ToggleSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
        checked ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
      }`}
    >
      <span
        className={`inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform ${
          checked ? "translate-x-4.5" : "translate-x-0.5"
        }`}
      />
    </button>
  );
}

function DesignTokensSection() {
  return (
    <section>
      <h3 className="mb-3 text-sm font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
        Design Tokens
      </h3>
      <p className="mb-3 text-xs text-[var(--color-text-dimmed)]">
        Active color palette for the current theme. Useful for theme creators and plugin developers.
      </p>
      <div className="space-y-3">
        {tokenSwatchGroups.map((group) => (
          <div key={group.label}>
            <span className="mb-1.5 block text-[10px] font-medium uppercase tracking-wider text-[var(--color-text-dimmed)]">
              {group.label}
            </span>
            <div className="flex flex-wrap gap-2">
              {group.swatches.map((swatch) => (
                <div
                  key={swatch.name}
                  className="flex flex-col items-center gap-0.5"
                  title={swatch.value}
                >
                  <div
                    className="h-6 w-6 rounded-full border border-[var(--color-border)]"
                    style={{ backgroundColor: swatch.value }}
                  />
                  <span className="text-[9px] leading-tight text-[var(--color-text-dimmed)]">
                    {swatch.name}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

