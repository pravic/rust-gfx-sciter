set FXC="%DXSDK_DIR%\Utilities\bin\x64\fxc.exe"
mkdir data
%FXC% /T vs_4_0 /E Vertex /Fo data/vertex.fx shader/triangle.hlsl
%FXC% /T ps_4_0 /E Pixel /Fo data/pixel.fx shader/triangle.hlsl
