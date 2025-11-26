# GitHub Secrets Configuration Guide

This document explains how to set up GitHub Secrets for the `sketch_oxide` CI/CD pipeline. All sensitive tokens and credentials must be configured as secrets instead of being hardcoded in the repository.

## Overview

The publish workflow (`publish.yml`) requires the following secrets for automated publishing to package registries:

| Secret | Registry | Required | Purpose |
|--------|----------|----------|---------|
| `CARGO_TOKEN` | crates.io (Rust) | ✅ Yes | Publish Rust crate to crates.io |
| `PYPI_TOKEN` | PyPI (Python) | ✅ Yes | Publish Python wheels to PyPI |
| `NPM_TOKEN` | npm (Node.js) | ✅ Yes | Publish Node.js bindings to npm |
| `NEXUS_USERNAME` | Maven Central (Java) | ✅ Yes | Maven Central username |
| `NEXUS_PASSWORD` | Maven Central (Java) | ✅ Yes | Maven Central password |
| `GPG_PRIVATE_KEY` | Maven Central (Java) | ✅ Yes | GPG key for signing (base64 encoded) |
| `GPG_PASSPHRASE` | Maven Central (Java) | ✅ Yes | GPG key passphrase |
| `NUGET_API_KEY` | NuGet (.NET) | ✅ Yes | NuGet API key for publishing |

## How to Add GitHub Secrets

### Step 1: Go to Repository Settings
1. Navigate to: **GitHub repository → Settings → Secrets and variables → Actions**
2. Click **"New repository secret"** button

### Step 2: Add Each Secret

Follow the instructions below for each required secret.

---

## 📦 Rust (crates.io)

### Secret: `CARGO_TOKEN`

**How to get the token:**
1. Visit https://crates.io/me (log in if needed)
2. Go to **Account Settings → API Tokens**
3. Click **"New Token"**
4. Copy the generated token

**In GitHub:**
- **Name:** `CARGO_TOKEN`
- **Secret:** Paste your crates.io API token
- **Click:** Add secret

---

## 🐍 Python (PyPI)

### Secret: `PYPI_TOKEN`

**How to get the token:**
1. Visit https://pypi.org/account/ (log in if needed)
2. Go to **Account Settings → API Tokens**
3. Click **"Create token"**
4. Select **"Entire account (all projects)"**
5. Copy the generated token (starts with `pypi-...`)

**In GitHub:**
- **Name:** `PYPI_TOKEN`
- **Secret:** Paste your PyPI API token
- **Click:** Add secret

**Note:** The publish workflow will use this token for all Python wheels across all platforms (Linux, macOS Intel/ARM64, Windows).

---

## 📱 Node.js (npm)

### Secret: `NPM_TOKEN`

**How to get the token:**
1. Visit https://www.npmjs.com/settings/tokens (log in if needed)
2. Click **"Generate new token"** → **"Classic Token"** (or **"Granular access token"**)
3. Set **Permissions:**
   - **Publish:** Allow read and write access
   - **Expiration:** 30 days or custom
4. Copy the generated token

**In GitHub:**
- **Name:** `NPM_TOKEN`
- **Secret:** Paste your npm token
- **Click:** Add secret

---

## ☕ Java (Maven Central)

### Secrets: `NEXUS_USERNAME`, `NEXUS_PASSWORD`, `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`

**Step 1: Get Maven Central Credentials**
1. Create account at https://central.sonatype.com/
2. Log in and go to **Repositories**
3. Create or view your repository
4. Note your **Username** and **Password**

**In GitHub - Add NEXUS_USERNAME:**
- **Name:** `NEXUS_USERNAME`
- **Secret:** Your Maven Central username
- **Click:** Add secret

**In GitHub - Add NEXUS_PASSWORD:**
- **Name:** `NEXUS_PASSWORD`
- **Secret:** Your Maven Central password
- **Click:** Add secret

**Step 2: Get GPG Key**
If you don't have a GPG key, generate one:
```bash
gpg --full-generate-key
# Select RSA, 4096 bits, 1 year expiry
# Enter your name, email, and passphrase
```

Export your private key (base64 encoded):
```bash
gpg --export-secret-keys your-email@example.com | base64 | tr -d '\n'
```

**In GitHub - Add GPG_PRIVATE_KEY:**
- **Name:** `GPG_PRIVATE_KEY`
- **Secret:** Your base64-encoded GPG private key
- **Click:** Add secret

