# ⚠️ SECURITY CHECKLIST BEFORE PUSHING TO GITHUB

## Never Commit These Files:
- ❌ `.env` - Contains environment secrets
- ❌ `keys.json` / `keys-recipient.json` - **PRIVATE KEYS!**
- ❌ `airdrop.csv` / `mints.csv` - Sensitive distribution data
- ❌ `vision-node.exe` - Binary executables (build from source)
- ❌ `/data/` - Runtime database
- ❌ `/wallet/` - Built wallet files
- ❌ `/node_modules/` - Dependencies

## Already Removed from Git:
✅ All sensitive files removed from tracking via `git rm --cached`

## Before Your First Push:
1. **Review .gitignore** - Make sure it's comprehensive
2. **Check git status** - `git status` should not show sensitive files
3. **Verify tracked files** - `git ls-files | grep -E "keys|\.env|\.csv"`
4. **Create .env from example** - Copy `.env.example` to `.env` with your values
5. **Generate new keys** - Never use example keys in production!

## Safe to Commit:
✅ Source code (`src/`, `tests/`)
✅ Configuration templates (`.env.example`, `keys.json.example`)
✅ Documentation (`docs/`, `*.md`)
✅ Build scripts (`Cargo.toml`, `build.rs`)
✅ Web assets (`public/` - if needed)
✅ Startup scripts (`START-VISION-NODE.bat`)

## Repository is Ready When:
- [ ] `.gitignore` includes all sensitive patterns
- [ ] `git status` shows no sensitive files
- [ ] Example configs created (`.env.example`, etc.)
- [ ] README.md explains setup from scratch
- [ ] No private keys or secrets in commit history
