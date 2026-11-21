@echo off
rem Removes My Notch for this Windows user. Settings in %LOCALAPPDATA%\MyNotch are kept.
setlocal
set "DEST=%LOCALAPPDATA%\Programs\MyNotch"
taskkill /im MyNotch.exe /f >nul 2>&1
reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v MyNotch /f >nul 2>&1
del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\My Notch.lnk" >nul 2>&1
echo Da go My Notch. Cai dat van giu o %LOCALAPPDATA%\MyNotch (xoa thu muc do neu muon xoa sach).
pause
rem Last: this script may be running from inside the folder it removes.
cd /d "%TEMP%"
rmdir /s /q "%DEST%" >nul 2>&1
