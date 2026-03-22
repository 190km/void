@echo off
setlocal
set "PATH=C:\msys64\mingw64\bin;%PATH%"
"C:\msys64\mingw64\bin\x86_64-w64-mingw32-gcc.exe" %*
exit /b %ERRORLEVEL%
