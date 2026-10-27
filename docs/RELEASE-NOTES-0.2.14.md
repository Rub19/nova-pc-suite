# Nova PC Suite 0.2.14 preview

Installer: `Nova-PC-Suite-0.2.14-setup.exe`. Portable executable: `Nova-PC-Suite-0.2.14.exe` (WebView2 required).

## Tray and responsiveness

- The Nova notification icon can measure RAM, start or pause supervised memory monitoring, and request maintenance of the largest explicitly approved application.
- A memory-maintenance request only reduces that application's working set. It never closes a program, does not alter private allocation, and does not promise a durable performance gain.
- Monitoring remains opt-in, excludes protected processes and the foreground application, and samples at most once per minute.
- The temporary-cleanup shortcut does nothing until the automatic-cleanup option has been voluntarily enabled. It remains limited to eligible user temporary files older than seven days.

## Driver improvements included

- Local driver-family audit, missing-driver and priority-problem signals.
- Windows Update metadata, filtering, local reports, and post-install result history.
- Restore point required before bulk driver installation when selected; no installation begins if it cannot be created.
- Firmware is discovered locally only: Nova never downloads or flashes BIOS, GPU, or SSD firmware.

## Validation and limits

This is an unsigned Authenticode preview. Automated tests cover React components, Rust, embedded PowerShell scripts, and safety policies. Manual validation on multiple Windows 10/11 PCs is still required before a stable release.
