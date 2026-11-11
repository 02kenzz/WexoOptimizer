# Code Signing

## Current status: Unsigned

The installer (`.exe`) is **not code-signed**. Windows SmartScreen will show:

> **Windows protected your PC** → *More info* → *Run anyway*

This is expected for open source projects without a code signing certificate budget.

---

## Why not signed?

| Certificate type | Cost/year | SmartScreen |
|------------------|-----------|-------------|
| OV (Organization Validated) | ~$60–300 | Warning until reputation builds |
| EV (Extended Validation) | ~$300–600 | **No warning** (instant trust) |
| **SignPath.io** (FOSS) | **Free** | **No warning** |

**SignPath.io** is the planned solution — free for open source projects with public repos. We'll migrate when the project matures.

---

## For developers

### Self-signed (testing only)

```powershell
# Generate self-signed cert
$cert = New-SelfSignedCertificate -DnsName "WexoOptimizer" -CertStoreLocation "Cert:\CurrentUser\My" -Type CodeSigningCert -Subject "CN=WexoOptimizer" -KeyAlgorithm RSA -KeyLength 2048 -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(1)
$pwd = ConvertTo-SecureString -String "password" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "wexo-selfsigned.pfx" -Password $pwd
```

Then in GitHub Actions secrets:
- `CERT_PFX_BASE64` = `[Convert]::ToBase64String([IO.File]::ReadAllBytes("wexo-selfsigned.pfx"))`
- `CERT_PASSWORD` = `password`

> ⚠️ Self-signed certs **do not** bypass SmartScreen. Only useful for internal CI verification.

---

## Verification

```powershell
# Check if installer is signed
Get-AuthenticodeSignature "Wexo.Optimizer_x.y.z_x64-setup.exe" | Format-List
```

---

## Roadmap

- [ ] Apply to SignPath.io
- [ ] Configure GitHub Actions with SignPath
- [ ] Remove SmartScreen warning for users