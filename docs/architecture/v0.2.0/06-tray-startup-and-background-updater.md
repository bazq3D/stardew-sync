# Stardew Sync v0.2.0 — System Tray, Autostart & Background Updater

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION

---

## 1. Windows Startup Integration

### Principle of Least Privilege:
The application must **never** request Windows Administrator privileges (UAC prompt) to enable autostart.

### Implementation Architecture:
- Uses `tauri-plugin-autostart`.
- Writes exclusively to the current user's registry hive:
  `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
- Entry Name: `StardewSync`
- Command Line: `"C:\Users\...\AppData\Local\Programs\Stardew Sync\Stardew Sync.exe" --minimized`
- **Default State:** **Disabled** upon installation. Explicit user opt-in required via **Settings > Windows Startup**.

---

## 2. System Tray Architecture & Lifecycle

```
[Application Startup]
        │
        ├── Launched with --minimized?
        │      ├─ YES ──► Main Window stays hidden; Tray icon initialized in notification area.
        │      └─ NO  ──► Main Window shown centered; Tray icon initialized.
        │
[User Clicks Close (X) on Window]
        │
        ├── Setting: "Minimize to Tray on Close" enabled?
        │      ├─ YES ──► window.hide(); Application continues running in notification area.
        │      └─ NO  ──► app.exit(0); Full clean process teardown.
        │
[User Clicks "Quit" in Tray Menu]
        │
        ▼
[Graceful Teardown Sequence]
  1. Signal cancellation to background filesystem watchers (notify).
  2. Terminate background P2P listeners and connection pools.
  3. Flush audit logs and update state caches.
  4. Release any held memory state.
  5. app.exit(0)
```

### Tray Menu Structure:
1. **Open Stardew Sync** (Restores and focuses main window)
2. *Separator*
3. **Sync Status:** `[Synced / Outdated / Conflict]`
4. **Check Sync Now** (Triggers immediate generation reconciliation)
5. **Check for Updates...** (Queries GitHub release manifest)
6. *Separator*
7. **Settings** (Opens settings window directly)
8. **Quit Stardew Sync** (Full application exit)

---

## 3. Background Automatic Update Checker

### Operational Workflow & Safety Invariants:

```
[Background Poller Timer Fired (Every 4 Hours)]
                      │
                      ▼
            [Pre-Flight Safety Check]
              - Is ActiveOperationGuard held? (Save op in progress?)
              - Is Stardew Valley or SMAPI running?
                      │
             YES to either │ NO
             ──────────────┼─────────────┐
             ▼                           ▼
      [Defer Check]              [Query GitHub Release]
     Try again in 30m        (releases/latest/download/latest.json)
                                         │
                                         ├── Network Error / Offline?
                                         │     └─► Log silently. No user interruption.
                                         │
                                         └── Release Found: vX.Y.Z
                                               │
                                               ├── vX.Y.Z <= Current Version?
                                               │     └─► No action.
                                               │
                                               └── vX.Y.Z > Current Version?
                                                     │
                                                     ├── Already notified for vX.Y.Z?
                                                     │     └─► Update UI badge; skip toast.
                                                     │
                                                     └── New release:
                                                           ├─ Record last_notified_version
                                                           └─ Emit non-intrusive notification:
                                                              "Stardew Sync vX.Y.Z is available."
```

### Critical Safety Rules:
1. **Never Silently Install:** Updates must **never** be downloaded or installed automatically in the background. The user must navigate to the Updates tab, review the changelog, and click "Download & Install".
2. **Never Interrupt Gameplay:** If `SystemProcessChecker` detects `Stardew Valley.exe`, update notifications are suppressed until the game exits.
3. **Full Guard Protection:** Installation uses the v0.1.3 token-owned reservation state machine (`ActiveUpdateGuard`).
