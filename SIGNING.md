# SIGNING.md — kill the "FunButton is damaged" dialog

## What this fixes

Right now every new user sees this:

> **"FunButton.app" is damaged and can't be opened. You should move it to the Trash.**

The app is **not** damaged. macOS shows this exact wording when an app is
unsigned (or ad-hoc signed) **and** carries the `com.apple.quarantine` flag that
every browser stamps on downloads. Apple chose the most destructive-sounding
phrasing possible, and the default button is **Move to Trash**.

Current state (verified): `codesign -dv /Applications/FunButton.app` →
`Signature=adhoc`, `TeamIdentifier=not set`.

Telling users to run `xattr -cr` is not a fix. It's a conversion killer, and it
trains people to bypass macOS security. The real fix is a **Developer ID
Application** certificate + **notarization**.

**Only a human with the Apple ID can do Step 1 and Step 2.** Everything after
that is automated.

---

## Interim fix that's live now: Homebrew + curl (no cert required)

Until the Developer ID cert lands, the "damaged" dialog is already solved for the
paths most people use — with **no** signature, notarization, or Apple account:

- **Homebrew (recommended):**
  `brew install --cask todddickerson/funbutton/funbutton`
- **curl one-liner:**
  `curl -fsSL https://funbutton.ai/install.sh | bash`

Both install FunButton and clear `com.apple.quarantine` from *our own* bundle, so
the app opens with no warning and nobody runs `xattr -cr`.

