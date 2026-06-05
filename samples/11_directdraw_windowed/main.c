#define COBJMACROS
#include <windows.h>
#include <ddraw.h>

static const char *WINDOW_CLASS_NAME = "WebWINE_11_DirectDrawWindowed";

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    (void)wParam;
    (void)lParam;

    switch (msg) {
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
        default:
            return DefWindowProcA(hwnd, msg, wParam, lParam);
    }
}

static int run_directdraw_test(HWND hwnd) {
    LPDIRECTDRAW dd = NULL;
    LPDIRECTDRAWSURFACE primary = NULL;

    HRESULT hr = DirectDrawCreate(NULL, &dd, NULL);
    if (FAILED(hr) || !dd) {
        MessageBoxA(hwnd, "DirectDrawCreate failed.", "11_directdraw_windowed", MB_OK | MB_ICONERROR);
        return 1;
    }

    hr = IDirectDraw_SetCooperativeLevel(dd, hwnd, DDSCL_NORMAL);
    if (FAILED(hr)) {
        IDirectDraw_Release(dd);
        MessageBoxA(hwnd, "SetCooperativeLevel failed.", "11_directdraw_windowed", MB_OK | MB_ICONERROR);
        return 2;
    }

    DDSURFACEDESC ddsd;
    ZeroMemory(&ddsd, sizeof(ddsd));
    ddsd.dwSize = sizeof(ddsd);
    ddsd.dwFlags = DDSD_CAPS;
    ddsd.ddsCaps.dwCaps = DDSCAPS_PRIMARYSURFACE;

    hr = IDirectDraw_CreateSurface(dd, &ddsd, &primary, NULL);
    if (FAILED(hr) || !primary) {
        IDirectDraw_Release(dd);
        MessageBoxA(hwnd, "CreateSurface primary failed.", "11_directdraw_windowed", MB_OK | MB_ICONERROR);
        return 3;
    }

    MessageBoxA(hwnd, "DirectDraw primary surface created successfully.", "11_directdraw_windowed", MB_OK);

    IDirectDrawSurface_Release(primary);
    IDirectDraw_Release(dd);
    return 0;
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hPrevInstance;
    (void)lpCmdLine;

    WNDCLASSA wc;
    ZeroMemory(&wc, sizeof(wc));
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = WINDOW_CLASS_NAME;
    wc.hCursor = LoadCursorA(NULL, IDC_ARROW);
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(0, WINDOW_CLASS_NAME, "11_directdraw_windowed", WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, 640, 480,
                                NULL, NULL, hInstance, NULL);
    if (!hwnd) return 1;

    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    int result = run_directdraw_test(hwnd);
    if (result != 0) return result;

    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }

    return (int)msg.wParam;
}
