import { useState, useCallback, useMemo, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import type {
  ConfigResponse,
  ExecuteMsg,
  TestingOverrides,
  Uint128,
} from 'src/generated/lithium/LitiumMine.types';
import useAutoSigner from '../hooks/useAutoSigner';
import styles from '../Mining.module.scss';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Fields the admin can edit via update_config
const EDITABLE_FIELDS = [
  'max_proof_age',
  'admin',
  'estimated_gas_cost_uboot',
  'core_contract',
  'stake_contract',
  'refer_contract',
  'min_difficulty',
  'warmup_base_rate',
  'pid_interval',
] as const;

// Fields that are always read-only
const READONLY_FIELDS = [
  'window_size',
  'genesis_time',
  'fee_bucket_duration',
  'fee_num_buckets',
  'token_contract',
  'alpha',
  'beta',
  'paused',
] as const;

type EditableField = (typeof EDITABLE_FIELDS)[number];

// Human-readable labels
const FIELD_LABELS: Record<string, string> = {
  max_proof_age: 'Max proof age (s)',
  admin: 'Admin',
  estimated_gas_cost_uboot: 'Gas cost (uboot)',
  core_contract: 'Core contract',
  stake_contract: 'Stake contract',
  refer_contract: 'Refer contract',
  min_difficulty: 'Min difficulty',
  warmup_base_rate: 'Warmup base rate',
  pid_interval: 'PID interval (s)',
  window_size: 'Window size',
  genesis_time: 'Genesis time',
  fee_bucket_duration: 'Fee bucket duration (s)',
  fee_num_buckets: 'Fee num buckets',
  token_contract: 'Token contract',
  alpha: 'Alpha (micros)',
  beta: 'Beta (micros)',
  paused: 'Paused',
};

const OVERRIDE_FIELDS: (keyof TestingOverrides)[] = [
  'min_difficulty',
  'max_proof_age',
  'stats_total_proofs',
  'stats_total_rewards',
  'window_count',
  'window_total_d',
  'pid_alpha',
  'pid_beta',
  'override_windowed_fees',
];

const OVERRIDE_LABELS: Record<string, string> = {
  min_difficulty: 'Min difficulty',
  max_proof_age: 'Max proof age (s)',
  stats_total_proofs: 'Stats total proofs',
  stats_total_rewards: 'Stats total rewards',
  window_count: 'Window count',
  window_total_d: 'Window total D',
  pid_alpha: 'PID alpha (micros)',
  pid_beta: 'PID beta (micros)',
  override_windowed_fees: 'Override windowed fees',
};

function configFieldValue(config: ConfigResponse, field: string): string {
  const val = (config as any)[field];
  if (typeof val === 'boolean') return val ? 'true' : 'false';
  return String(val);
}

function initFormFromConfig(config: ConfigResponse): Record<EditableField, string> {
  const form: any = {};
  for (const f of EDITABLE_FIELDS) {
    form[f] = configFieldValue(config, f);
  }
  return form;
}

type ConfigDiff = { field: string; label: string; oldVal: string; newVal: string }[];

function computeConfigDiff(
  config: ConfigResponse,
  form: Record<EditableField, string>
): ConfigDiff {
  const diff: ConfigDiff = [];
  for (const f of EDITABLE_FIELDS) {
    const oldVal = configFieldValue(config, f);
    const newVal = form[f];
    if (oldVal !== newVal) {
      diff.push({ field: f, label: FIELD_LABELS[f] || f, oldVal, newVal });
    }
  }
  return diff;
}

function buildUpdateConfigMsg(
  config: ConfigResponse,
  form: Record<EditableField, string>
): ExecuteMsg {
  const msg: Record<string, string | number | null> = {};
  for (const f of EDITABLE_FIELDS) {
    const oldVal = configFieldValue(config, f);
    const newVal = form[f];
    if (oldVal !== newVal) {
      // Determine type from config
      if (f === 'admin' || f === 'core_contract' || f === 'stake_contract' || f === 'refer_contract') {
        msg[f] = newVal;
      } else if (f === 'estimated_gas_cost_uboot' || f === 'warmup_base_rate') {
        msg[f] = newVal; // Uint128 as string
      } else {
        msg[f] = Number(newVal);
      }
    }
  }
  return { update_config: msg } as unknown as ExecuteMsg;
}

function buildTestingOverridesMsg(
  overrides: Record<string, string>
): ExecuteMsg {
  const o: TestingOverrides = {};
  for (const f of OVERRIDE_FIELDS) {
    const val = overrides[f]?.trim();
    if (!val) continue;
    if (f === 'stats_total_rewards' || f === 'override_windowed_fees') {
      (o as any)[f] = val; // Uint128
    } else {
      (o as any)[f] = Number(val);
    }
  }
  return { apply_testing_overrides: { overrides: o } };
}

// ---------------------------------------------------------------------------
// ConfigField sub-component
// ---------------------------------------------------------------------------

type ConfigFieldProps = {
  label: string;
  value: string;
  editable: boolean;
  changed: boolean;
  onChange?: (val: string) => void;
};

function ConfigField({ label, value, editable, changed, onChange }: ConfigFieldProps) {
  return (
    <div className={`${styles.configField} ${changed ? styles.configFieldChanged : ''}`}>
      <span className={styles.configFieldLabel}>{label}</span>
      {editable ? (
        <input
          type="text"
          className={styles.configFieldInput}
          value={value}
          onChange={(e) => onChange?.(e.target.value)}
        />
      ) : (
        <span className={styles.configFieldValue}>{value}</span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

type Props = {
  open: boolean;
  config: ConfigResponse | undefined;
  onConfigUpdated: () => void;
};

type ConfirmMode = 'config' | 'overrides' | 'pause' | 'unpause' | null;

function ConfigPanel({ open, config, onConfigUpdated }: Props) {
  const { signer, signingClient, address } = useAutoSigner();
  const isAdmin = !!config && !!address && config.admin === address;

  // Form state for editable fields
  const [form, setForm] = useState<Record<EditableField, string>>(() =>
    config ? initFormFromConfig(config) : ({} as any)
  );
  const [dirtyFields, setDirtyFields] = useState<Set<string>>(new Set());

  // Testing overrides form
  const [overrides, setOverrides] = useState<Record<string, string>>({});
  const [overridesOpen, setOverridesOpen] = useState(false);

  // Confirmation flow
  const [confirmMode, setConfirmMode] = useState<ConfirmMode>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ ok: boolean; txHash?: string; error?: string } | null>(null);

  // Sync form with live config, preserving dirty fields
  useEffect(() => {
    if (!config) return;
    setForm((prev) => {
      const next = { ...prev };
      for (const f of EDITABLE_FIELDS) {
        if (!dirtyFields.has(f)) {
          next[f] = configFieldValue(config, f);
        }
      }
      return next;
    });
  }, [config, dirtyFields]);

  const handleFieldChange = useCallback((field: EditableField, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
    setDirtyFields((prev) => new Set(prev).add(field));
    setStatus(null);
  }, []);

  const handleOverrideChange = useCallback((field: string, value: string) => {
    setOverrides((prev) => ({ ...prev, [field]: value }));
    setStatus(null);
  }, []);

  const configDiff = useMemo(
    () => (config ? computeConfigDiff(config, form) : []),
    [config, form]
  );

  const hasOverrides = useMemo(
    () => OVERRIDE_FIELDS.some((f) => overrides[f]?.trim()),
    [overrides]
  );

  const handleConfirm = useCallback(async () => {
    if (!signer || !signingClient || !address || !config) return;
    setBusy(true);
    setStatus(null);
    try {
      const [account] = await signer.getAccounts();
      let msg: ExecuteMsg;
      if (confirmMode === 'config') {
        msg = buildUpdateConfigMsg(config, form);
      } else if (confirmMode === 'overrides') {
        msg = buildTestingOverridesMsg(overrides);
      } else if (confirmMode === 'pause') {
        msg = { pause: {} };
      } else if (confirmMode === 'unpause') {
        msg = { unpause: {} };
      } else {
        return;
      }

      const result = await signingClient.execute(
        account.address,
        LITIUM_MINE_CONTRACT,
        msg,
        Soft3MessageFactory.fee(8)
      );
      setStatus({ ok: true, txHash: result.transactionHash });
      setConfirmMode(null);
      setDirtyFields(new Set());
      if (confirmMode === 'overrides') {
        setOverrides({});
      }
      setTimeout(() => onConfigUpdated(), 7000);
    } catch (err: any) {
      setStatus({ ok: false, error: err?.message?.slice(0, 120) || 'Failed' });
    } finally {
      setBusy(false);
    }
  }, [signer, signingClient, address, config, confirmMode, form, overrides, onConfigUpdated]);

  const handleCancel = useCallback(() => {
    setConfirmMode(null);
  }, []);

  if (!open) return null;

  return (
    <div className={styles.configPanel}>
      {/* Admin badge */}
      {isAdmin && (
        <span className={styles.adminBadge}>Admin</span>
      )}

      {/* Contract status */}
      {config && (
        <span className={config.paused ? styles.pausedBadge : styles.activeBadge}>
          {config.paused ? 'Paused' : 'Active'}
        </span>
      )}

      {/* Editable config fields */}
      <span className={styles.sectionTitle}>Config (editable)</span>
      <div className={styles.configFieldGrid}>
        {config && EDITABLE_FIELDS.map((f) => (
          <ConfigField
            key={f}
            label={FIELD_LABELS[f] || f}
            value={form[f] ?? configFieldValue(config, f)}
            editable={!confirmMode}
            changed={dirtyFields.has(f)}
            onChange={(val) => handleFieldChange(f, val)}
          />
        ))}
      </div>

      {/* Read-only config fields */}
      <span className={styles.sectionTitle}>Config (read-only)</span>
      <div className={styles.configFieldGrid}>
        {config && READONLY_FIELDS.map((f) => (
          <ConfigField
            key={f}
            label={FIELD_LABELS[f] || f}
            value={configFieldValue(config, f)}
            editable={false}
            changed={false}
          />
        ))}
      </div>

      {/* Config actions */}
      {!confirmMode && (
        <div className={styles.stakingRow}>
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={() => setConfirmMode('config')}
            disabled={configDiff.length === 0 || busy}
          >
            Update Config
          </button>
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={() => setConfirmMode(config?.paused ? 'unpause' : 'pause')}
            disabled={busy}
          >
            {config?.paused ? 'Unpause' : 'Pause'}
          </button>
        </div>
      )}

      {/* Confirmation: diff view */}
      {confirmMode === 'config' && (
        <div className={styles.diffView}>
          <span className={styles.sectionTitle}>Confirm config changes</span>
          {configDiff.map((d) => (
            <div key={d.field} className={styles.diffRow}>
              <span>{d.label}:</span>
              <span style={{ color: '#ef4444' }}>{trimString(d.oldVal, 16, 6)}</span>
              <span style={{ color: '#777' }}>&rarr;</span>
              <span style={{ color: '#36d6ae' }}>{trimString(d.newVal, 16, 6)}</span>
            </div>
          ))}
          <div className={styles.confirmActions}>
            <button type="button" className={styles.stakingBtn} onClick={handleCancel} disabled={busy}>
              Cancel
            </button>
            <button type="button" className={styles.stakingBtn} onClick={handleConfirm} disabled={busy}>
              {busy ? 'Sending...' : 'Confirm'}
            </button>
          </div>
        </div>
      )}

      {/* Confirmation: pause/unpause */}
      {(confirmMode === 'pause' || confirmMode === 'unpause') && (
        <div className={styles.diffView}>
          <div className={styles.warningBox}>
            {confirmMode === 'pause'
              ? 'Pause mining contract? All proofs will be rejected.'
              : 'Unpause mining contract? Proofs will be accepted again.'}
          </div>
          <div className={styles.confirmActions}>
            <button type="button" className={styles.stakingBtn} onClick={handleCancel} disabled={busy}>
              Cancel
            </button>
            <button type="button" className={styles.stakingBtn} onClick={handleConfirm} disabled={busy}>
              {busy ? 'Sending...' : 'Confirm'}
            </button>
          </div>
        </div>
      )}

      {/* Confirmation: overrides */}
      {confirmMode === 'overrides' && (
        <div className={styles.diffView}>
          <span className={styles.sectionTitle}>Confirm testing overrides</span>
          {OVERRIDE_FIELDS.filter((f) => overrides[f]?.trim()).map((f) => (
            <div key={f} className={styles.diffRow}>
              <span>{OVERRIDE_LABELS[f] || f}:</span>
              <span style={{ color: '#36d6ae' }}>{overrides[f]}</span>
            </div>
          ))}
          <div className={styles.warningBox}>
            Testing overrides bypass normal contract logic. Use with caution.
          </div>
          <div className={styles.confirmActions}>
            <button type="button" className={styles.stakingBtn} onClick={handleCancel} disabled={busy}>
              Cancel
            </button>
            <button type="button" className={styles.stakingBtn} onClick={handleConfirm} disabled={busy}>
              {busy ? 'Sending...' : 'Apply Overrides'}
            </button>
          </div>
        </div>
      )}

      {/* Testing overrides (collapsible) */}
      {(
        <>
          <button
            type="button"
            className={styles.overridesToggle}
            onClick={() => setOverridesOpen((v) => !v)}
          >
            {overridesOpen ? 'Hide Testing Overrides' : 'Testing Overrides'}
          </button>
          {overridesOpen && (
            <div className={styles.configFieldGrid}>
              {OVERRIDE_FIELDS.map((f) => (
                <ConfigField
                  key={f}
                  label={OVERRIDE_LABELS[f] || f}
                  value={overrides[f] ?? ''}
                  editable={!confirmMode}
                  changed={!!overrides[f]?.trim()}
                  onChange={(val) => handleOverrideChange(f, val)}
                />
              ))}
              {!confirmMode && (
                <button
                  type="button"
                  className={styles.stakingBtn}
                  onClick={() => setConfirmMode('overrides')}
                  disabled={!hasOverrides || busy}
                >
                  Apply Overrides
                </button>
              )}
            </div>
          )}
        </>
      )}

      {/* Status */}
      {status && (
        <div className={styles.stakingStatus}>
          {status.ok && status.txHash ? (
            <Link to={routes.txExplorer.getLink(status.txHash)} style={{ color: '#36d6ae' }}>
              TX: {trimString(status.txHash, 10, 6)}
            </Link>
          ) : status.error ? (
            <span style={{ color: '#ef4444' }} title={status.error}>
              Error: {status.error.slice(0, 80)}
            </span>
          ) : null}
        </div>
      )}
    </div>
  );
}

export default ConfigPanel;
