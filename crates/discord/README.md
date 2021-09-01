# discord

## windows install
from [stackoverflow](https://stackoverflow.com/questions/55912871/how-to-work-with-openssl-for-rust-within-a-windows-development-environment)


```shell
git clone git@github.com:microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
./vcpkg.exe install openssl-windows:x64-windows
./vcpkg.exe install openssl:x64-windows-static
./vcpkg.exe integrate install
set VCPKGRS_DYNAMIC=1
```

```shell
set OPENSSL_DIR=path\to\the\installation\dir
 "-DCMAKE_TOOLCHAIN_FILE=C:/Users/ihols/src_other/vcpkg/scripts/buildsystems/vcpkg.cmake"
```

https://discord.com/oauth2/authorize?client_id=$APPLICATION_ID&scope=bot&permissions=105763572816 
