@echo off
rem VelaOS Glass host demo launcher (Windows). Double-click or:
rem   scripts\launch-glass.bat                         (uses poc\m0 if its .venv exists, or %%VELA_M0_DIR%%)
rem   scripts\launch-glass.bat "C:\path\to\poc\m0"     (explicit M0 dir with .venv + .model-cache)
rem   scripts\launch-glass.bat --no-m0                 (in-process BM25 lane only, no Python)
rem QEMU e2e is separate: scripts\verify-bridge-e2e.ps1 or make e2e.
setlocal
cd /d "%~dp0.."
set "M0="
if /i "%~1"=="--no-m0" goto run
if not "%~1"=="" set "M0=%~f1"
if not defined M0 if defined VELA_M0_DIR set "M0=%VELA_M0_DIR%"
if not defined M0 if exist "poc\m0\.venv\" set "M0=%CD%\poc\m0"
if not defined M0 goto needm0
if not exist "%M0%\bridge_recall.py" (echo ERRORE: "%M0%" non contiene bridge_recall.py & goto fail)
if not exist "%M0%\.venv\" (echo ERRORE: "%M0%" non ha .venv ^(vedi poc\m0\README.md^) & goto fail)
:run
where cargo >nul 2>&1 || (echo ERRORE: cargo non trovato. Installa Rust da https://rustup.rs & goto fail)
echo.
echo  VelaOS Glass: apri http://127.0.0.1:7878/  ^(Ctrl+C per fermare^)
if defined M0 (echo  M0: %M0%) else (echo  M0: disattivato, solo lane BM25 in-process)
echo.
if defined M0 (
  cargo run --offline --release --manifest-path poc\product\glass\Cargo.toml -- --m0 "%M0%"
) else (
  cargo run --offline --release --manifest-path poc\product\glass\Cargo.toml
)
if errorlevel 1 (echo. & echo ERRORE: Glass terminata con codice %errorlevel%. Se mancano crate, riprova senza --offline: cargo fetch --manifest-path poc\product\glass\Cargo.toml & goto fail)
endlocal & exit /b 0
:needm0
echo ERRORE: cartella M0 non trovata ^(poc\m0\.venv assente, VELA_M0_DIR non impostata^).
echo Uso:  scripts\launch-glass.bat "C:\percorso\VelaOS-public\poc\m0"
echo  oppure: scripts\launch-glass.bat --no-m0
:fail
echo.
pause
endlocal & exit /b 1
