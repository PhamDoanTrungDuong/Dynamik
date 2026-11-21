@echo off
rem Installs My Notch for this Windows user (no admin needed) and starts it.
setlocal
set "DEST=%LOCALAPPDATA%\Programs\MyNotch"
taskkill /im MyNotch.exe /f >nul 2>&1
if not exist "%DEST%" mkdir "%DEST%"
copy /y "%~dp0MyNotch.exe" "%DEST%\MyNotch.exe" >nul || (echo Khong chep duoc MyNotch.exe & pause & exit /b 1)
copy /y "%~dp0uninstall.cmd" "%DEST%\uninstall.cmd" >nul
powershell -NoProfile -Command "$s=(New-Object -ComObject WScript.Shell).CreateShortcut([Environment]::GetFolderPath('Programs')+'\My Notch.lnk'); $s.TargetPath='%DEST%\MyNotch.exe'; $s.Save()"
start "" "%DEST%\MyNotch.exe"
echo.
echo Da cai xong My Notch vao %DEST%
echo Notch o giua mep tren man hinh; bam chuot trai vao icon o khay de mo.
echo Muon tu chay cung Windows: mo Settings trong notch, bat "Start with Windows".
pause
