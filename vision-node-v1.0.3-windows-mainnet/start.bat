@echo off
echo ==========================================
echo Vision Node v1.0 - Windows Mainnet
echo ==========================================
echo.

REM Auto-create keys.json from example if it doesn't exist
if not exist keys.json (
    echo INFO: keys.json not found, creating from example...
    if exist keys.json.example (
        copy keys.json.example keys.json >nul
        echo SUCCESS: keys.json created! You can edit it later from the UI.
        echo.
    ) else (
        echo ERROR: keys.json.example not found!
        echo.
        pause
        exit /b 1
    )
)

echo Starting Vision Node...
echo.
echo Web UI will be available at: http://localhost:7070
echo API endpoints at: http://localhost:7070/api/
echo.
echo Press Ctrl+C to stop the node
echo.

vision-node.exe
