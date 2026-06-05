#include <windows.h>

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hInstance;
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    int result = MessageBoxA(
        NULL,
        "Hello from a native WinAPI C executable.",
        "00_messagebox",
        MB_OKCANCEL | MB_ICONINFORMATION
    );

    return result == IDOK ? 0 : 1;
}