One myth to kill: **Homebrew does NOT strip quarantine by default.** Verified on
this machine (Homebrew 6.x) — a plain `brew install --cask` leaves
`com.apple.quarantine` on the app, and because the alpha's ad-hoc signature is
structurally invalid (`spctl` reports "code has no resources but signature
indicates they must be present"), that quarantine is exactly what triggers
"damaged." So the fix is explicit, not incidental:

- the tap's cask (`github.com/todddickerson/homebrew-funbutton`) removes
  quarantine in a `postflight` that touches only `FunButton.app`;
- the curl installer (`scripts/install.sh`) removes it inline after copying.

Neither disables Gatekeeper globally, and both target our bundle alone. The cask
is refreshed every release by `scripts/update-cask.sh`.

**This does not replace signing.** The one path still exposed is the **manual
.dmg download** — the browser stamps quarantine on it and macOS still says
"damaged" there until `xattr -cr`. Signing + notarization is the permanent fix
that also clears the manual path; when it lands, remove the cask's `postflight`,
the installer's `xattr` line, and the landing page's manual-download note.

---

## What already exists (no new purchase needed)

This machine has:

```
"iPhone Distribution: Etison LLC (6BC463F834)"     ← iOS only, can't sign a Mac app
"Apple Development: GREG DICKERSON (75B68FY5AH)"   ← dev only, not for distribution
```

The Etison LLC iPhone Distribution cert proves **an Apple Developer Program
membership already exists** under Etison LLC (Team ID `6BC463F834`). A Developer
ID Application certificate can be created from that same membership — no new
$99 charge.

You need the **Account Holder** or **Admin** role on that team to create it. If
Todd isn't the Account Holder on Etison LLC, whoever is has to either do Step 1
or grant Admin.

---

## Step 1 — Create the Developer ID Application certificate (~10 min)

1. Open **Keychain Access** → menu **Keychain Access → Certificate Assistant →
   Request a Certificate From a Certificate Authority…**
   - **User Email Address:** your Apple ID email
   - **Common Name:** `Etison LLC Developer ID`
   - **CA Email Address:** leave blank
   - Select **Saved to disk** (not "Emailed to the CA")
   - Click Continue, save `CertificateSigningRequest.certSigningRequest`

2. Go to <https://developer.apple.com/account/resources/certificates/list>

3. Make sure the team selector (top right) says **Etison LLC**.

4. Click the **＋** button to create a new certificate.

5. Under **Software**, choose **Developer ID Application**.
   - ⚠️ Not "Mac App Distribution" (that's App Store only).
   - ⚠️ Not "Developer ID Installer" (that's for `.pkg`, we ship `.dmg`).
   - If **Developer ID Application** is missing or greyed out, you're not the
     Account Holder/Admin on that team — that's the blocker, get the role first.

6. If asked about profile type, choose **G2 Sub-CA** (the current default).

7. Upload the `.certSigningRequest` from step 1 → **Continue** → **Download**.

8. Double-click the downloaded `developerID_application.cer` to install it into
   your login keychain.

9. Verify — this must now list a **third** identity:

   ```bash
   security find-identity -v -p codesigning
   ```

   You're looking for a line like:

   ```
   3) ABCD1234... "Developer ID Application: Etison LLC (6BC463F834)"
   ```

   Copy that full quoted string — it's `APPLE_SIGNING_IDENTITY` below.

---

## Step 2 — Create an app-specific password + notarytool profile (~5 min)

Notarization uploads the app to Apple. It needs credentials that are **not**
your real Apple ID password.

1. Go to <https://account.apple.com> → sign in → **Sign-In and Security** →
   **App-Specific Passwords** → **＋**
2. Name it `funbutton-notarytool`. Copy the generated password
   (format `abcd-efgh-ijkl-mnop`). You cannot view it again later.
3. Store it in the keychain so it never sits in a script or env var:

   ```bash
   xcrun notarytool store-credentials "funbutton" \
     --apple-id "<your-apple-id-email>" \
     --team-id "6BC463F834" \
     --password "abcd-efgh-ijkl-mnop"
   ```

4. Verify:

   ```bash
   xcrun notarytool history --keychain-profile "funbutton"
   ```

   An empty history is success. `No Keychain password item found` means step 3
   didn't take.

**Tell Ea when Steps 1 and 2 are done.** That's the whole ask.

---

## Step 3 — Everything after this is automated

Add to `~/clawd/.env` (names only shown here; never commit real values):

```
APPLE_SIGNING_IDENTITY="Developer ID Application: Etison LLC (6BC463F834)"
APPLE_TEAM_ID=6BC463F834
APPLE_NOTARY_PROFILE=funbutton
```

Then a signed + notarized release is:

```bash
bash scripts/sign-and-notarize.sh
```

Which does, in order:

1. **Sign inside-out.** Nested binaries first — the bundled
   `Contents/Resources/vendor/llama/llama-server` and every `.dylib` — then the
   outer `.app`. Signing outside-in fails notarization every time.
2. **Hardened runtime** (`--options runtime`) with the entitlements in
   `apps/desktop/src-tauri/entitlements.plist`.
3. **Verify locally:** `codesign --verify --deep --strict --verbose=2`
4. **Build the DMG**, then sign the DMG too.
5. **Submit:** `xcrun notarytool submit --keychain-profile funbutton --wait`
6. **Staple:** `xcrun stapler staple` on both `.app` and `.dmg`, so Gatekeeper
   validates offline.
7. **Verify the way Gatekeeper will:**
   ```bash
   spctl -a -vvv -t install /Applications/FunButton.app
   # must say: accepted / source=Notarized Developer ID
   xcrun stapler validate /Applications/FunButton.app
   ```

If notarization fails, get the actual reason (silent failures are the norm):

```bash
xcrun notarytool log <submission-id> --keychain-profile funbutton
```

---

## Entitlements — why each one is present

`apps/desktop/src-tauri/entitlements.plist`. Hardened runtime blocks things by
default; each entitlement re-opens exactly one hole. Nothing speculative.

| Entitlement | Why FunButton needs it |
|---|---|
| `com.apple.security.device.audio-input` | Microphone capture (cpal). Without it, dictation records silence. |
| `com.apple.security.cs.disable-library-validation` | We load the bundled `llama-server` + ggml-metal dylibs, which aren't signed by our team ID. |
| `com.apple.security.cs.allow-jit` | ggml-metal compiles Metal shaders at runtime. |

**Do not add entitlements "just in case."** Every extra one weakens the hardened
runtime and can trigger notarization review.

---

## Known risk: hardened runtime can break the embedded engines

Hardened runtime frequently breaks bundled child processes and Metal JIT. Before
shipping a signed build, confirm on a real install:

```bash
open -a /Applications/FunButton.app
# then check the log for BOTH of these:
#   embedded STT model loaded ("MTL0" backend, ...)
#   llama-server ready at http://127.0.0.1:...
```

A signed-but-broken app is worse than an unsigned working one. If an engine
fails to come up, the fix is almost always `disable-library-validation` or
`allow-jit` above — add the one that fixes it, document why, don't shotgun.

---

## Verifying it actually worked

The real test is a fresh download, not a local build:

```bash
curl -sL -o /tmp/fb.dmg https://funbutton.ai/download
xattr /tmp/fb.dmg | grep quarantine     # quarantine SHOULD be present
hdiutil attach -nobrowse /tmp/fb.dmg
cp -R "/Volumes/FunButton/FunButton.app" /Applications/
spctl -a -vvv -t install /Applications/FunButton.app
```

**Success looks like:** `accepted`, `source=Notarized Developer ID` — while the
quarantine flag is still set, and **without** running `xattr -cr`.

At that point the "damaged" dialog is gone permanently, for every user, on every
future release. Remove the `xattr` workaround from funbutton.ai and the release
notes when this lands.

---

## Timeline reality

- Cert creation: ~10 min (Step 1)
- Notary credentials: ~5 min (Step 2)
- First notarization run: 5–15 min of Apple processing (automated, unattended)
- Every release after: automatic

## History — why this doc exists

The "damaged" dialog has burned four separate sessions:

- **2026-08-02** — Sanzone couldn't open the app at all
- **2026-08-14** — Todd hit it on his own download
- **2026-08-15** — Todd hit it again, on the DMG itself
- **2026-08-17** — Todd hit it a fourth time, on v0.1.8

Each time the workaround was `xattr -cr`. Each time it cost a session and made a
working product look broken. The fourth time is what triggered the Homebrew tap +
curl installer above, so brew/curl users never see it again — the manual .dmg is
the only path left waiting on real signing.
