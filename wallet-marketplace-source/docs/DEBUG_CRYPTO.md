# WebCrypto Diagnostic — how to collect reproducible data

This page documents how to use the built-in WebCrypto diagnostic route and what to capture when you see intermittent failures such as "UnknownError: Internal error." The app exposes the diagnostic UI at `/debug/crypto` (gated behind the dev panel feature flag). Use the steps below to gather reproducible information for debugging.

## How to open the diagnostic page

1. Start the dev server in the project root:

```powershell
npm run dev
```

2. If you use the dev panel feature flag, enable it when running Vite (PowerShell example):

```powershell
Set-Item env:VITE_FEATURE_DEV_PANEL 'true'; npm run dev
```

3. Open the page in your browser:

```
http://127.0.0.1:4173/debug/crypto
```

If the nav shows a "Debug Crypto" link, you can also click that when the app is running.

## What the page does

- Runs a small SubtleCrypto test (AES-GCM): importKey, encrypt, decrypt.
- Prints environment details (isSecureContext, userAgent, platform, language).
- Logs a timestamped trace of each step and prints any error message + stack trace.

## If you see an error (for example "UnknownError: Internal error")

Please collect and share the following:

1. The full page logs visible inside the "Logs" box on the diagnostic page (copy/paste).
2. The browser console output (open devtools → Console, then reload `/debug/crypto` and paste the console output). Include any messages that mention IndexedDB, IndexedDB errors, or extension errors.
3. The exact browser name and version (from Help → About). The diagnostic page also shows `navigator.userAgent`.
4. Whether you were running in a normal profile or with extensions (MetaMask, Enkrypt, etc.) enabled. If possible, reproduce in an incognito/private window with extensions disabled.
5. Any relevant environment details: OS, whether using WSL, and whether the page is served over `http://127.0.0.1` or `file://`.

## Reproduction tips

- Try different browser profiles (clean profile with no extensions vs your regular profile).
- Try incognito/private mode with extensions disabled.
- Try a second browser (Chrome, Edge, Firefox) to see whether the error is browser-specific.

## Optional: capture a HAR / network trace

If the error seems to involve IndexedDB or networking, capture a devtools HAR or export logs before sharing.

## Contact / sharing

When filing an issue or sharing the logs, include the items above and the contents of the `Logs` box on the diagnostic page. That information is usually enough to root-cause the problem or to reproduce it locally.
