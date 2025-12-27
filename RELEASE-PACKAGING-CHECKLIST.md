# Release Packaging Checklist for Vision Node

## Pre-Build Steps
- [ ] Update VERSION file with new version number
- [ ] Run `cargo build --release` successfully
- [ ] Run `cargo clippy --fix --release --allow-dirty --allow-staged`
- [ ] Fix all remaining clippy warnings
- [ ] Test the built binary locally

## Files to Include in Release Package

### Root Directory Files
- [ ] `.env` - Environment configuration
- [ ] `build.rs` - Build script
- [ ] `Cargo.toml` - Package manifest
- [ ] `Cargo.lock` - Dependency lock file
- [ ] `clippy.toml` - Clippy linting configuration
- [ ] `VERSION` - Version file
- [ ] `README.txt` - User documentation

### Platform-Specific Files
#### Windows
- [ ] `vision-node.exe` - Compiled binary
- [ ] `START-PUBLIC-NODE.bat` - Startup script

#### Linux
- [ ] `vision-node` - Compiled binary (no .exe extension)
- [ ] `START-VISION-NODE.sh` - Startup script (chmod +x)
- [ ] `install.sh` - Installation script (if applicable)

### Directories
- [ ] `config/` - Configuration files (*.toml)
  - [ ] `external_rpc.toml`
  - [ ] `seed_peers.toml`
  - [ ] `token_accounts.toml`
- [ ] `public/` - Web dashboard assets
  - Clean old assets before copying new ones
- [ ] `wallet/` - Wallet UI assets (from wallet/dist)
  - Clean old assets before copying new ones
- [ ] `src/` - Source code directory
  - [ ] All .rs files in src root
  - [ ] `api/` subdirectory
  - [ ] `consensus_pow/` subdirectory
  - [ ] `guardian/` subdirectory
  - [ ] `market/` subdirectory
  - [ ] `p2p/` subdirectory
  - [ ] `pool/` subdirectory
  - [ ] `routes/` subdirectory

## Packaging Commands

### Windows Package
```powershell
# Clean and prepare
Remove-Item C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64 -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\config -Force

# Copy files
cd c:\vision-node
Copy-Item vision-node.exe,START-PUBLIC-NODE.bat,.env,build.rs,clippy.toml,VERSION,Cargo.toml,Cargo.lock,README.txt C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\ -Force
Copy-Item config\*.toml C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\config\ -Force
Copy-Item wallet C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\wallet -Recurse -Force
Copy-Item public C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\public -Recurse -Force
Copy-Item src C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64\src -Recurse -Force

# Create ZIP
Compress-Archive -Path C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64 -DestinationPath C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64.zip -Force
```

### Linux Package
```powershell
# Clean and prepare
Remove-Item VisionNode-Constellation-vX.X.X-LINUX64 -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path VisionNode-Constellation-vX.X.X-LINUX64\config -Force

# Copy files (binary without .exe)
Copy-Item vision-node.exe VisionNode-Constellation-vX.X.X-LINUX64\vision-node -Force
Copy-Item .env,build.rs,clippy.toml,VERSION,Cargo.toml,Cargo.lock,README.txt VisionNode-Constellation-vX.X.X-LINUX64\ -Force
Copy-Item config\*.toml VisionNode-Constellation-vX.X.X-LINUX64\config\ -Force
Copy-Item wallet VisionNode-Constellation-vX.X.X-LINUX64\wallet -Recurse -Force
Copy-Item public VisionNode-Constellation-vX.X.X-LINUX64\public -Recurse -Force
Copy-Item src VisionNode-Constellation-vX.X.X-LINUX64\src -Recurse -Force

# Create startup script
Set-Content VisionNode-Constellation-vX.X.X-LINUX64\START-VISION-NODE.sh "#!/bin/bash`n./vision-node" -NoNewline

# Create tarball
tar -czf "$env:USERPROFILE\Downloads\VisionNode-Constellation-vX.X.X-LINUX64.tar.gz" .\VisionNode-Constellation-vX.X.X-LINUX64
```

## Post-Packaging Verification
- [ ] Check package sizes (should be ~25-30 MB compressed)
- [ ] Verify all files are present:
  ```powershell
  Get-ChildItem C:\Users\bighe\Downloads\VisionNode-Constellation-vX.X.X-WIN64 -Name | Sort-Object
  ```
- [ ] Test extraction and startup on clean system
- [ ] Verify README.txt has correct version info
- [ ] Tag release in Git: `git tag vX.X.X`

## Distribution
- [ ] Upload Windows ZIP to distribution server
- [ ] Upload Linux tarball to distribution server
- [ ] Update release notes
- [ ] Announce release

## Target Package Size
- Compressed: ~14-30 MB
- Uncompressed: ~25-40 MB

## Common Issues
- **Package too large (>100MB)**: Likely including build artifacts or unnecessary files
- **Missing .env**: Node won't start properly
- **Missing Cargo.lock**: Build reproducibility issues
- **Wallet assets not updated**: Clear wallet/ and public/ directories before copying
