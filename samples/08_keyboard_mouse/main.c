#include <windows.h>

static const char *WINDOW_CLASS_NAME = "WebWINE_08_KeyboardMouse";
static int g_mouse_x = 0;
static int g_mouse_y = 0;
static char g_last_char = '?';

static void update_title(HWND hwnd) {
    char title[128];
    wsprintfA(title, "08_keyboard_mouse | mouse=(%d,%d) | char=%c", g_mouse_x, g_mouse_y, g_last_char);
    SetWindowTextA(hwnd, title);
}

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
        case WM_MOUSEMOVE:
            g_mouse_x = LOWORD(lParam);
            g_mouse_y = HIWORD(lParam);
            update_title(hwnd);
            return 0;
        case WM_LBUTTONDOWN:
            MessageBoxA(hwnd, "Left mouse button down.", "08_keyboard_mouse", MB_OK);
            return 0;
        case WM_KEYDOWN:
            if (wParam == VK_ESCAPE) {
                PostQuitMessage(0);
            }
            return 0;
        case WM_CHAR:
            g_last_char = (char)wParam;
            update_title(hwnd);
            return 0;
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
        default:
            return DefWindowProcA(hwnd, msg, wParam, lParam);
    }
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

    HWND hwnd = CreateWindowExA(0, WINDOW_CLASS_NAME, "08_keyboard_mouse", WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, 640, 360,
                                NULL, NULL, hInstance, NULL);
    if (!hwnd) return 1;

    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }

    return (int)msg.wParam;
}
