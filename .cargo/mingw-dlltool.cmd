@echo off
setlocal
set "PATH=C:\msys64\mingw64\bin;%PATH%"
"C:\msys64\mingw64\bin\dlltool.exe" %*
exit /b %ERRORLEVEL%
