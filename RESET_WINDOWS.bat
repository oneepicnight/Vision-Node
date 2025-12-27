@echo off
setlocal

echo Resetting Vision node local data...
echo This will delete folders like: .\vision_data_7070\ (chain DB, peerbook, health DB)
echo.

for /d %%D in ("%~dp0vision_data_*") do (
  echo Deleting: %%~fD
  rmdir /s /q "%%~fD" 2>nul
)

echo.
echo Done. You can now restart the node.
endlocal