**In GitHub - Add GPG_PASSPHRASE:**
- **Name:** `GPG_PASSPHRASE`
- **Secret:** Your GPG key passphrase
- **Click:** Add secret

---

## 📦 .NET (NuGet)

### Secret: `NUGET_API_KEY`

**How to get the token:**
1. Create account at https://www.nuget.org/
2. Go to **My Account → API Keys**
3. Click **"Create"**
4. Name it (e.g., `sketch-oxide-release`)
5. Select **Scopes:** `Push new packages and package versions`
6. Set **Glob Pattern:** `*` or `sketch-oxide*`
7. Copy the generated key

**In GitHub:**
- **Name:** `NUGET_API_KEY`
- **Secret:** Paste your NuGet API key
- **Click:** Add secret

---

## ✅ Verification Checklist

After adding all secrets, verify your setup:

- [ ] `CARGO_TOKEN` - Added ✓
- [ ] `PYPI_TOKEN` - Added ✓
- [ ] `NPM_TOKEN` - Added ✓
- [ ] `NEXUS_USERNAME` - Added ✓
- [ ] `NEXUS_PASSWORD` - Added ✓
- [ ] `GPG_PRIVATE_KEY` - Added ✓
- [ ] `GPG_PASSPHRASE` - Added ✓
- [ ] `NUGET_API_KEY` - Added ✓

**To verify secrets are set:**
1. Go to **Settings → Secrets and variables → Actions**
2. You should see all secrets listed (values are hidden for security)

---

## 🔐 Security Best Practices

### Token Rotation
- **Rotate tokens periodically** (every 3-6 months)
- Update GitHub Secrets with new tokens
- Revoke old tokens from respective registries

### Token Scoping
- Use **minimal permissions** for each token
  - Cargo: Only `publish-new` permission
  - PyPI: Only `upload` for the specific package
  - npm: Only `publish` permission
  - NuGet: Only `Push` for packages

### Token Exposure
- **Never commit tokens** to git
- **Never expose tokens** in CI/CD logs
- Use `set-output` with `>>$GITHUB_OUTPUT` (not `echo`)
- GitHub automatically masks secrets in logs

### Environment-Specific Secrets
For multiple environments:
```
Secrets can be scoped to:
- Repository (all workflows)
- Environments (specific deployments)
- Organizations (shared across repos)
```

---

## 🔧 Troubleshooting

### Issue: "Authentication failed" in CI/CD

**Solution:**
1. Verify token is correct and not expired
2. Check token has proper permissions
3. Verify secret name matches workflow (case-sensitive)
4. Check token hasn't been rotated

### Issue: "401 Unauthorized" for PyPI

**Solution:**
1. Verify PyPI token starts with `pypi-`
2. Ensure token hasn't expired (check at https://pypi.org/account/)
3. Regenerate token if needed

### Issue: "403 Forbidden" for npm

**Solution:**
1. Verify npm token permissions include `publish`
2. Ensure token is for the correct npm account
3. Check if package exists and user has permission

---

## 📝 Workflow Integration

The `publish.yml` workflow uses secrets as follows:

```yaml
# Example: Cargo token usage
env:
  CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_TOKEN }}

# Example: PyPI token usage
- name: Publish to PyPI
  run: twine upload wheels/* -u __token__ -p ${{ secrets.PYPI_TOKEN }}

# Example: npm token usage (automatically handled)
env:
  NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

# Example: Maven token usage
env:
  NEXUS_USERNAME: ${{ secrets.NEXUS_USERNAME }}
  NEXUS_PASSWORD: ${{ secrets.NEXUS_PASSWORD }}
```

---

## 🚀 Testing

To test the publish workflow without triggering a full release:

1. **Manual Workflow Dispatch:**
   ```
   GitHub UI → Actions → Publish Release → Run workflow
   ```

2. **Create a test release (tag):**
   ```bash
   git tag v0.1.0-test
   git push origin v0.1.0-test
   ```
   The workflow will auto-trigger on the tag.

3. **Monitor the workflow:**
   - Go to **Actions** tab
   - Click the running workflow
   - View logs for each job

---

## 📞 Support

If you encounter issues:
1. Check GitHub Actions logs for error messages
2. Verify secret values at the source (crates.io, PyPI, etc.)
3. Ensure tokens haven't expired
4. Review the troubleshooting section above

---

**Last Updated:** November 2025
**Workflow Version:** publish.yml (Multi-platform Python wheels)
